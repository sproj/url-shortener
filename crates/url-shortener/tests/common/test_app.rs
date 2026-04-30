#![allow(dead_code)]
use core::net::SocketAddr;
use reqwest::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use url_shortener::{
    api::server,
    application::{
        app::App,
        config::Config,
        service::{
            auth::refresh_token_cache_trait::{NoopRefreshTokenCache, RefreshTokenCacheTrait},
            short_url::{
                code_generator::CodeGenerator,
                redirect_cache::RedirectCacheChecker,
                redirect_cache_trait::{NoopRedirectCache, RedirectCache},
            },
        },
        state::SharedState,
    },
    infrastructure::{database::postgres::Database, redis::connect},
};

use crate::common::{
    constants,
    test_db::{self, SharedTestDb},
    test_redis::SharedTestRedis,
};

pub struct TestApp {
    pub state: SharedState,
    pub redirect_cache: Arc<dyn RedirectCache>,
    pub refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
    socket_address: SocketAddr,
    _db: Arc<SharedTestDb>,
    _redis: Option<Arc<SharedTestRedis>>,
}

impl TestApp {
    pub fn builder() -> TestAppBuilder {
        TestAppBuilder::default()
    }

    pub fn build_path(&self, path: &str) -> reqwest::Url {
        let url = format!("http://{}/{}", self.socket_address, path);
        tracing::info!(%url, "building url");
        reqwest::Url::parse(&url).unwrap()
    }
}

pub struct TestAppBuilder {
    config: Option<Config>,
    db: Option<Arc<SharedTestDb>>,
    redis: Option<Arc<SharedTestRedis>>,
    code_generator: Option<Arc<dyn CodeGenerator>>,
}

impl TestAppBuilder {
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_db(mut self, db: Arc<SharedTestDb>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn with_redis(mut self, redis: Arc<SharedTestRedis>) -> Self {
        self.redis = Some(redis);
        self
    }

    pub fn with_code_generator(mut self, code_generator: Arc<dyn CodeGenerator>) -> Self {
        self.code_generator = Some(code_generator);
        self
    }

    pub async fn build(self) -> TestApp {
        let db = match self.db {
            Some(db) => db,
            None => test_db::get_or_create().await,
        };

        let config = match self.config {
            Some(config) => config,
            None => config_from_db(&db),
        };

        // Always migrate against the real container — config may be deliberately broken in error tests
        let migration_pool = Database::connect(&db.config).unwrap();
        Database::migrate(&migration_pool).await.unwrap();

        let pool = Database::connect(&config.db).unwrap();

        let mut app_builder = App::builder(config.clone(), pool);

        if let Some(code_generator) = self.code_generator {
            app_builder = app_builder.with_code_generator(code_generator);
        }

        // Build test-side handles to the caches so tests can assert against them directly,
        // without needing them on AppState. Both handles share the same Redis backend as the app.
        let (redirect_cache, refresh_token_cache): (
            Arc<dyn RedirectCache>,
            Arc<dyn RefreshTokenCacheTrait>,
        ) = if let Some(redis) = &self.redis {
            let conn = connect::connect(&redis.config).await.unwrap();
            app_builder = app_builder.with_redis(conn.clone());
            (
                Arc::new(RedirectCacheChecker::new(conn.clone())),
                Arc::new(url_shortener::application::service::auth::refresh_token_cache::RefreshTokenCache::new(conn)),
            )
        } else {
            (Arc::new(NoopRedirectCache), Arc::new(NoopRefreshTokenCache))
        };

        let app = app_builder.build().await.unwrap();
        let state = app.state().clone();

        let listener = server::listen(config).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = tracing_subscriber::fmt::try_init();
        tracing::info!(%addr, "test app listening");

        tokio::spawn(server::serve(listener, state.clone()));

        let sut = TestApp {
            socket_address: addr,
            redirect_cache,
            refresh_token_cache,
            _db: db,
            _redis: self.redis,
            state,
        };

        let healthz = sut.build_path(constants::API_PATH_HEALTH);
        wait_for_service(Duration::from_secs(5), healthz.as_str()).await;

        sut
    }
}

fn config_from_db(db: &SharedTestDb) -> Config {
    let mut config = url_shortener::application::config::load().unwrap();
    config.db = db.config.clone();
    config.app.service_port = 0;
    config
}

impl Default for TestAppBuilder {
    fn default() -> Self {
        Self {
            config: None,
            db: None,
            redis: None,
            code_generator: None,
        }
    }
}

async fn wait_for_service(duration: Duration, url: &str) {
    let timeout = Instant::now() + duration;
    loop {
        if let Ok(response) = reqwest::get(url).await
            && response.status() == StatusCode::OK
        {
            break;
        }
        if Instant::now() > timeout {
            panic!("Could not start API Server in: {:?}", duration);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
