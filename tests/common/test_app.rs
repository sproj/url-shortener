use core::net::SocketAddr;
use reqwest::StatusCode;
use std::time::Duration;
use tokio::time::Instant;
use url_shortener::{
    api::server,
    application::{app::build, config::Config, startup_error::StartupError},
};

use crate::common::{constants, test_db};
pub struct TestApp {
    socket_address: SocketAddr,
}

impl TestApp {
    pub fn build_path(&self, path: &str) -> reqwest::Url {
        let url = format!("http://{}/{}", self.socket_address, path);
        dbg!("building url: {}", &url);
        reqwest::Url::parse(&url).unwrap()
    }
}

pub async fn spawn() -> TestApp {
    let mut config = load_config().await.unwrap();

    let shared = &*test_db::SHARED_POSTGRES;

    config.db.postgres_host = shared.host.clone();
    config.db.postgres_port = shared.port.clone();
    config.db.postgres_db = shared.db_name.clone();
    config.db.postgres_user = shared.user.clone();
    config.db.postgres_password = shared.password.clone();

    spawn_with_config(config).await
}

pub async fn load_config() -> Result<Config, StartupError> {
    url_shortener::application::config::load()
}

pub async fn spawn_with_config(config: Config) -> TestApp {
    let cfg = config.clone();

    let state = build(&cfg).await.unwrap();

    let listener = server::listen(config).await.unwrap();
    let addr = listener.local_addr().unwrap();
    dbg!(format!("test_app will listen on port: {}", &addr));

    tokio::spawn(server::serve(listener, state));
    // tokio::spawn(async move { server::serve(listener).await });

    let sut = TestApp {
        socket_address: addr,
    };

    let healthz = sut.build_path(constants::API_PATH_HEALTH);
    wait_for_service(Duration::from_secs(5), healthz.as_str()).await;

    return sut;
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
