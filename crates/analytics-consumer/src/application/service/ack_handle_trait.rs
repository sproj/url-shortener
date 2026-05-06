use crate::infrastructure::messaging::messaging_error::MessagingError;

#[async_trait::async_trait]
pub trait AckHandle: Send {
    async fn ack(self: Box<Self>) -> Result<(), MessagingError>;
}
