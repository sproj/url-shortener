use crate::application::repository::database_error::DatabaseError;

pub mod short_url_repository;
pub mod database_error;

pub type RepositoryResult<T> = Result<T, DatabaseError>;
