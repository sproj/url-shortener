use common::events::redirect_event::RedirectEvent;

use crate::infrastructure::messaging::messaging_error::MessagingError;

#[async_trait::async_trait]
pub trait AnalyticsPublisherTrait: Send + Sync {
    async fn publish(&self, event: RedirectEvent) -> Result<(), MessagingError>;
}

pub struct NoopAnalyticsPublisher;

#[async_trait::async_trait]
impl AnalyticsPublisherTrait for NoopAnalyticsPublisher {
    async fn publish(&self, _event: RedirectEvent) -> Result<(), MessagingError> {
        Ok(())
    }
}
