use url_shortener::application::{app, startup_error::StartupError};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    let config = app::build().await?;
    app::run(config).await?;
    Ok(())
}
