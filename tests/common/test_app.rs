use core::net::SocketAddr;
use redis::aio::MultiplexedConnection;
use reqwest::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use url_shortener::{
    api::server,
    application::{
        app::App,
        config::Config,
        state::{AppStateBuilder, SharedState},
    },
};

use crate::common::{
    constants,
    test_db::SharedTestDb,
    test_redis::SharedTestRedis,
};

pub struct TestApp {
    pub state: SharedState,
    socket_address: SocketAddr,
    _db: Arc<SharedTestDb>,
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
    state_builder: AppStateBuilder,
    auto_migrate: bool,
    redis: Option<Arc<SharedTestRedis>>,
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

    pub fn with_state_builder(mut self, state_builder: AppStateBuilder) -> Self {
        self.state_builder = state_builder;
        self
    }

    pub fn with_auto_migrate(mut self, auto_migrate: bool) -> Self {
        self.auto_migrate = auto_migrate;
        self
    }

    pub fn with_redis(mut self, conn: Arc<SharedTestRedis>) -> Self {
        self.redis = Some(conn);
        self
    }

    pub async fn build(self) -> TestApp {
        let db = match self.db {
            Some(db) => db,
            None => test_db::get_or_create().await,
        };

        let config = match self.config {
            Some(config) => config,
            None => Self::default_test_config_for_db(db.as_ref()).await,
        };

        let redis_connection = match self.redis {
            Some(conn) => conn,
            None => test_redis::get_or_create().await,
        };

        let app = App::builder()
            .with_config(config.clone())
            .with_state_builder(self.state_builder)
            .with_redis(redis_connection)
            .with_auto_migrate(false)
            .build()
            .await
            .unwrap();

        let app = if self.auto_migrate {
            app.migrate().await.unwrap()
        } else {
            app
        };

        let state = app.state().clone();

        let listener = server::listen(config).await.unwrap();
        let addr = listener.local_addr().unwrap();
        tracing::info!(%addr, "test app listening");

        tokio::spawn(server::serve(listener, state.clone()));

        let sut = TestApp {
            socket_address: addr,
            _db: db,
            state,
        };

        let healthz = sut.build_path(constants::API_PATH_HEALTH);
        wait_for_service(Duration::from_secs(5), healthz.as_str()).await;

        sut
    }
    async fn default_test_config_for_db(db: &SharedTestDb) -> Config {
        let mut config = url_shortener::application::config::load().unwrap();
        config.db.postgres_host = db.config.postgres_host.clone();
        config.db.postgres_port = db.config.postgres_port.clone();
        config.db.postgres_db = db.config.postgres_db.clone();
        config.db.postgres_user = db.config.postgres_user.clone();
        config.db.postgres_password = db.config.postgres_password.clone();

        // .env.test should include this, but force the issue to avoid accidental fixed-port tests.
        config.app.service_port = 0;
        config
    }
}

impl Default for TestAppBuilder {
    fn default() -> Self {
        Self {
            config: None,
            db: None,
            state_builder: AppStateBuilder::default(),
            auto_migrate: false,
            redis: None,
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
