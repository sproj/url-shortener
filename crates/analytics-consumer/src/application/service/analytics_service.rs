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

pub enum AckDecision {
    Ack,
    Requeue,
    DeadLetter,
}

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

    fn repository_result_to_ack_decision(
        &self,
        result: Result<(), RepositoryError>,
    ) -> AckDecision {
        match result {
            Ok(_) => {
                tracing::debug!("saved event, acking");
                AckDecision::Ack
            }
            Err(RepositoryError::Internal(e)) => {
                tracing::error!(%e, "repository query write failed at database level - dead letter");
                AckDecision::DeadLetter
            }
            Err(RepositoryError::Conflict {
                constraint,
                message,
            }) => {
                if let Some(reason) = constraint {
                    tracing::warn!(%reason, "redirect event constraint violated");
                }
                tracing::warn!(%message, "duplicate insert failed - dead letter");
                AckDecision::DeadLetter
            }
            Err(RepositoryError::Pool(e)) => {
                tracing::error!(%e, "repository pool failure - requeuing");
                AckDecision::Requeue
            }
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
                    let save_result = self.repository.save(&event).await;
                    match self.repository_result_to_ack_decision(save_result) {
                        AckDecision::Ack => {
                            handle.ack().await?;
                            tracing::debug!("message acknowledged");
                            continue;
                        }
                        AckDecision::Requeue => {
                            handle.nack(true).await?;
                            tracing::debug!("message requeued");
                            continue;
                        }
                        AckDecision::DeadLetter => {
                            handle.nack(false).await?;
                            tracing::debug!("message sent to dead letter");
                            continue;
                        }
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
