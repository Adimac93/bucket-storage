use std::path::PathBuf;
use anyhow::anyhow;
use axum::extract::{FromRef, FromRequestParts, Multipart, Path, State};
use axum::{debug_handler, Json, Router, TypedHeader};
use axum::body::{Body, Bytes, StreamBody};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use serde::Serialize;
use sha1::{Sha1, Digest};
use sqlx::{PgPool, query};
use tokio::{fs, io};
use tokio::fs::{File, write};
use tokio_util::io::ReaderStream;
use tracing::{debug, error};
use uuid::Uuid;
use crate::AppState;
use crate::auth::{ArgonHash, Claims, Credentials, get_auth_parts, verify_credentials};
use crate::errors::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/download/:file_id", get(download))
        .route("/upload/key", get(upload_url))
        .route("/upload", post(upload))
        .route("/upload/:upload_id", post(upload_with_key))
        .route("/delete/:file_id", get(delete))
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

    let file = Store::new().read(&StoreFile::new(file_id, res.extension.clone())).await?;
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



#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct UploadKey {
    upload_id: Uuid
}

async fn upload_url(claims: Claims, State(pool): State<PgPool>) -> Result<Json<UploadKey>, AppError>{
    let upload_id = query!(r#"
    INSERT INTO upload_keys (bucket_id)
    VALUES ($1)
    RETURNING id
    "#, claims.bucket_id).fetch_one(&pool).await?.id;

    debug!("Issued new upload id");
    Ok(Json(UploadKey {upload_id}))
}

#[debug_handler]
async fn upload_with_key(State(pool): State<PgPool>, Path(upload_id): Path<Uuid>, multipart: Multipart) -> Result<Json<Vec<Uuid>>, AppError> {
    let bucket_id = query!(r#"
    SELECT bucket_id
    FROM upload_keys
    WHERE id = $1
    "#, upload_id).fetch_optional(&pool).await?.ok_or(AppError::Expected {code: StatusCode::BAD_REQUEST, message: "Wrong upload key"})?.bucket_id;

    let file_ids = save_multipart(&pool, multipart, bucket_id).await?;
    debug!("Uploaded files with upload key");
    Ok(Json(file_ids))
}

#[debug_handler]
async fn upload(claims: Claims, State(pool): State<PgPool>, mut multipart: Multipart) -> Result<Json<Vec<Uuid>>, AppError> {
    debug!("Received multipart form");
    let file_ids = save_multipart(&pool, multipart, claims.bucket_id).await?;
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

async fn save_multipart(pool: &PgPool, mut multipart: Multipart, bucket_id: Uuid) -> Result<Vec<Uuid>, AppError> {
    let mut transaction = pool.begin().await?;
    let mut file_ids = Vec::new();
    while let Some(field) = multipart.next_field().await.unwrap() {
        let (name, extension) = if let Some(file_name) = field.file_name() {
            let file_name_parts = file_name.rsplit_once('.');
            match file_name_parts {
                None => (file_name.to_string(), None),
                Some((name,extension)) => (name.to_string(), Some(extension.to_string()))
            }
        } else {
            error!("Missing file name");
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
        "#, checksum).fetch_optional(&mut transaction).await?;

        if let Some(file) = file {
            debug!("Matching file checksum");
            file_ids.push(file.id);
            continue
        }

        let file_id = query!(r#"
        INSERT INTO files (extension, checksum)
        VALUES ($1, $2)
        RETURNING id
        "#, extension, checksum).fetch_optional(&mut transaction).await?.ok_or(AppError::Expected {code: StatusCode::NO_CONTENT, message: "File not found"})?.id;
        Store::new().save(&StoreFile::new(file_id, extension), bytes).await?;

        query!(r#"
        INSERT INTO bucket_files (name, bucket_id, file_id)
        VALUES ($1, $2, $3)
        "#, name, bucket_id, file_id).execute(&mut transaction).await?;
        file_ids.push(file_id);
    }
    transaction.commit().await?;
    debug!("{file_ids:#?}");
    Ok(file_ids)
}
