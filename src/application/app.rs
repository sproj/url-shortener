use std::sync::Arc;

use crate::application::config;
use crate::application::startup_error::StartupError;
use crate::application::state::{AppState, SharedState};
use crate::{api::server, application::config::Config};

use deadpool_postgres::{Config as PgConfig, ManagerConfig, RecyclingMethod};
use tokio_postgres::NoTls;

pub async fn load() -> Result<Config, StartupError> {
    config::load()
}

pub async fn build(config: &Config) -> Result<SharedState, StartupError> {
    let mut pg = PgConfig::new();
    pg.user = Some(config.postgres_user.clone());
    pg.password = Some(config.postgres_password.clone());
    pg.host = Some(config.postgres_host.clone());
    pg.port = Some(config.postgres_port);
    pg.dbname = Some(config.postgres_db.clone());
    pg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: config.postgres_connection_pool as usize,
        ..Default::default()
    });

    pg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    let pool = pg
        .create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)
        .map_err(|e| StartupError::Db(e.to_string()))?;

    Ok(Arc::new(AppState { db_pool: pool }))
}

pub async fn run(config: config::Config, state: SharedState) -> Result<(), StartupError> {
    server::start(config, state).await;
    Ok(())
}
