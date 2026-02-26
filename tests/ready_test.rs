pub mod common;
use common::test_app;
use hyper::StatusCode;
use url_shortener::application::config::Config;

#[tokio::test]
async fn ready_fails_on_no_database() {
    let sut = test_app::spawn().await;

    let url = sut.build_path("ready");
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn ready_fails_on_invalid_db_config() {
    let invalid_config = Config {
        postgres_host: "127.0.0.1".to_string(),
        postgres_password: "invalid".to_string(),
        postgres_user: "invalid".to_string(),
        postgres_port: 1,
        postgres_db: "invalid".to_string(),
        service_host: "127.0.0.1".to_string(),
        service_port: 0,
        postgres_connection_pool: 5,
    };

    let sut = test_app::spawn_with_config(invalid_config).await;

    let url = sut.build_path("ready");
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
