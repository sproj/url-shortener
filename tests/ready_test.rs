pub mod common;
use common::test_app;
use hyper::StatusCode;
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres;
use url_shortener::application::config::{Config, DbConfig};

#[tokio::test]
async fn ready_succeeds_on_db_connectable() {
    let db = postgres::Postgres::default()
        .with_db_name("url_shortener")
        .with_user("admin")
        .with_password("password")
        .with_mapped_port(5432, 5432.into())
        .start()
        .await
        .unwrap();

    let sut = test_app::spawn().await;

    let url = sut.build_path("ready");
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    db.stop().await.unwrap();
}

#[tokio::test]
async fn ready_fails_on_no_database() {
    let sut = test_app::spawn().await;

    let url = sut.build_path("ready");
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn ready_fails_on_invalid_db_config() {
    let db = postgres::Postgres::default()
        .with_db_name("url_shortener")
        .with_user("oh_dear_this_is_wrong")
        .start()
        .await
        .unwrap();

    let db_port = db.get_host_port_ipv4(5432).await.unwrap();

    let cfg = Config {
        service_host: "127.0.0.1".to_string(),
        service_port: 0,
        db: DbConfig {
            postgres_port: db_port,
            postgres_host: "127.0.0.1".into(),
            postgres_user: "admin".into(),
            postgres_password: "password".into(),
            postgres_db: "url_shortener".into(),
            postgres_connection_pool: 5,
        },
    };

    let sut = test_app::spawn_with_config(cfg).await;

    let url = sut.build_path("ready");
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    db.stop().await.unwrap();
}
