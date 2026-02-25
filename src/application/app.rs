use crate::api::server;
use crate::application::config;
use crate::application::startup_error::StartupError;
use crate::application::state::AppState;

pub async fn build() -> Result<AppState, StartupError> {
    let config = config::load()?;

    Ok(AppState { config })
}

pub async fn run(state: AppState) -> Result<(), StartupError> {
    server::start(state).await;
    Ok(())
}
