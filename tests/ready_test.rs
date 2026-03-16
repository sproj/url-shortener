pub mod common;
use axum::http::StatusCode;
use common::test_app;

use crate::common::{constants::API_PATH_READY, test_db};

#[tokio::test]
async fn ready_succeeds_on_db_connectable() {
    let sut = test_app::TestApp::builder().build().await;

    let url = sut.build_path(API_PATH_READY);
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_fails_on_no_database() {
    let mut cfg = url_shortener::application::config::load().unwrap();
    cfg.db.postgres_port = 1;
    cfg.app.service_port = 0;
    let db = test_db::get_or_create().await;

    let sut = test_app::TestApp::builder()
        .with_db(db)
        .with_config(cfg)
        .build()
        .await;

    let url = sut.build_path(API_PATH_READY);
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn ready_fails_on_invalid_db_config() {
    let mut cfg = url_shortener::application::config::load().unwrap();
    let db = test_db::get_or_create().await;

    // cfg.db.postgres_host = db.postgres_host.clone();
    // cfg.db.postgres_port = db.postgres_port;
    // cfg.db.postgres_db = db.postgres_db.clone();
    // cfg.db.postgres_user = db.postgres_user.clone();
    cfg.db.postgres_password = "invalid".into();
    cfg.app.service_port = 0;

    let sut = test_app::TestApp::builder()
        .with_db(db)
        .with_config(cfg)
        .build()
        .await;

    let url = sut.build_path(API_PATH_READY);
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
