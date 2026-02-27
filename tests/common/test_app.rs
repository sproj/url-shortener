use core::net::SocketAddr;
use reqwest::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use url_shortener::{
    api::server,
    application::{app::build, config::Config, startup_error::StartupError},
};

use crate::common::{
    constants,
    test_db::{self, SharedTestDb},
};

pub struct TestApp {
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

    spawn_with_config(config, db).await
}

pub async fn load_config() -> Result<Config, StartupError> {
    url_shortener::application::config::load()
}

pub async fn spawn_with_config(config: Config, db: Arc<SharedTestDb>) -> TestApp {
    let cfg = config.clone();

    let state = build(&cfg).await.unwrap();

    let listener = server::listen(config).await.unwrap();
    let addr = listener.local_addr().unwrap();
    dbg!(format!("test_app will listen on port: {}", &addr));

    tokio::spawn(server::serve(listener, state));

    let sut = TestApp {
        socket_address: addr,
        _db: db,
    };

    let healthz = sut.build_path(constants::API_PATH_HEALTH);
    wait_for_service(Duration::from_secs(5), healthz.as_str()).await;

    sut
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
