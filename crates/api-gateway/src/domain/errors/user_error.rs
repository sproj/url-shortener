use auth::auth_error::AuthError;
use thiserror::Error;

use crate::domain::errors::RepositoryError;
use crate::domain::validation_issue::ValidationIssue;

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
    #[error("duplicate user creation: {0}")]
    Conflict(String),
}

impl From<RepositoryError> for UserError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::Conflict { .. } => {
                tracing::warn!(%err, "attempted to create a duplicate user");
                Self::Conflict(err.to_string())
            }
            _ => {
                tracing::error!(%err, "create user failed");
                Self::Storage(err)
            }
        }
    }
}
