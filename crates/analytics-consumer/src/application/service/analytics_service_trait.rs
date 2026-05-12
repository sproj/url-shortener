use crate::infrastructure::messaging::messaging_error::MessagingError;

#[async_trait::async_trait]
pub trait AnalyticsServiceTrait: Send + Sync {
    async fn run(&self) -> Result<(), MessagingError>;
}
