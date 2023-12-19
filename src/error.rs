use axum::{
    http,
    response::{IntoResponse, Response},
};

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

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        AppError {
            code: http::StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("SQLx Error: {}", error),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError {
            code: http::StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("Anyhow Error: {}", error),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        println!("->> {self:?}\n");

        (self.code, self.message).into_response()
    }
}
