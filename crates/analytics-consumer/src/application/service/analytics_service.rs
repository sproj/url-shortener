use std::sync::Arc;

use crate::application::{
    repository::redirect_event_repository_trait::RedirectRepositoryTrait,
    service::{
        analytics_consumer_trait::AnalyticsConsumerTrait,
        analytics_service_trait::AnalyticsServiceTrait,
    },
    startup_error::StartupError,
};

pub struct AnalyticsService {
    consumer: Arc<dyn AnalyticsConsumerTrait>,
    repository: Arc<dyn RedirectRepositoryTrait>,
}

impl AnalyticsService {
    pub fn new(
        consumer: Arc<dyn AnalyticsConsumerTrait>,
        repository: Arc<dyn RedirectRepositoryTrait>,
    ) -> Self {
        Self {
            consumer,
            repository,
        }
    }
}

#[async_trait::async_trait]
impl AnalyticsServiceTrait for AnalyticsService {
    async fn run(&self) -> Result<(), StartupError> {
        loop {
            let (event, handle) = self
                .consumer
                .next()
                .await
                .map_err(|e| StartupError::RabbitMqConnection(e.to_string()))?;
            tracing::debug!("redirect consumer received an event");
            match self.repository.save(&event).await {
                Ok(_) => {
                    tracing::debug!("saved event, acking");
                    handle
                        .ack()
                        .await
                        .map_err(|e| StartupError::RabbitMqConnection(e.to_string()))?;
                    tracing::debug!("acked");
                    continue;
                }
                Err(e) => {
                    tracing::error!(%e, "repository failed to save event");
                    continue;
                }
            }
        }
    }
}
