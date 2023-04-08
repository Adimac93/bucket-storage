use axum::extract::multipart::MultipartError;
use axum::Json;
use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use serde::Serialize;
use sqlx::Error;
use thiserror::Error;
use tracing::{error};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid request {message}")]
    Expected {
        code: StatusCode,
        message: String
    },
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error)
}

impl AppError {
    pub fn expected(code: StatusCode, message: impl Into<String>) -> Self {
        Self::Expected {code, message: message.into()}
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, info) = match self {
            AppError::Expected {code, message} => {
                error!("{code}: {message}");
                (code, message)
            },
            AppError::Unexpected(error) => {
                error!("{error}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Unexpected server error".to_string())
            },
        };

        (status_code, Json(ErrorResponse {error_info: info})).into_response()
    }
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct ErrorResponse {
    error_info: String,
}

impl From<sqlx::Error> for AppError {
    fn from(e: Error) -> Self {
        Self::Unexpected(anyhow::Error::from(e))
    }
}

impl From<tokio::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Unexpected(anyhow::Error::from(e))
    }
}

impl From<MultipartError> for AppError {
    fn from(e: MultipartError) -> Self {
        Self::expected(e.status(), e.body_text())
    }
}
