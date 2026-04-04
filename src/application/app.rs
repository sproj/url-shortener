use std::sync::Arc;

use crate::application::repository::short_url_repository::PostgresShortUrlRepository;
use crate::application::repository::users_repository::PostgresUsersRepository;
use crate::application::service::auth::auth_service::AuthService;
use crate::application::service::auth::refresh_token_cache::RefreshTokenCache;
use crate::application::service::auth::refresh_token_cache_trait::{
    NoopRefreshTokenCache, RefreshTokenCacheTrait,
};
use crate::application::service::short_url::code_generator::{CodeGenerator, RandomCodeGenerator};
use crate::application::service::short_url::redirect_cache::RedirectCacheChecker;
use crate::application::service::short_url::redirect_cache_trait::{
    NoopRedirectCache, RedirectCache,
};
use crate::application::service::short_url::short_url_service::ShortUrlService;
use crate::application::service::user::user_service::UsersService;
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
    redis: Option<MultiplexedConnection>,
    code_generator: Option<Arc<dyn CodeGenerator>>,
}

impl AppBuilder {
    pub fn builder(config: Config, db_pool: Pool) -> Self {
        Self {
            config,
            db_pool,
            redis: None,
            code_generator: None,
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

    pub fn with_redis(mut self, redis: MultiplexedConnection) -> Self {
        self.redis = Some(redis);
        self
    }

    pub fn with_code_generator(mut self, code_generator: Arc<dyn CodeGenerator>) -> Self {
        self.code_generator = Some(code_generator);
        self
    }

    pub async fn build(self) -> Result<App, StartupError> {
        let code_generator: Arc<dyn CodeGenerator> = self
            .code_generator
            .unwrap_or_else(|| Arc::new(RandomCodeGenerator));
        let redirect_cache: Arc<dyn RedirectCache> = match &self.redis {
            Some(conn) => Arc::new(RedirectCacheChecker::new(conn.clone())),
            None => Arc::new(NoopRedirectCache),
        };
        let refresh_token_cache: Arc<dyn RefreshTokenCacheTrait> = match &self.redis {
            Some(conn) => Arc::new(RefreshTokenCache::new(conn.clone())),
            None => Arc::new(NoopRefreshTokenCache),
        };
        let short_url_repository = Arc::new(PostgresShortUrlRepository::new(self.db_pool.clone()));
        let users_repository = Arc::new(PostgresUsersRepository::new(self.db_pool.clone()));

        let user_service = Arc::new(UsersService::new(users_repository.clone()));

        let cfg = self.config.clone();

        let state = Arc::new(AppState {
            db_pool: self.db_pool,
            short_url_service: Arc::new(ShortUrlService::new(
                short_url_repository,
                users_repository.clone(),
                redirect_cache.clone(),
                code_generator.clone(),
                cfg.app.max_retries,
            )),
            user_service: user_service.clone(),
            auth_service: Arc::new(AuthService::new(
                user_service,
                refresh_token_cache.clone(),
                cfg.jwt.jwt_expire_access_token_seconds,
                cfg.jwt.jwt_expire_refresh_token_seconds,
                cfg.jwt.jwt_keys.encoding.clone(),
            )),
            jwt_decoding_key: cfg.jwt.jwt_keys.decoding,
            refresh_token_cache,
            redirect_cache,
            code_generator,
        });
        Ok(App {
            config: self.config,
            state,
        })
    }
}
