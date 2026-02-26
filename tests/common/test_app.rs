use core::net::SocketAddr;
use reqwest::StatusCode;
use std::time::Duration;
use tokio::time::Instant;
use url_shortener::{
    api::server,
    application::{app::build, config::Config},
};

use crate::common::{constants, helpers};
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
pub async fn spawn_with_config(config: Config) -> TestApp {
    let cfg = config.clone();

    let listener = server::listen(config).await.unwrap();
    let addr = listener.local_addr().unwrap();
    dbg!(format!("test_app will listen on port: {}", &addr));

    let state = build(&cfg).await.unwrap();
    tokio::spawn(server::serve(listener, state));
    // tokio::spawn(async move { server::serve(listener).await });

    let sut = TestApp {
        socket_address: addr,
    };

    let healthz = sut.build_path(constants::API_PATH_HEALTH);
    wait_for_service(Duration::from_secs(5), healthz.as_str()).await;

    return sut;
}

pub async fn spawn() -> TestApp {
    let config = url_shortener::application::config::load().unwrap();
    helpers::CONFIG.get_or_init(|| config.clone());

    spawn_with_config(config).await
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
