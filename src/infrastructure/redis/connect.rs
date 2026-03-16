use redis::aio::MultiplexedConnection;

use crate::application::{config::RedisConfig, startup_error::StartupError};

pub async fn connect(config: &RedisConfig) -> Result<MultiplexedConnection, StartupError> {
    let url = format!("redis://{}:{}", config.redis_host, config.redis_port);
    match redis::Client::open(url) {
        Ok(client) => match client.get_multiplexed_async_connection().await {
            Ok(connection) => {
                tracing::info!("Connected to redis");
                Ok(connection)
            }
            Err(e) => {
                tracing::error!(%e, "Could not connect to redis");
                Err(StartupError::RedisConnection(e.to_string()))
            }
        },
        Err(e) => {
            tracing::error!(%e, "Could not open redis");
            Err(StartupError::RedisConnection(e.to_string()))
        }
    }
}
