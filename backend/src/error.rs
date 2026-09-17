use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("ai provider error: {0}")]
    Ai(String),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError::BadRequest(msg.into())
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }
    pub fn ai(msg: impl Into<String>) -> Self {
        AppError::Ai(msg.into())
    }

    fn code(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "bad_request",
            AppError::NotFound(_) => "not_found",
            AppError::Conflict(_) => "conflict",
            AppError::Validation(_) => "validation_error",
            AppError::Database(_) => "database_error",
            AppError::Ai(_) => "ai_error",
            AppError::Internal(_) => "internal_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) | AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Database(_) | AppError::Ai(_) | AppError::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = %self, "request failed");
        }
        let body = Json(json!({
            "error": { "code": self.code(), "message": self.to_string() }
        }));
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
