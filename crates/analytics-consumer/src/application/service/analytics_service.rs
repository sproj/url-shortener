use std::sync::Arc;

use crate::{
    application::{
        repository::{self, redirect_event_repository_trait::RedirectRepositoryTrait},
        service::analytics_consumer_trait::AnalyticsConsumerTrait,
    },
    infrastructure::messaging::messaging_error::MessagingError,
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

    pub async fn run(&self) -> Result<(), MessagingError> {
        loop {
            let (event, handle) = self.consumer.next().await?;
            tracing::debug!("redirect consumer received an event");
            match self.repository.save(&event).await {
                Ok(_) => {
                    tracing::debug!("saved event, acking");
                    handle.ack().await;
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
