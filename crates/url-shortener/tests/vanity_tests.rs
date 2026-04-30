use reqwest::StatusCode;
use serde_json::json;
use url_shortener::{
    api::handlers::short_url::CreateShortUrlResponse, domain::models::short_url::ShortUrl,
};

use crate::common::{
    constants::{API_PATH_REDIRECT, API_PATH_SHORTEN_BY_UUID, API_PATH_VANITY},
    helpers::create_user_and_login,
    test_app::TestApp,
};

pub mod common;

#[tokio::test]
async fn create_vanity_url_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "vanity_create_succeeds").await;
    let vanity_code = "my-custom-code";

    let res = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": "http://example.com",
            "vanity_url": vanity_code,
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let body = res.json::<CreateShortUrlResponse>().await.unwrap();
    assert_eq!(body.code, vanity_code);
    assert_eq!(body.long_url, "http://example.com");
}

#[tokio::test]
async fn vanity_url_redirects() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let token = create_user_and_login(&client, &sut, "vanity_redirects").await;
    let vanity_code = "redirect-me";
    let long_url = "http://example.com/destination";

    let create = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": long_url,
            "vanity_url": vanity_code,
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let redirect = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, vanity_code).as_str()))
        .send()
        .await
        .unwrap();

    assert_eq!(redirect.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        redirect.headers().get(reqwest::header::LOCATION).unwrap(),
        long_url
    );
}

#[tokio::test]
async fn create_vanity_url_requires_auth() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let res = client
        .post(sut.build_path(API_PATH_VANITY))
        .json(&json!({
            "long_url": "http://example.com",
            "vanity_url": "some-code",
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn duplicate_vanity_code_is_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "vanity_duplicate_code").await;
    let input = json!({
        "long_url": "http://example.com",
        "vanity_url": "duplicate-this",
        "expires_at": null,
    });

    let first = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::CREATED);

    let second = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&input)
        .send()
        .await
        .unwrap();

    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn create_vanity_url_rejects_invalid_long_url() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "vanity_invalid_long_url").await;

    let res = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": "not-a-url",
            "vanity_url": "valid-code",
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_vanity_url_rejects_expired_expires_at() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "vanity_past_expiry").await;
    let yesterday = chrono::Utc::now() - chrono::Duration::days(1);

    let res = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": "http://example.com",
            "vanity_url": "expires-yesterday",
            "expires_at": yesterday,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn vanity_url_is_associated_with_creating_user() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "vanity_user_association").await;

    let create = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": "http://example.com",
            "vanity_url": "owned-by-user",
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let created = create.json::<CreateShortUrlResponse>().await.unwrap();

    let get = client
        .get(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);

    let record = get.json::<ShortUrl>().await.unwrap();
    dbg!(&record);
    assert!(
        record.user_id.is_some(),
        "vanity url should have an associated user_id"
    );
}

#[tokio::test]
async fn vanity_code_with_invalid_characters_is_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "vanity_invalid_chars").await;

    let res = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": "http://example.com",
            "vanity_url": "invalid code!",
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err: url_shortener::api::error::ApiError = res.json().await.unwrap();
    assert_eq!(
        err.kind,
        url_shortener::api::error::ApiErrorKind::ValidationError
    );
}

#[tokio::test]
async fn vanity_code_exceeding_max_length_is_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "vanity_too_long_code").await;
    let long_code = "a".repeat(65);

    let res = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": "http://example.com",
            "vanity_url": long_code,
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err: url_shortener::api::error::ApiError = res.json().await.unwrap();
    assert_eq!(
        err.kind,
        url_shortener::api::error::ApiErrorKind::ValidationError
    );
}

#[tokio::test]
async fn empty_vanity_code_is_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "vanity_empty_code").await;

    let res = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": "http://example.com",
            "vanity_url": "",
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err: url_shortener::api::error::ApiError = res.json().await.unwrap();
    assert_eq!(
        err.kind,
        url_shortener::api::error::ApiErrorKind::ValidationError
    );
}
