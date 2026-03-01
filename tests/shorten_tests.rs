use crate::common::{constants::API_PATH_SHORTEN, test_app};
use hyper::StatusCode;
use url_shortener::domain::models::short_url::ShortUrl;

pub mod common;

#[tokio::test]
async fn create_short_from_input_succeeds() {
    let sut = test_app::spawn().await;

    test_app::migrate_test_db(&sut.state).await;

    let url = sut.build_path(API_PATH_SHORTEN);

    let client = reqwest::Client::new();

    let res = client
        .post(url)
        .json("http://create_this_you_casual.com")
        // .header("content-type", "application/json")
        // .body("http://test.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn get_after_create_succeeds() {
    let sut = test_app::spawn().await;

    test_app::migrate_test_db(&sut.state).await;

    let url = sut.build_path(API_PATH_SHORTEN);

    let client = reqwest::Client::new();

    let expected = "http://read.me";

    let create = client
        .post(url.clone())
        .json(&expected)
        // .header("content-type", "application/json")
        // .body("http://test.com")
        .send()
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);

    let read = client.get(url).send().await.unwrap();

    assert_eq!(read.status(), StatusCode::OK);

    let short_result = read.json::<Vec<ShortUrl>>().await.unwrap();

    assert!(short_result.iter().any(|short| short.long_url == expected))
}
