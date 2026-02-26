use std::time::Duration;
use core::net::SocketAddr;
use reqwest::StatusCode;
use tokio::time::Instant;
use url_shortener::{
    api::server,
    application::config,
};

use crate::common::{constants, helpers};
pub struct TestApp {
    socket_address: SocketAddr
}

impl TestApp {
    pub fn build_path(&self, path: &str) -> reqwest::Url {
        let url = format!("http://{}/{}", self.socket_address, path);
        dbg!("building url: {}", &url);
        reqwest::Url::parse(&url).unwrap()
    }
}

pub async fn run() -> TestApp {
    let config = config::load().unwrap();
    helpers::CONFIG.get_or_init(|| config.clone());
    
    let listener = server::listen(config).await.unwrap();
    let addr = listener.local_addr().unwrap();

    dbg!(format!("test_app will listen on port: {}", &addr));

    tokio::spawn(server::serve(listener));
    // tokio::spawn(async move { server::serve(listener).await });

    let sut = TestApp {
        socket_address: addr
    };

    let healthz = sut.build_path(constants::API_PATH_HEALTH);
    wait_for_service(Duration::from_secs(5), healthz.as_str()).await;

    return sut
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
