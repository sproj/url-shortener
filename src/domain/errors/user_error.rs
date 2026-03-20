use argon2::password_hash::Error as HashError;
use thiserror::Error;

use crate::{
    domain::validation_issue::ValidationIssue,
    infrastructure::database::database_error::DatabaseError,
};

#[derive(Debug, Error)]
pub enum UserError {
    #[error("failed to hash password: {0}")]
    HashingError(HashError),
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
