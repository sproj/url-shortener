use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0}")]
    Missing(String),
    #[error("{0}")]
    Parse(String),
    #[error("{0}")]
    Url(String),
}
