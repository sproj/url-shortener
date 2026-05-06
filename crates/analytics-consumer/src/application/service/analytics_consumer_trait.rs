use common::events::redirect_event::RedirectEvent;

use crate::{
    application::service::ack_handle_trait::AckHandle,
    infrastructure::messaging::messaging_error::MessagingError,
};

#[async_trait::async_trait]
pub trait AnalyticsConsumerTrait: Send + Sync {
    async fn next(&self) -> Result<(RedirectEvent, Box<dyn AckHandle>), MessagingError>;
}
