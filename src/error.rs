use axum::{
    http,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(Debug)]
pub struct AppError {
    message: String,
    code: http::StatusCode,
}

impl AppError {
    pub fn new(code: http::StatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<serde_json::error::Error> for AppError {
    fn from(error: serde_json::error::Error) -> Self {
        AppError {
            code: http::StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Serde JSON Error:\n{}", error),
        }
    }
}

// NOTE: Pattern match for all error types with their respective status codes
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        AppError {
            code: http::StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("SQLx Error:\n{}", error),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError {
            code: http::StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Anyhow Error:\n{}", error),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("{} {:<12} - {}", "ERROR", self.code, self.message);

        (self.code, self.message).into_response()
    }
}
