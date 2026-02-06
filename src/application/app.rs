use crate::api::server;
use crate::application::config;

pub async fn run() {
    let config = config::load();

    server::start(config).await;
}
