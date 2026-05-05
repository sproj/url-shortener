use crate::infrastructure::messaging::messaging_error::MessagingError;

#[async_trait::async_trait]
pub trait AnalyticsConsumerTrait: Send + Sync {
    async fn consume(&self, routing_key: &str) -> Result<(), MessagingError>;
}
