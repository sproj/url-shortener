use url_shortener::application::{app, startup_error::StartupError};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    let app_state = app::build().await?;
    app::run(app_state).await?;
    Ok(())
}
