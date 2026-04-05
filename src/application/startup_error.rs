use thiserror::Error;

#[derive(Debug, Error)]
pub enum StartupError {
    #[error("config error")]
    Config(String),
    #[error("failed to create db connection pool")]
    DbPoolCreation(#[from] deadpool_postgres::CreatePoolError),
    #[error("failed to get pool before applying migrations")]
    DbPoolAccess(#[from] deadpool_postgres::PoolError),
    #[error("failed to apply migrations")]
    DbMigrations(#[from] refinery::Error),
    #[error("server startup error")]
    Server(String),
    #[error("redis startup error")]
    RedisConnection(String),
    #[error("tracing subscriber init error: {0}")]
    TracingSubscriber(String),
}
