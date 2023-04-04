use axum::extract::{FromRef, FromRequestParts, Multipart, Path, State};
use axum::{async_trait, debug_handler, Json, Router, TypedHeader};
use axum::body::{Body, StreamBody};
use axum::http::request::Parts;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use reqwest::{header, StatusCode};
use sha1::{Sha1, Digest};
use sha1::digest::FixedOutput;
use sqlx::{PgPool, query};
use tokio::fs;
use tokio::fs::{File, write};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::io::{ReaderStream, StreamReader};
use tracing::debug;
use uuid::Uuid;
use crate::AppState;
use crate::auth::{Credentials, get_auth_parts, verify_credentials};
use crate::errors::AppError;

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/download/:file_id", get(download))
        .route("/upload", post(upload))
}

#[debug_handler]
async fn download(claims: Claims, State(pool): State<PgPool>, Path(file_id): Path<Uuid>) -> Result<impl IntoResponse, AppError> {
    let res = query!(r#"
    SELECT files.name, extension
    FROM buckets
    JOIN files ON files.bucket_id = buckets.id
    WHERE buckets.id = $1 AND files.id = $2
    "#, claims.bucket_id, file_id).fetch_optional(&pool).await?.ok_or(AppError::Expected {code: StatusCode::NO_CONTENT, message: "File not found"})?;

    let file = File::open(format!("./store/{}/{file_id}.png", claims.bucket_id)).await.unwrap();
    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    Ok(body)
}


#[debug_handler]
async fn upload(claims: Claims, State(pool): State<PgPool>, mut multipart: Multipart) -> Result<impl IntoResponse, AppError> {
    let mut file_ids = Vec::new();
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = if let Some(filename) = field.file_name() {
            filename.to_string()
        } else {
            continue;
        };

        let bytes = field.bytes().await.unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bytes.clone());
        let hash = hasher.finalize();
        let checksum = format!("{hash:x}");
        let extension = "png";

        let file = query!(r#"
        SELECT *
        FROM files
        WHERE checksum = $1
        "#, checksum).fetch_optional(&pool).await?;

        if let Some(file) = file {
            debug!("Matching file checksum");
            file_ids.push(file.id);
            return Ok(());
        }

        let file_id = query!(r#"
        INSERT INTO files (name, extension, checksum, bucket_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#, name, extension, checksum, claims.bucket_id).fetch_optional(&pool).await?.ok_or(AppError::Expected {code: StatusCode::NO_CONTENT, message: "File not found"})?.id;
        fs::create_dir(format!("./store/{}", claims.bucket_id)).await.ok();
        write(format!("./store/{}/{file_id}.{extension}", claims.bucket_id), bytes).await.unwrap();

        file_ids.push(file_id);
    }
    println!("{file_ids:#?}");
    Ok(())
}

struct Claims {
    key_id: Uuid,
    bucket_id: Uuid
}

#[async_trait]
impl <S>FromRequestParts<S> for Claims
    where S: Send + Sync, PgPool: FromRef<S>
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);
        let credentials = get_auth_parts(parts).await?;
        let bucket_id = verify_credentials(&pool, &credentials).await?;

        Ok(Self { key_id: credentials.key_id, bucket_id})
    }
}