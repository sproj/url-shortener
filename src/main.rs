use url_shortener::application::{app::App, startup_error::StartupError};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    App::builder().build().await?.start().await
}
