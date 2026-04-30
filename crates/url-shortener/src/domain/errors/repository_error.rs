use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("repository conflict: constraint={constraint:?}, message={message:?}")]
    Conflict {
        constraint: Option<String>,
        message: String,
    },
    #[error("database level error: {0}")]
    Internal(String),
}
