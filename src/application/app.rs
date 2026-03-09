use std::ops::DerefMut;
use std::sync::Arc;

use crate::application::config;
use crate::application::startup_error::StartupError;
use crate::application::state::{AppStateBuilder, SharedState};
use crate::{api::server, application::config::Config};

use deadpool_postgres::{Config as PgConfig, ManagerConfig, Pool, RecyclingMethod};
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
        let report = crate::application::app::migrate(&self.state).await?;
        println!("{:?}", report.applied_migrations());
        Ok(self)
    }

    pub async fn start(self) -> Result<(), StartupError> {
        if self.auto_migrate {
            let report = crate::application::app::migrate(&self.state).await?;
            println!("{:?}", report.applied_migrations());
        }

        crate::application::app::run(self.config, self.state).await
    }
}

pub struct AppBuilder {
    config: Option<Config>,
    db_pool: Option<Pool>,
    state: Option<SharedState>,
    state_builder: AppStateBuilder,
    auto_migrate: bool,
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

    pub async fn build(self) -> Result<App, StartupError> {
        let config = match self.config {
            Some(config) => config,
            None => load().await?,
        };

        let state = match self.state {
            Some(state) => state,
            None => {
                let db_pool = match self.db_pool {
                    Some(pool) => pool,
                    None => create_db_pool(&config)?,
                };
                Arc::new(self.state_builder.build(db_pool))
            }
        };

        Ok(App {
            config,
            state,
            auto_migrate: self.auto_migrate,
        })
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
        }
    }
}

pub async fn load() -> Result<Config, StartupError> {
    println!("Loading config");
    config::load()
}

pub async fn build(config: &Config) -> Result<SharedState, StartupError> {
    build_with_state_builder(config, AppStateBuilder::default()).await
}

pub async fn build_with_state_builder(
    config: &Config,
    state_builder: AppStateBuilder,
) -> Result<SharedState, StartupError> {
    println!("Creating AppState");

    let pool = create_db_pool(config)?;

    println!("Setting AppState");
    Ok(Arc::new(state_builder.build(pool)))
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
