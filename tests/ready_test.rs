pub mod common;
use common::test_app;
use hyper::StatusCode;

use crate::common::{constants::API_PATH_READY, test_app::load_config, test_db};

#[tokio::test]
async fn ready_succeeds_on_db_connectable() {
    let sut = test_app::spawn().await;

    let url = sut.build_path(API_PATH_READY);
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_fails_on_no_database() {
    let db = test_db::get_or_create().await;

    let mut cfg = load_config().await.unwrap();
    cfg.db.postgres_port = 1;

    let sut = test_app::spawn_with_config(cfg, db).await;

    let url = sut.build_path(API_PATH_READY);
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn ready_fails_on_invalid_db_config() {
    let db = test_db::get_or_create().await;

    let mut cfg = load_config().await.unwrap();
    cfg.db.postgres_host = db.host.clone();
    cfg.db.postgres_port = db.port;
    cfg.db.postgres_db = db.db_name.clone();
    cfg.db.postgres_user = db.user.clone();
    cfg.db.postgres_password = "invalid".into();

    let sut = test_app::spawn_with_config(cfg, db).await;

    let url = sut.build_path(API_PATH_READY);
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
