use url_shortener::application::app;

#[tokio::main]
async fn main() {
    app::run().await;
}
