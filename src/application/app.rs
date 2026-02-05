use crate::api::server;

pub async fn run() {
    server::start().await;
}
