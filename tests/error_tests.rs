pub mod common;
use common::{helpers, test_app};
use hyper::StatusCode;

#[tokio::test]
async fn positive_404() {
    test_app::run().await;

    let url = helpers::build_path("no_such_path");
    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
