use lapin::{BasicProperties, Channel, options::BasicPublishOptions, types::ShortString};
use tracing::instrument;

use crate::{
    application::service::analytics::analytics_publisher_trait::{
        AnalyticsPublisherTrait, RedirectEvent,
    },
    infrastructure::messaging::messaging_error::MessagingError,
};

pub struct RabbitMqPublisher {
    channel: Channel,
    exchange: String,
    routing_key: String,
}

impl RabbitMqPublisher {
    pub fn new(channel: Channel, exchange: String, routing_key: String) -> Self {
        Self {
            channel,
            exchange,
            routing_key,
        }
    }
}

#[async_trait::async_trait]
impl AnalyticsPublisherTrait for RabbitMqPublisher {
    #[instrument(skip(self))]
    async fn publish(&self, event: RedirectEvent) -> Result<(), MessagingError> {
        let payload = serde_json::to_vec(&event)?;
        self.channel
            .basic_publish(
                ShortString::from(self.exchange.as_str()),
                ShortString::from(self.routing_key.as_str()),
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?;
        Ok(())
    }
}
