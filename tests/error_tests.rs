pub mod common;
use axum::http::StatusCode;
use common::test_app;

#[tokio::test]
async fn positive_404() {
    let sut = test_app::TestApp::builder().build().await;

    let url = sut.build_path("no_such_path");
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
