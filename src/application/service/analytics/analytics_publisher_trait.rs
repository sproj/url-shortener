use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::infrastructure::messaging::messaging_error::MessagingError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RedirectType {
    Permanent,
    Temporary,
}

#[derive(Debug, Clone, Serialize)]
pub struct RedirectEvent {
    pub code: String,
    pub timestamp: DateTime<Utc>,
    pub redirect_type: RedirectType,
}

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
