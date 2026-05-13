use std::sync::Arc;

use common::repository_error::RepositoryError;

use crate::{
    application::{
        consume_result::ConsumeResult,
        repository::redirect_event_repository_trait::RedirectRepositoryTrait,
        service::{
            analytics_consumer_trait::AnalyticsConsumerTrait,
            analytics_service_trait::AnalyticsServiceTrait,
        },
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
}

#[async_trait::async_trait]
impl AnalyticsServiceTrait for AnalyticsService {
    async fn run(&self) -> Result<(), MessagingError> {
        loop {
            match self.consumer.next().await {
                ConsumeResult::Message(event, handle) => {
                    tracing::debug!("redirect consumer received an event");
                    match self.repository.save(&event).await {
                        Ok(_) => {
                            tracing::debug!("saved event, acking");
                            handle.ack().await?;
                            tracing::debug!("acked");
                            continue;
                        }
                        Err(e) => match e {
                            RepositoryError::Internal(e) => {
                                tracing::error!(%e, "repository query write event");
                                handle.nack(false).await?;
                                continue;
                            }
                            RepositoryError::Pool(e) => {
                                tracing::error!(%e, "repository pool failure");
                                handle.nack(true).await?;
                                continue;
                            }
                            RepositoryError::Conflict {
                                constraint: _,
                                message,
                            } => {
                                tracing::warn!(%message, "duplicate insert failed");
                                handle.ack().await?;
                                continue;
                            }
                        },
                    }
                }
                ConsumeResult::InvalidMessage(error, handle) => {
                    tracing::error!(%error, "redirect event consumer received invalid message.");
                    handle.nack(false).await?;
                    continue;
                }
                ConsumeResult::ChannelError(e) => {
                    tracing::error!(%e, "analytics consumer channel failed");
                    return Err(e);
                }
            }
        }
    }
}
