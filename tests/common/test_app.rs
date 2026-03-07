use core::net::SocketAddr;
use reqwest::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use url_shortener::{
    api::server,
    application::{
        self,
        app::build_with_state_builder,
        config::Config,
        startup_error::StartupError,
        state::{AppStateBuilder, SharedState},
    },
};

use crate::common::{
    constants,
    test_db::{self, SharedTestDb},
};

pub struct TestApp {
    pub state: SharedState,
    socket_address: SocketAddr,
    _db: Arc<SharedTestDb>,
}

impl TestApp {
    pub fn build_path(&self, path: &str) -> reqwest::Url {
        let url = format!("http://{}/{}", self.socket_address, path);
        dbg!("building url: {}", &url);
        reqwest::Url::parse(&url).unwrap()
    }
}

pub async fn spawn() -> TestApp {
    let db = test_db::get_or_create().await;

    let mut config = load_config().await.unwrap();
    config.db.postgres_host = db.host.clone();
    config.db.postgres_port = db.port;
    config.db.postgres_db = db.db_name.clone();
    config.db.postgres_user = db.user.clone();
    config.db.postgres_password = db.password.clone();

    // hoom hum: .env.test should include this but force the issue so you don't make mistakes such as `ENT_TEST=1 cargo test`.
    config.service_port = 0;

    spawn_with_config(config, db).await
}

pub async fn load_config() -> Result<Config, StartupError> {
    url_shortener::application::config::load()
}

pub async fn spawn_with_config(config: Config, db: Arc<SharedTestDb>) -> TestApp {
    spawn_with_config_and_builder(config, db, AppStateBuilder::default()).await
}

pub async fn spawn_with_config_and_builder(
    config: Config,
    db: Arc<SharedTestDb>,
    state_builder: AppStateBuilder,
) -> TestApp {
    let cfg = config.clone();

    let state = build_with_state_builder(&cfg, state_builder).await.unwrap();

    let listener = server::listen(config).await.unwrap();
    let addr = listener.local_addr().unwrap();
    dbg!(format!("test_app will listen on port: {}", &addr));

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

pub async fn migrate_test_db(state: &Arc<application::state::AppState>) {
    application::app::migrate(state).await.unwrap();
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
