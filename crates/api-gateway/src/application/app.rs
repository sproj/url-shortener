use std::sync::Arc;

use deadpool_postgres::Pool;
use redis::aio::MultiplexedConnection;

use crate::application::{
    config::Config,
    repository::users_repository::PostgresUsersRepository,
    service::{
        auth::{
            auth_service::AuthService,
            refresh_token_cache::RefreshTokenCache,
            refresh_token_cache_trait::{NoopRefreshTokenCache, RefreshTokenCacheTrait},
        },
        user::user_service::UsersService,
    },
    startup_error::StartupError,
    state::{AppState, SharedState},
};
use crate::{api::server, domain::traits::UsersRepositoryTrait};

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
        tracing::info!(
            "Starting api-gateway on: {}:{}",
            self.config.app.service_host,
            self.config.app.service_port
        );
        server::start(self.config, self.state).await
    }
}

pub struct AppBuilder {
    config: Config,
    db_pool: Pool,
    redis: Option<MultiplexedConnection>,
}

impl AppBuilder {
    pub fn builder(config: Config, db_pool: Pool) -> Self {
        Self {
            config,
            db_pool,
            redis: None,
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

    pub async fn build(self) -> Result<App, StartupError> {
        let refresh_token_cache: Arc<dyn RefreshTokenCacheTrait> = match &self.redis {
            Some(conn) => Arc::new(RefreshTokenCache::new(conn.clone())),
            None => Arc::new(NoopRefreshTokenCache),
        };

        let users_repository: Arc<dyn UsersRepositoryTrait> =
            Arc::new(PostgresUsersRepository::new(self.db_pool.clone()));

        let user_service = Arc::new(UsersService::new(users_repository));

        let cfg = self.config.clone();

        let state = Arc::new(AppState {
            db_pool: self.db_pool,
            jwt_decoding_key: cfg.jwt.jwt_keys.decoding,
            user_service: user_service.clone(),
            auth_service: Arc::new(AuthService::new(
                user_service,
                refresh_token_cache,
                cfg.jwt.jwt_expire_access_token_seconds,
                cfg.jwt.jwt_expire_refresh_token_seconds,
                cfg.jwt.jwt_keys.encoding.clone(),
            )),
        });

        Ok(App {
            config: self.config,
            state,
        })
    }
}
