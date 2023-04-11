use anyhow::anyhow;
use argon2::password_hash::{SaltString};
use argon2::{Argon2, password_hash, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::{async_trait, Json, Router};
use axum::response::IntoResponse;
use axum::routing::get;
use base64::Engine;
use rand::thread_rng;
use serde_json::json;
use sqlx::{PgPool, query};
use sqlx::types::Uuid;
use tracing::debug;
use crate::AppState;
use crate::errors::AppError;


pub fn router() -> Router<AppState> {
    Router::new()
        .route("/key", get(issue_key))
        .route("/key/verify", get(verify_key))
}

async fn verify_key(claims: Claims) -> impl IntoResponse {
    debug!("Verified key with access to bucket: {}", claims.bucket_id);
    "Authorized access"
}

async fn issue_key(State(pool): State<PgPool>) -> Result<impl IntoResponse, AppError> {
    let mut transaction = pool.begin().await?;

    let bucket_name = "bucket";
    let bucket_id = query!(r#"
    INSERT INTO buckets (name)
    VALUES ($1)
    RETURNING id
    "#, bucket_name).fetch_one(&mut transaction).await?.id;

    let key = Uuid::new_v4().to_string();
    let key_id = query!(r#"
    INSERT INTO bucket_keys (key, bucket_id)
    VALUES ($1, $2)
    RETURNING id
    "#, ArgonHash::hash(&key)?, bucket_id).fetch_one(&mut transaction).await?.id;

    transaction.commit().await?;
    debug!("Issued new bucket key");
    Ok(Json(json!({"id": key_id, "key": key})))
}

pub struct Claims {
    pub key_id: Uuid,
    pub bucket_id: Uuid
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

pub struct ArgonHash;

impl ArgonHash {
    pub fn hash(input: &str) -> anyhow::Result<String> {
        let salt = SaltString::generate(thread_rng());
        let hash = Argon2::default().hash_password(input.as_bytes(), &salt).map_err(|e| anyhow!("Failed to hash password: {e}"))?;
        Ok(hash.to_string())
    }

    pub fn verify(input: &str, hash: &str) -> anyhow::Result<bool> {
        let hash = PasswordHash::new(hash).map_err(|e| anyhow!("Invalid password hash: {e}"))?;
        let res = Argon2::default().verify_password(input.as_bytes(), &hash);
        match res {
            Ok(()) => Ok(true),
            Err(password_hash::Error::Password) => Ok(false),
            Err(e) => Err(anyhow!("Failed to verify password: {e}")),
        }
    }
}

#[derive(Debug)]
pub struct Credentials {
    pub key_id: Uuid,
    key: String
}

impl Credentials {
    fn new(id: Uuid, key: String) -> Self {
        Self { key_id: id, key}
    }
}

pub async fn verify_credentials(pool: &PgPool, credentials: &Credentials) -> Result<Uuid, AppError> {
    let rec = query!(r#"
    SELECT *
    FROM bucket_keys
    WHERE id = $1
    "#, credentials.key_id).fetch_optional(pool).await?;

    if let Some(rec) = rec {
        let is_correct = ArgonHash::verify(&credentials.key, &rec.key)?;
        if is_correct {
            return Ok(rec.bucket_id);
        }
        return Err(AppError::expected(StatusCode::BAD_REQUEST, "Failed to verify credentials: incorrect key"))
    }

    Err(AppError::expected(StatusCode::BAD_REQUEST, "Failed to verify credentials: invalid key_id"))
}

pub async fn get_auth_parts(parts: &mut Parts) -> Result<Credentials, AppError>{
    let authorization = get_auth_header(parts)?;
    let split = authorization.split_once(' ');
    match split {
        Some((name, contents)) if name == "Basic" => {
            Ok(decode(contents)?)
        }
        _ => Err(AppError::expected(StatusCode::BAD_REQUEST, "`Authorization` header must be for basic authentication"))
    }

}

fn decode(input: &str) -> Result<Credentials, AppError> {
    let decoded = base64::engine::general_purpose::STANDARD.decode(input)
        .map_err(|_| AppError::expected(StatusCode::BAD_REQUEST, "Unprocessable base64"))?;
    let decoded = String::from_utf8(decoded)
        .map_err(|_| AppError::expected(StatusCode::BAD_REQUEST, "Unprocessable characters"))?;

    if let Some((id, password)) = decoded.split_once(':') {
        debug!("{id} {password}");
        let id = Uuid::parse_str(id)
            .map_err(|_| AppError::expected( StatusCode::BAD_REQUEST, "Key id is not of a type UUID"))?;
        return Ok(Credentials::new(id, password.to_owned()));
    }
    Err(AppError::expected( StatusCode::BAD_REQUEST, "Missing `:` delimiter"))
}

pub fn get_auth_header(parts: &mut Parts) -> Result<&str, AppError> {
    parts
        .headers
        .get(AUTHORIZATION)
        .ok_or(AppError::expected(StatusCode::BAD_REQUEST, "`Authorization` header is missing" ))?
        .to_str()
        .map_err(|_| AppError::expected( StatusCode::BAD_REQUEST, "`Authorization` header contains invalid characters" ))
}