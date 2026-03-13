use url_shortener::application::{app::App, config, startup_error::StartupError};
use url_shortener::infrastructure;

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    tracing_subscriber::fmt::init();
    let cfg = config::load()?;
    let redis = infrastructure::redis::connect::connect(&cfg).await?;
    App::builder()
        .with_config(cfg)
        .with_redis(redis)
        .build()
        .await?
        .start()
        .await
}
