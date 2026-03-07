use std::ops::DerefMut;
use std::sync::Arc;

use crate::application::config;
use crate::application::repository::short_url_repository::ShortUrlRepository;
use crate::application::service::short_url::short_url_service::ShortUrlService;
use crate::application::startup_error::StartupError;
use crate::application::state::{AppState, SharedState};
use crate::{api::server, application::config::Config};

use deadpool_postgres::{Config as PgConfig, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;

pub async fn load() -> Result<Config, StartupError> {
    println!("Loading config");
    config::load()
}

pub async fn build(config: &Config) -> Result<SharedState, StartupError> {
    println!("Creating AppState");

    let pool = create_db_pool(config)?;

    println!("Creating domain services");
    let short_url_repository = ShortUrlRepository::new(pool.clone());
    let short_url_service = Arc::new(ShortUrlService::new(short_url_repository));

    Ok(Arc::new(AppState {
        db_pool: pool,
        short_url: short_url_service,
    }))
}

fn create_db_pool(config: &Config) -> Result<Pool, StartupError> {
    println!("Creating database connection pool");
    let mut pg = PgConfig::new();
    pg.user = Some(config.db.postgres_user.clone());
    pg.password = Some(config.db.postgres_password.clone());
    pg.host = Some(config.db.postgres_host.clone());
    pg.port = Some(config.db.postgres_port);
    pg.dbname = Some(config.db.postgres_db.clone());
    pg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: config.db.postgres_connection_pool as usize,
        ..Default::default()
    });

    pg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    pg.create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)
        .map_err(StartupError::DbPoolCreation)
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("src/infrastructure/database/migrations");
}

pub async fn migrate(state: &SharedState) -> Result<refinery::Report, StartupError> {
    println!("Running migrations");
    let mut conn = state.db_pool.get().await?;
    let client = conn.deref_mut().deref_mut();
    embedded::migrations::runner()
        .run_async(client)
        .await
        .map_err(StartupError::DbMigrations)
}

pub async fn run(config: config::Config, state: SharedState) -> Result<(), StartupError> {
    println!("Starting server");
    server::start(config, state).await;
    Ok(())
}
