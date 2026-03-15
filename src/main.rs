use url_shortener::application::{app::App, config, startup_error::StartupError};
use url_shortener::infrastructure::{database::pool, redis::connect};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    tracing_subscriber::fmt::init();
    let cfg = config::load()?;
    let db_pool = pool::create_db_pool(&cfg)?;
    let redis = connect::connect(&cfg).await?;
    App::builder()
        .with_config(cfg)
        .with_db_pool(db_pool)
        .with_redis(redis)
        .build()
        .await?
        .start()
        .await
}
