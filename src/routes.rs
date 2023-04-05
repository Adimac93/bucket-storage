use axum::extract::{FromRef, FromRequestParts, Multipart, Path, State};
use axum::{async_trait, debug_handler, Json, Router, TypedHeader};
use axum::body::{Body, StreamBody};
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::http::request::Parts;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use reqwest::{header, StatusCode};
use serde_json::json;
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
use crate::auth::{ArgonHash, Claims, Credentials, get_auth_parts, verify_credentials};
use crate::errors::AppError;

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/key", get(issue_key))
        .route("/download/:file_id", get(download))
        .route("/upload", post(upload))
}

struct KeyIssue {
    bucket_name: String
}

struct KeyParts {
    key_id: Uuid,
    key: String,

}

async fn issue_key(State(pool): State<PgPool>) -> Result<impl IntoResponse, AppError> {
    let bucket_name = "bucket";
    let bucket_id = query!(r#"
    INSERT INTO buckets (name)
    VALUES ($1)
    RETURNING id
    "#, bucket_name).fetch_one(&pool).await?.id;

    let key = Uuid::new_v4().to_string();
    let key_id = query!(r#"
    INSERT INTO bucket_keys (key, bucket_id)
    VALUES ($1, $2)
    RETURNING id
    "#, ArgonHash::hash(&key)?, bucket_id).fetch_one(&pool).await?.id;

    Ok(Json(json!({"keyId": key_id, "key": key})))
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
    let mut headers = HeaderMap::new();
    headers.append(CONTENT_TYPE, "Image/png".parse().unwrap());
    Ok((headers, body))
}


#[debug_handler]
async fn upload(claims: Claims, State(pool): State<PgPool>, mut multipart: Multipart) -> Result<Json<Vec<Uuid>>, AppError> {
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
            continue
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
    debug!("{file_ids:#?}");
    Ok(Json(file_ids))
}
