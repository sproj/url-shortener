use url_shortener::application::{app, startup_error::StartupError};

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    let config = app::load().await?;
    let state = app::build(&config).await?;

    let migrations_report= app::migrate(&state).await.unwrap();
    println!("{:?}", migrations_report.applied_migrations());

    app::run(config, state).await?;
    Ok(())
}
