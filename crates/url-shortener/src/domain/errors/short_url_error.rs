use serde_json::json;
use thiserror::Error;

use crate::{
    api::error::{ApiError, ApiErrorKind},
    application::security::auth_error::AuthError,
    domain::{errors::RepositoryError, validation_issue::ValidationIssue},
    infrastructure::redis::cache_error::CacheError,
};

#[derive(Debug, Error)]
pub enum ShortUrlError {
    #[error("short_url not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    UnprocessableInput(String),
    #[error("invalid input url: {0:?}")]
    InvalidInput(Vec<ValidationIssue>),
    #[error("data layer error: {0}")]
    Storage(RepositoryError),
    #[error("code generation exhausted")]
    CodeGenerationExhausted,
    #[error("redis error: {0}")]
    Cache(#[from] CacheError),
    #[error("conflict on code creation {0}")]
    Conflict(String),
    #[error("unauthorized short url action {0}")]
    Unauthorized(#[from] AuthError),
}

impl From<RepositoryError> for ShortUrlError {
    fn from(e: RepositoryError) -> Self {
        Self::Storage(e)
    }
}

impl From<ShortUrlError> for ApiError {
    fn from(short_url_error: ShortUrlError) -> Self {
        ApiError::from(&short_url_error)
    }
}

impl From<&ShortUrlError> for ApiError {
    fn from(short_url_error: &ShortUrlError) -> Self {
        let short_url_error_message = &short_url_error.to_string();

        match short_url_error {
            ShortUrlError::NotFound(id_or_code) => {
                tracing::info!(%short_url_error);
                ApiError::new(short_url_error_message)
                    .kind(ApiErrorKind::ResourceNotFound)
                    .detail(json!({ "not_found": id_or_code }))
            }
            ShortUrlError::UnprocessableInput(msg) => {
                tracing::warn!(%short_url_error, "unprocessable short_url input");
                ApiError::new("unprocessable_input")
                    .kind(ApiErrorKind::UnprocessableInput)
                    .detail(json!({"invalid_input_url": [{
                            "field": "request_body",
                            "code": "parse_create_short_url_input_fail",
                            "message": msg
                        }]
                    }))
            }
            ShortUrlError::InvalidInput(issues) => {
                tracing::warn!(%short_url_error, "invalid user input");
                ApiError::new("input url is invalid")
                    .kind(ApiErrorKind::ValidationError)
                    .detail(json!({"invalid_input_url": issues}))
            }
            ShortUrlError::Storage(e) => {
                tracing::error!(%e, "unexpected database error");
                ApiError::new("internal database error").kind(ApiErrorKind::Internal)
            }
            ShortUrlError::CodeGenerationExhausted => {
                tracing::error!(%short_url_error, "code generation exhausted");
                ApiError::new("failed to generate a code").kind(ApiErrorKind::Internal)
            }
            ShortUrlError::Cache(e) => {
                tracing::error!(%e, "cache level error");
                ApiError::new("cache layer caused error").kind(ApiErrorKind::Internal)
            }
            ShortUrlError::Conflict(e) => {
                tracing::error!("conflict on attempted vanity url creation");
                ApiError::new(e).kind(ApiErrorKind::Conflict).message(e)
            }
            ShortUrlError::Unauthorized(e) => {
                tracing::warn!("action on short url attempted which failed authorization check");
                ApiError::new(e.to_string()).kind(ApiErrorKind::Forbidden)
            }
        }
    }
}
