use thiserror::Error;

use crate::{
    application::security::auth_error::AuthError, domain::validation_issue::ValidationIssue,
    infrastructure::database::database_error::DatabaseError,
};

#[derive(Debug, Error)]
pub enum UserError {
    #[error("user authentication failed: {0}")]
    AuthenticationError(AuthError),
    #[error("user repository error: {0}")]
    Storage(DatabaseError),
    #[error("invalid user input: {0:?}")]
    InvalidInput(Vec<ValidationIssue>),
    #[error("unprocessable input: {0}")]
    UnprocessableInput(String),
    #[error("user not found: {0}")]
    NotFound(String),
}

impl From<DatabaseError> for UserError {
    fn from(err: DatabaseError) -> Self {
        Self::Storage(err)
    }
}
