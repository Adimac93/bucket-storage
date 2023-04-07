use axum::Json;
use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use serde::Serialize;
use sqlx::Error;
use thiserror::Error;
use tracing::{debug, error};
use tracing::log::trace;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid request {message}")]
    Expected {
        code: StatusCode,
        message: &'static str
    },
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error)
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code = match &self {
            AppError::Expected {code, message} => code.to_owned(),
            AppError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let info = match self {
            AppError::Unexpected(error) => {
                error!("{error}");
                "Unexpected server error".to_string()
            }
            AppError::Expected {code, message} => {
                error!("{code}: {message}");
                self.to_string()
            }
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

