use lapin::options::BasicAckOptions;

use crate::{
    application::service::ack_handle_trait::AckHandle,
    infrastructure::messaging::messaging_error::MessagingError,
};

pub struct RabbitMqAckHandle {
    pub delivery: lapin::message::Delivery,
}

#[async_trait::async_trait]
impl AckHandle for RabbitMqAckHandle {
    async fn ack(self: Box<Self>) -> Result<(), MessagingError> {
        self.delivery.ack(BasicAckOptions::default()).await?;
        Ok(())
    }
}
