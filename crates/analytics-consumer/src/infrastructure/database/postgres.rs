use std::ops::DerefMut;

use deadpool_postgres::{Config as PgConfig, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::NoTls;
use tracing::instrument;

use crate::application::{config::DbConfig, startup_error::StartupError};

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("src/infrastructure/database/migrations");
}

pub struct Database;

impl Database {
    #[instrument(skip(config))]
    pub fn connect(config: &DbConfig) -> Result<Pool, StartupError> {
        tracing::info!("Creating database connection pool");

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
            .map_err(StartupError::DbPoolCreation)?;
        tracing::info!("Database pool creation complete");

        Ok(pool)
    }

    #[instrument(skip_all)]
    pub async fn migrate(db_pool: &Pool) -> Result<refinery::Report, StartupError> {
        tracing::info!("Running migrations");

        let mut conn = db_pool.get().await?;
        let client = conn.deref_mut().deref_mut();

        let report = embedded::migrations::runner()
            .run_async(client)
            .await
            .map_err(StartupError::DbMigrations)?;

        tracing::info!(?report, "Database migration compelte");

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrate_returns_pool_access_error_when_db_unreachable() {
        let db_config = DbConfig {
            postgres_user: "admin".to_string(),
            postgres_password: "password".to_string(),
            postgres_host: "127.0.0.1".to_string(),
            postgres_port: 1,
            postgres_db: "missing".to_string(),
            postgres_connection_pool: 1,
        };

        let pool = Database::connect(&db_config).unwrap();
        let result = Database::migrate(&pool).await;

        assert!(matches!(result, Err(StartupError::DbPoolAccess(_))));
    }
}
