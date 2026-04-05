use url_shortener::application::{app::App, config, startup_error::StartupError};
use url_shortener::infrastructure::{database::postgres::Database, redis::connect};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    tracing_subscriber::fmt::try_init()
        .map_err(|e| StartupError::TracingSubscriber(e.to_string()))?;

    let cfg = config::load()?;
    let db_pool = Database::connect(&cfg.db)?;
    Database::migrate(&db_pool).await?;

    let redis = connect::connect(&cfg.redis).await?;

    App::builder(cfg, db_pool)
        .with_redis(redis)
        .build()
        .await?
        .start()
        .await
}
