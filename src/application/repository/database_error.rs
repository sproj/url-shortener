use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("database connection pool error")]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error("database query error")]
    Query(#[from] tokio_postgres::Error),
    #[error("database row mapping error: {0}")]
    Mapping(String),
}
