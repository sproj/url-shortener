use url_shortener::application::{app, startup_error::StartupError};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    let config = app::load().await?;
    let state = app::build(&config).await?;
    app::run(config, state).await?;
    Ok(())
}
