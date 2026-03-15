use deadpool_postgres::{Config as PgConfig, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;

use crate::application::{config::Config, startup_error::StartupError};

pub fn create_db_pool(config: &Config) -> Result<Pool, StartupError> {
    tracing::info!("Creating database connection pool");
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
