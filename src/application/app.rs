use std::ops::DerefMut;
use std::sync::Arc;

use crate::application::startup_error::StartupError;
use crate::application::state::{AppStateBuilder, SharedState};
use crate::infrastructure;
use crate::{api::server, application::config::Config};

use deadpool_postgres::{Config as PgConfig, ManagerConfig, Pool, RecyclingMethod};
use redis::aio::MultiplexedConnection;
use tokio_postgres::NoTls;

pub struct App {
    config: Config,
    state: SharedState,
    auto_migrate: bool,
}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::default()
    }

    pub async fn run() -> Result<(), StartupError> {
        Self::builder().build().await?.start().await
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn state(&self) -> &SharedState {
        &self.state
    }

    pub async fn migrate(self) -> Result<Self, StartupError> {
        let report = Self::run_migrations(&self.state).await?;
        tracing::info!(?report, "Migrations applied");
        Ok(self)
    }

    pub async fn start(self) -> Result<(), StartupError> {
        if self.auto_migrate {
            let report = Self::run_migrations(&self.state).await?;
            tracing::info!(?report, "Migrations applied");
        }

        Self::run_server(self.config, self.state).await
    }

    async fn run_migrations(state: &SharedState) -> Result<refinery::Report, StartupError> {
        tracing::info!("Running migrations");
        let mut conn = state.db_pool.get().await?;
        let client = conn.deref_mut().deref_mut();
        embedded::migrations::runner()
            .run_async(client)
            .await
            .map_err(StartupError::DbMigrations)
    }

    async fn run_server(config: Config, state: SharedState) -> Result<(), StartupError> {
        tracing::info!("Starting server");
        server::start(config, state).await
    }
}

pub struct AppBuilder {
    config: Option<Config>,
    db_pool: Option<Pool>,
    state: Option<SharedState>,
    state_builder: AppStateBuilder,
    auto_migrate: bool,
    redis: Option<MultiplexedConnection>,
}

impl AppBuilder {
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_db_pool(mut self, db_pool: Pool) -> Self {
        self.db_pool = Some(db_pool);
        self
    }

    pub fn with_state(mut self, state: SharedState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_state_builder(mut self, state_builder: AppStateBuilder) -> Self {
        self.state_builder = state_builder;
        self
    }

    pub fn with_auto_migrate(mut self, auto_migrate: bool) -> Self {
        self.auto_migrate = auto_migrate;
        self
    }

    pub async fn with_redis(mut self, redis: MultiplexedConnection) -> Self {
        self.redis = Some(redis);
        self
    }

    pub async fn build(self) -> Result<App, StartupError> {
        let config = match self.config {
            Some(config) => config,
            None => Self::load_config()?,
        };

        let state = match self.state {
            Some(state) => state,
            None => {
                let db_pool = match self.db_pool {
                    Some(pool) => pool,
                    None => Self::create_db_pool(&config)?,
                };
                let redis = match self.redis {
                    Some(conn) => conn,
                    None => Self::create_redis_connection(&config).await?,
                };

                Arc::new(self.state_builder.build(db_pool, redis))
            }
        };

        Ok(App {
            config,
            state,
            auto_migrate: self.auto_migrate,
        })
    }

    fn load_config() -> Result<Config, StartupError> {
        tracing::info!("Loading config");
        crate::application::config::load()
    }

    fn create_db_pool(config: &Config) -> Result<Pool, StartupError> {
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

    async fn create_redis_connection(
        config: &Config,
    ) -> Result<MultiplexedConnection, StartupError> {
        infrastructure::redis::connect::connect(config).await
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            config: None,
            db_pool: None,
            state: None,
            state_builder: AppStateBuilder::default(),
            auto_migrate: true,
            redis: None,
        }
    }
}

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("src/infrastructure/database/migrations");
}
