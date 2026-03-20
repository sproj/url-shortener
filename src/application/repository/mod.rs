use crate::infrastructure::database::database_error::DatabaseError;

pub mod short_url_repository;
pub mod users_repository;

pub type RepositoryResult<T> = Result<T, DatabaseError>;
