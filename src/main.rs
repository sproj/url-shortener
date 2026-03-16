use url_shortener::application::{app::App, config, startup_error::StartupError};
use url_shortener::infrastructure::{database::database, redis::connect};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    tracing_subscriber::fmt::init();
    let cfg = config::load()?;
    let db_pool = database::Database::connect(&cfg.db)?;
    database::Database::migrate(&db_pool).await?;
    let redis = connect::connect(&cfg.redis).await?;

    App::builder(cfg, db_pool)
        .with_redis(redis)
        .build()
        .await?
        .start()
        .await
}
