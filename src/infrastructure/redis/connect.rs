use redis::aio::MultiplexedConnection;

use crate::application::{config::Config, startup_error::StartupError};

pub async fn connect(config: &Config) -> Result<MultiplexedConnection, StartupError> {
    match redis::Client::open(config.redis_url()) {
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
