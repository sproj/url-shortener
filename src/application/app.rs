use crate::api::server;
use crate::application::config;
use crate::application::startup_error::StartupError;

pub async fn build() -> Result<config::Config, StartupError> {
    let config = config::load()?;

    Ok(config)
}

pub async fn run(config: config::Config) -> Result<(), StartupError> {
    server::start(config).await;
    Ok(())
}
