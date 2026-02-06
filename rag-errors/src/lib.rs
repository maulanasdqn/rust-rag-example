use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Invalid document format: {0}")]
    InvalidDocumentFormat(String),

    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("OpenAI API error: {0}")]
    OpenAIError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::DocumentNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::InvalidDocumentFormat(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::ConfigError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error".to_string())
            }
            AppError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            AppError::OpenAIError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "AI service error".to_string())
            }
            AppError::EmbeddingError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Embedding error".to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(ErrorResponse { error: message });
        (status, body).into_response()
    }
}
