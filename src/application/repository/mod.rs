use crate::application::repository::database_error::DatabaseError;

pub mod database_error;
pub mod short_url_repository;

pub type RepositoryResult<T> = Result<T, DatabaseError>;
