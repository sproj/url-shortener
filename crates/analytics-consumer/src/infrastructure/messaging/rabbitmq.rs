use deadpool_postgres::Pool;
use futures_lite::stream::StreamExt;
use lapin::{
    Channel,
    options::{
        BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
};
use tracing::instrument;

use crate::{
    application::service::analytics_consumer_trait::AnalyticsConsumerTrait,
    infrastructure::messaging::messaging_error::MessagingError,
};

pub struct RabbitMqConsumer {
    channel: Channel,
    exchange_name: String,
    queue_name: String,
}

impl RabbitMqConsumer {
    pub fn new(channel: Channel, exchange_name: String, queue_name: String) -> Self {
        Self {
            channel,
            exchange_name,
            queue_name,
        }
    }
}

#[async_trait::async_trait]
impl AnalyticsConsumerTrait for RabbitMqConsumer {
    #[instrument(skip(self))]
    async fn consume(&self, routing_key: &str) -> Result<(), MessagingError> {
        self.channel
            .exchange_declare(
                self.exchange_name.clone().into(),
                lapin::ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to declare exchange");
        self.channel
            .queue_declare(
                self.queue_name.clone().into(),
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to create queue");

        self.channel
            .queue_bind(
                self.queue_name.clone().into(),
                self.exchange_name.clone().into(),
                routing_key.to_string().into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to bind queue");

        let mut consumer = self
            .channel
            .basic_consume(
                self.queue_name.clone().into(),
                "analytics-consumer".into(),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to create consumer");

        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(d) => {
                    tracing::debug!("received message");
                    d.ack(BasicAckOptions::default());
                }
                Err(e) => {
                    tracing::error!(%e, "delivery error encountered");
                }
            }
        }

        Ok(())
    }
}
