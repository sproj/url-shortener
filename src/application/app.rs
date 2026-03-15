use std::ops::DerefMut;
use std::sync::Arc;

use crate::application::service::short_url::{
    redirect_cache::RedirectCacheChecker, redirect_cache_trait::RedirectCache,
};
use crate::application::startup_error::StartupError;
use crate::application::state::{AppStateBuilder, SharedState};
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

        tracing::info!("Starting server");
        server::start(self.config, self.state).await
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

    pub fn with_redis(mut self, redis: MultiplexedConnection) -> Self {
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
                let state_builder = match self.redis {
                    Some(conn) => {
                        let cache: Arc<dyn RedirectCache> =
                            Arc::new(RedirectCacheChecker::new(conn));
                        self.state_builder.with_redirect_cache(cache)
                    }
                    None => self.state_builder,
                };
                Arc::new(state_builder.build(db_pool))
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
