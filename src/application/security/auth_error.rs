use argon2::password_hash::Error as HashError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use thiserror::Error;

use crate::infrastructure::redis::cache_error::CacheError;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("failed to hash password input: {0}")]
    HashingError(HashError),
    #[error("provided credentials incorrect")]
    IncorrectCredentials,
    #[error("no credentials provided")]
    MissingCredentials,
    #[error("failed to create token")]
    TokenCreation,
    #[error("provided token incorrect")]
    InvalidToken,
    #[error("forbidden")]
    Forbidden,
    #[error("token signature has expired")]
    ExpiredSignature(String),
    #[error("cache layer error: {0}")]
    CachingError(CacheError),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AuthError::HashingError(_e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create hash")
            }
            AuthError::Forbidden => (StatusCode::FORBIDDEN, "Can't do that"),
            AuthError::IncorrectCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::ExpiredSignature(_) => (StatusCode::UNAUTHORIZED, "Token signature expired"),
            AuthError::CachingError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "cache layer error"),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}
