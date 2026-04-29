use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessagingError {
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error(transparent)]
    RabbitMq(#[from] lapin::Error),
}

impl From<serde_json::Error> for MessagingError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}
