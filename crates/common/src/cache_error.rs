use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("redis pool connection error")]
    PoolError(#[from] deadpool_redis::PoolError),
    #[error(transparent)]
    RedisError(#[from] redis::RedisError),
    #[error("serialization error {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for CacheError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}
