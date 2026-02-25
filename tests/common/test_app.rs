use std::{sync::Arc, time::Duration};

use reqwest::StatusCode;
use tokio::time::Instant;
use url_shortener::{
    api::server,
    application::{config, state::AppState},
};

use crate::common::{constants, helpers};

pub async fn run() {
    let config = config::load().unwrap();
    helpers::CONFIG.get_or_init(|| config.clone());
    let app_state = Arc::new(AppState { config });
    let listener = server::listen(&app_state.config).await.unwrap();
    let addr = listener.local_addr().unwrap();

    dbg!(format!("test_app will listen on port: {}", &addr));
    helpers::ADDR.get_or_init(|| format!("http://{}", addr));

    tokio::spawn(server::serve(listener));
    // tokio::spawn(async move { server::serve(listener).await });

    wait_for_service(Duration::from_secs(5)).await
}

async fn wait_for_service(duration: Duration) {
    let timeout = Instant::now() + duration;
    loop {
        let url = helpers::build_path(constants::API_PATH_HEALTH);
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
