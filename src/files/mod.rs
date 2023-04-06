use std::path::PathBuf;
use anyhow::anyhow;
use axum::extract::{FromRef, FromRequestParts, Multipart, Path, State};
use axum::{async_trait, debug_handler, Json, Router, TypedHeader};
use axum::body::{Body, Bytes, StreamBody};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use sha1::{Sha1, Digest};
use sqlx::{PgPool, query};
use tokio::{fs, io};
use tokio::fs::{File, write};
use tokio_util::io::ReaderStream;
use tracing::debug;
use tracing_test::traced_test;
use uuid::Uuid;
use crate::AppState;
use crate::auth::{ArgonHash, Claims, Credentials, get_auth_parts, verify_credentials};
use crate::errors::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/download/:file_id", get(download))
        .route("/upload", post(upload))
        .route("/delete/:file_id", get(delete))
}


struct KeyIssue {
    bucket_name: String
}

struct KeyParts {
    key_id: Uuid,
    key: String,

}

#[debug_handler]
async fn download(claims: Claims, State(pool): State<PgPool>, Path(file_id): Path<Uuid>) -> Result<impl IntoResponse, AppError> {
    let res = query!(r#"
    SELECT bucket_files.name, extension
    FROM buckets
    JOIN bucket_files ON bucket_files.bucket_id = buckets.id
    JOIN files ON files.id = bucket_files.file_id
    WHERE buckets.id = $1 AND files.id = $2
    "#, claims.bucket_id, file_id).fetch_optional(&pool).await?.ok_or(AppError::Expected {code: StatusCode::NO_CONTENT, message: "File not found"})?;

    let file = Store::new().read(&StoreFile::new(file_id, res.extension.clone())).await.unwrap();
    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);
    let mut headers = HeaderMap::new();
    if let Some(ext) = res.extension {
        if &ext == "png" || &ext == "jpg" {
            headers.append(CONTENT_TYPE, format!("image/{ext}").parse().unwrap());
        }
    }

    Ok((headers, body))
}


#[debug_handler]
async fn upload(claims: Claims, State(pool): State<PgPool>, mut multipart: Multipart) -> Result<Json<Vec<Uuid>>, AppError> {
    let mut file_ids = Vec::new();
    while let Some(field) = multipart.next_field().await.unwrap() {
        let (name, extension) = if let Some(file_name) = field.file_name() {
            let file_name_parts = file_name.rsplit_once('.');
            match file_name_parts {
                None => (file_name.to_string(), None),
                Some((name,extension)) => (name.to_string(), Some(extension.to_string()))
            }
        } else {
            continue;
        };

        debug!("file name: {name}, extension: {extension:?}");

        let bytes = field.bytes().await.unwrap();
        let mut hasher = Sha1::new();
        hasher.update(bytes.clone());
        let hash = hasher.finalize();
        let checksum = format!("{hash:x}");

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
        INSERT INTO files (extension, checksum)
        VALUES ($1, $2)
        RETURNING id
        "#, extension, checksum).fetch_optional(&pool).await?.ok_or(AppError::Expected {code: StatusCode::NO_CONTENT, message: "File not found"})?.id;
        Store::new().save(&StoreFile::new(file_id, extension), bytes).await.unwrap();

        query!(r#"
        INSERT INTO bucket_files (name, bucket_id, file_id)
        VALUES ($1, $2, $3)
        "#, name, claims.bucket_id, file_id).execute(&pool).await?;
        file_ids.push(file_id);
    }
    debug!("{file_ids:#?}");
    Ok(Json(file_ids))
}

#[debug_handler]
async fn delete(claims: Claims, State(pool): State<PgPool>, Path(file_id): Path<Uuid>) -> Result<(), AppError> {
    let rec = query!(r#"
    SELECT *
    FROM bucket_files
    WHERE bucket_id = $1 AND file_id = $2
    "#, claims.bucket_id, file_id).fetch_optional(&pool).await?.ok_or(AppError::Expected {code: StatusCode::BAD_REQUEST, message: "File does not exists"})?;

    let rec = query!(r#"
    SELECT COUNT(*)
    FROM bucket_files
    WHERE file_id = $1
    "#, file_id).fetch_optional(&pool).await?.ok_or(AppError::Expected {code: StatusCode::BAD_REQUEST, message: "Trying to access non existing file reference"})?;

    let count = rec.count.ok_or(AppError::Unexpected(anyhow!("Could not count referenced files")))?;
    debug!("File referenced by: {count}");

    query!(r#"
    DELETE FROM bucket_files
    WHERE bucket_id = $1 AND file_id = $2
    "#, claims.bucket_id, file_id).execute(&pool).await?;

    if count == 1 {
        debug!("Deleting file permanently");
        let extension = query!(r#"
        DELETE FROM files
        WHERE id = $1
        RETURNING extension
        "#, file_id).fetch_one(&pool).await?.extension;

        Store::new().remove(&StoreFile::new(file_id, extension)).await.unwrap();
    }

    Ok(())
}

struct StoreFile {
    id: Uuid,
    extension: Option<String>,
}

impl StoreFile {
    fn new(id: Uuid, extension: Option<String>) -> Self {
        Self {
            id,
            extension,
        }
    }

    fn path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(self.id.to_string());
        if let Some(ext) = &self.extension {
            path.set_extension(ext);
        }
        path
    }
}

struct Store;
const STORE_NAME: &str = "store";
impl Store {
    fn new() -> Self {
        Self
    }

    async fn save(&self, file: &StoreFile, contents: Bytes) -> io::Result<()> {
        let path = self.path(file.path());
        tokio::fs::write(&path,contents).await?;
        debug!("Saved file at: {path:?}");
        Ok(())
    }

    async fn remove(&self, file: &StoreFile) -> io::Result<()> {
        let path = self.path(file.path());
        tokio::fs::remove_file(&path).await?;
        debug!("Removed file at: {path:?}");
        Ok(())
    }

    async fn read(&self, file: &StoreFile) -> io::Result<File> {
        let path = self.path(file.path());
        let file = File::open(&path).await?;
        debug!("Read file at: {path:?}");
        Ok(file)
    }

    fn path(&self, other: PathBuf) -> PathBuf {
        PathBuf::from(format!("./{}", STORE_NAME)).join(other)
    }
}
