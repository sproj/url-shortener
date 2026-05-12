use common::events::redirect_event::RedirectEvent;

use crate::{
    application::service::ack_handle_trait::AckHandle,
    infrastructure::messaging::messaging_error::MessagingError,
};

pub enum ConsumeResult {
    Message(RedirectEvent, Box<dyn AckHandle>),
    InvalidMessage(MessagingError, Box<dyn AckHandle>),
    ChannelError(MessagingError),
}
