pub mod common;
use common::{helpers, constants::API_PATH_HEALTH, test_app};

#[tokio::test]
async fn health_test() {
    test_app::run().await;

    let url = helpers::build_path(API_PATH_HEALTH);
    let response = reqwest::get(url).await.unwrap();

    let body = response.text().await.unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();

    println!("{:?}", json);

    assert_eq!(json["status"], "healthy");
}
