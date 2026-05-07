use common::events::redirect_event::RedirectEvent;
use futures_lite::stream::StreamExt;
use lapin::{
    Channel, Consumer,
    options::{BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
};
use tokio::sync::Mutex;
use tracing::instrument;

use crate::{
    application::service::{
        ack_handle_trait::AckHandle, analytics_consumer_trait::AnalyticsConsumerTrait,
    },
    infrastructure::messaging::{
        messaging_error::MessagingError, rabbitmq_ack_handle::RabbitMqAckHandle,
    },
};

pub struct RabbitMqConsumer {
    channel: Channel,
    exchange_name: String,
    queue_name: String,
    consumer: Mutex<Option<Consumer>>,
}

impl RabbitMqConsumer {
    pub fn new(channel: Channel, exchange_name: String, queue_name: String) -> Self {
        Self {
            channel,
            exchange_name,
            queue_name,
            consumer: Mutex::new(None),
        }
    }

    pub async fn setup(self, routing_key: &str) -> Result<Self, MessagingError> {
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
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
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

        let consumer = self
            .channel
            .basic_consume(
                self.queue_name.clone().into(),
                "analytics-consumer".into(),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to create consumer");

        Ok(Self {
            channel: self.channel,
            exchange_name: self.exchange_name,
            queue_name: self.queue_name,
            consumer: Mutex::new(Some(consumer)),
        })
    }
}

#[async_trait::async_trait]
impl AnalyticsConsumerTrait for RabbitMqConsumer {
    #[instrument(skip(self))]
    async fn next(&self) -> Result<(RedirectEvent, Box<dyn AckHandle>), MessagingError> {
        let mut guard = self.consumer.lock().await;
        let consumer = guard.as_mut().ok_or(MessagingError::NoConsumer)?;
        if let Some(delivery_result) = consumer.next().await {
            match delivery_result {
                Ok(delivery) => {
                    tracing::debug!("received message");
                    let event = serde_json::from_slice::<RedirectEvent>(&delivery.data)
                        .map_err(|e| MessagingError::Deserialization(e.to_string()))?;
                    Ok((event, Box::new(RabbitMqAckHandle { delivery })))
                }
                Err(e) => {
                    tracing::error!(%e, "delivery error encountered");
                    Err(MessagingError::RabbitMq(e))
                }
            }
        } else {
            Err(MessagingError::EmptyMessage)
        }
    }
}
