use std::sync::Arc;

use crate::application::service::short_url::code_generator::{CodeGenerator, RandomCodeGenerator};
use crate::application::service::short_url::redirect_cache::RedirectCacheChecker;
use crate::application::service::short_url::redirect_cache_trait::NoopRedirectCache;
use crate::application::startup_error::StartupError;
use crate::application::state::{AppState, SharedState};
use crate::{api::server, application::config::Config};

use deadpool_postgres::Pool;
use redis::aio::MultiplexedConnection;

pub struct App {
    config: Config,
    state: SharedState,
}

impl App {
    pub fn builder(config: Config, db_pool: Pool) -> AppBuilder {
        AppBuilder::builder(config, db_pool)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn state(&self) -> &SharedState {
        &self.state
    }

    pub async fn start(self) -> Result<(), StartupError> {
        tracing::info!("Starting server");
        server::start(self.config, self.state).await
    }
}

pub struct AppBuilder {
    config: Config,
    db_pool: Pool,
    state: Option<SharedState>,
    redis: Option<MultiplexedConnection>,
    code_generator: Option<Arc<dyn CodeGenerator>>,
    max_retries: Option<u8>,
}

impl AppBuilder {
    pub fn builder(config: Config, db_pool: Pool) -> Self {
        Self {
            config,
            db_pool,
            state: None,
            redis: None,
            code_generator: None,
            max_retries: None,
        }
    }
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    pub fn with_database(mut self, db_pool: Pool) -> Self {
        self.db_pool = db_pool;
        self
    }

    pub fn with_state(mut self, state: SharedState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_redis(mut self, redis: MultiplexedConnection) -> Self {
        self.redis = Some(redis);
        self
    }

    pub fn with_code_generator(mut self, code_generator: Arc<dyn CodeGenerator>) -> Self {
        self.code_generator = Some(code_generator);
        self
    }

    pub fn with_max_retries(mut self, max_retries: u8) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    pub async fn build(self) -> Result<App, StartupError> {
        let state = Arc::new(AppState {
            code_generator: self
                .code_generator
                .unwrap_or_else(|| Arc::new(RandomCodeGenerator)),
            redirect_cache: match self.redis {
                Some(conn) => Arc::new(RedirectCacheChecker::new(conn)),
                None => Arc::new(NoopRedirectCache),
            },
            max_retries: self.config.app.max_retries,
            db_pool: self.db_pool,
            jwt_decoding_key: self.config.jwt.jwt_keys.decoding.clone(),
            jwt_encoding_key: self.config.jwt.jwt_keys.encoding.clone(),
            jwt_access_token_seconds: self.config.jwt.jwt_expire_access_token_seconds,
            jwt_refresh_token_seconds: self.config.jwt.jwt_expire_refresh_token_seconds,
        });
        Ok(App {
            config: self.config,
            state,
        })
    }
}
