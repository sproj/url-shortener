use crate::{
    domain::errors::RepositoryError, infrastructure::database::database_error::DatabaseError,
};

pub mod short_url_repository;
pub mod users_repository;

pub type RepositoryResult<T> = Result<T, RepositoryError>;

impl From<DatabaseError> for RepositoryError {
    fn from(db_err: DatabaseError) -> Self {
        match db_err {
            DatabaseError::Conflict {
                state: _,
                constraint,
                message,
            } => RepositoryError::Conflict {
                constraint,
                message,
            },
            e => RepositoryError::Internal(e.to_string()),
        }
    }
}
