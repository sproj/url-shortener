use redis::aio::MultiplexedConnection;
use tracing::instrument;

use crate::application::{config::RedisConfig, startup_error::StartupError};

#[instrument(skip_all)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_returns_startup_error_when_unreachable() {
        let redis_config = RedisConfig {
            redis_host: "127.0.0.1".to_string(),
            redis_port: 1,
        };

        let result = connect(&redis_config).await;

        assert!(matches!(result, Err(StartupError::RedisConnection(_))));
    }
}
