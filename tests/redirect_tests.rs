pub mod common;

use hyper::StatusCode;
use url_shortener::api::handlers::short_url::CreateShortUrlResponse;

use crate::common::{
    constants::{API_PATH_REDIRECT, API_PATH_SHORTEN},
    test_app,
};

#[tokio::test]
async fn permanent_get_redirect_succeeds() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let expected = "http://create.me".to_string();
    let input = serde_json::json!( {
        "long_url": expected,
        "expires_at": null,
    });

    let create_url = sut.build_path(API_PATH_SHORTEN);

    let create = client.post(create_url).json(&input).send().await.unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let short: CreateShortUrlResponse = create.json().await.unwrap();
    let redirect_url = sut.build_path(format!("{}/{}", API_PATH_REDIRECT, &short.code).as_str());

    let actual = client.get(redirect_url).send().await.unwrap();

    assert_eq!(actual.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        actual.headers().get(reqwest::header::LOCATION).unwrap(),
        &expected
    );
}
