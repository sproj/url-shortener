use common::events::redirect_event::RedirectEvent;
use futures_lite::stream::StreamExt;
use lapin::{
    Channel, Consumer,
    options::{BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable, ShortString},
};
use tokio::sync::Mutex;
use tracing::instrument;

use crate::{
    application::{
        consume_result::ConsumeResult, service::analytics_consumer_trait::AnalyticsConsumerTrait,
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

const ANALYTICS_DLQ_NAME: &str = "analytics_redirect_events_dlq";
const ANALYTICS_DLX_NAME: &str = "url-shortener-dlx";

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
        self.declare_main_exchange().await;

        self.declare_dlx_exchange().await;

        self.declare_dlq_queue().await;

        self.bind_dlq_to_dlx().await;

        self.declare_main_queue().await;

        self.bind_main_queue_to_main_exchange(routing_key).await;

        let consumer = self.start_basic_consumer().await;

        Ok(Self {
            channel: self.channel,
            exchange_name: self.exchange_name,
            queue_name: self.queue_name,
            consumer: Mutex::new(Some(consumer)),
        })
    }

    async fn declare_main_exchange(&self) {
        self.channel
            .exchange_declare(
                self.exchange_name.clone().into(),
                lapin::ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to declare analytics exchange");
    }

    async fn declare_dlx_exchange(&self) {
        self.channel
            .exchange_declare(
                ANALYTICS_DLX_NAME.into(),
                lapin::ExchangeKind::Direct,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to declare DLX exchange");
    }

    async fn declare_dlq_queue(&self) {
        self.channel
            .queue_declare(
                ANALYTICS_DLQ_NAME.into(),
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .expect("failed to declare analytics DLQ");
    }

    async fn bind_dlq_to_dlx(&self) {
        self.channel
            .queue_bind(
                ANALYTICS_DLQ_NAME.into(),
                ANALYTICS_DLX_NAME.into(),
                ANALYTICS_DLQ_NAME.into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to bind DLQ to DLX");
    }

    async fn declare_main_queue(&self) {
        let mut exchange_args = FieldTable::default();
        exchange_args.insert(
            ShortString::from("x-dead-letter-exchange"),
            AMQPValue::LongString(ANALYTICS_DLX_NAME.into()),
        );
        exchange_args.insert(
            ShortString::from("x-dead-letter-routing-key"),
            AMQPValue::LongString(ANALYTICS_DLQ_NAME.into()),
        );
        self.channel
            .queue_declare(
                self.queue_name.clone().into(),
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                exchange_args,
            )
            .await
            .expect("failed to create analytics queue");
    }

    async fn bind_main_queue_to_main_exchange(&self, routing_key: &str) {
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
    }

    async fn start_basic_consumer(&self) -> Consumer {
        self.channel
            .basic_consume(
                self.queue_name.clone().into(),
                "analytics-consumer".into(),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("failed to create consumer")
    }
}

#[async_trait::async_trait]
impl AnalyticsConsumerTrait for RabbitMqConsumer {
    #[instrument(skip(self))]
    async fn next(&self) -> ConsumeResult {
        let mut guard = self.consumer.lock().await;
        match guard.as_mut() {
            None => ConsumeResult::ChannelError(MessagingError::NoConsumer),

            Some(consumer) => match consumer.next().await {
                Some(Ok(delivery)) => {
                    tracing::debug!("received message");
                    match serde_json::from_slice::<RedirectEvent>(&delivery.data) {
                        Ok(event) => {
                            ConsumeResult::Message(event, Box::new(RabbitMqAckHandle { delivery }))
                        }
                        Err(e) => ConsumeResult::InvalidMessage(
                            MessagingError::Deserialization(e.to_string()),
                            Box::new(RabbitMqAckHandle { delivery }),
                        ),
                    }
                }
                Some(Err(e)) => {
                    tracing::error!(%e, "delivery error encountered");
                    ConsumeResult::ChannelError(MessagingError::RabbitMq(e))
                }
                None => ConsumeResult::ChannelError(MessagingError::EmptyMessage),
            },
        }
    }
}
