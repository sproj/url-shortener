use lapin::{Channel, Connection, ConnectionProperties};
use tracing::instrument;

use crate::application::{config::RabbitMqConfig, startup_error::StartupError};

#[instrument(skip_all)]
pub async fn connect(config: &RabbitMqConfig) -> Result<Channel, StartupError> {
    let url = config.amqp_url();
    match Connection::connect(&url, ConnectionProperties::default()).await {
        Ok(conn) => match conn.create_channel().await {
            Ok(channel) => {
                tracing::info!("Connected to RabbitMQ");
                Ok(channel)
            }
            Err(e) => {
                tracing::error!(%e, "Could not create RabbitMQ channel");
                Err(StartupError::RabbitMqConnection(e.to_string()))
            }
        },
        Err(e) => {
            tracing::error!(%e, "Could not connect to RabbitMQ");
            Err(StartupError::RabbitMqConnection(e.to_string()))
        }
    }
}
