use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use granit_api::ApiErrorBody;

/// Errors surfaced by the backup server, mapped onto HTTP responses.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("invalid or missing API key")]
    Unauthorized,
    #[error("backup not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("object storage error: {0}")]
    S3(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = match &self {
            ServerError::Unauthorized => StatusCode::UNAUTHORIZED,
            ServerError::NotFound => StatusCode::NOT_FOUND,
            ServerError::Conflict(_) => StatusCode::CONFLICT,
            ServerError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ServerError::Config(_) | ServerError::Db(_) | ServerError::S3(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        // Internal failure details go to the log, not to the client.
        let message = if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("internal error: {self}");
            "internal server error".to_string()
        } else {
            self.to_string()
        };
        (status, Json(ApiErrorBody { message })).into_response()
    }
}
