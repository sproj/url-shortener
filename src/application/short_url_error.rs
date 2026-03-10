use thiserror::Error;

use crate::{
    api::error::{ApiError, ApiErrorKind},
    application::repository::database_error::DatabaseError,
    domain::validation_issue::ValidationIssue,
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
    Storage(DatabaseError),
    #[error("code generation exhausted")]
    CodeGenerationExhausted,
}

impl From<DatabaseError> for ShortUrlError {
    fn from(e: DatabaseError) -> Self {
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
        eprintln!("ShortUrlError: {:?}", &short_url_error);
        match short_url_error {
            ShortUrlError::NotFound(id_or_code) => ApiError::new(short_url_error_message)
                .kind(ApiErrorKind::ResourceNotFound)
                .detail(serde_json::json!({ "not_found": id_or_code })),
            ShortUrlError::UnprocessableInput(msg) => ApiError::new("unprocessable_input")
                .kind(ApiErrorKind::UnprocessableInput)
                .detail(serde_json::json!({"invalid_input_url": [{
                        "field": "request_body",
                        "code": "parse_create_short_url_input_fail",
                        "message": msg
                    }]
                })),
            ShortUrlError::InvalidInput(issues) => ApiError::new("input url is invalid")
                .kind(ApiErrorKind::ValidationError)
                .detail(serde_json::json!({"invalid_input_url": issues})),
            ShortUrlError::Storage(e) => {
                eprintln!("Unexpected database error: {:?}", e);
                ApiError::new("internal database error").kind(ApiErrorKind::Internal)
            }
            ShortUrlError::CodeGenerationExhausted => {
                ApiError::new("failed to generate a code").kind(ApiErrorKind::Internal)
            }
        }
    }
}
