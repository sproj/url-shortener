use thiserror::Error;

use crate::{
    application::security::auth_error::AuthError,
    domain::{errors::RepositoryError, validation_issue::ValidationIssue},
};

#[derive(Debug, Error)]
pub enum UserError {
    #[error("user authentication failed: {0}")]
    AuthenticationError(AuthError),
    #[error("user repository error: {0}")]
    Storage(RepositoryError),
    #[error("invalid user input: {0:?}")]
    InvalidInput(Vec<ValidationIssue>),
    #[error("unprocessable input: {0}")]
    UnprocessableInput(String),
    #[error("user not found: {0}")]
    NotFound(String),
}

impl From<RepositoryError> for UserError {
    fn from(err: RepositoryError) -> Self {
        Self::Storage(err)
    }
}
