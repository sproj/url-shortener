use crate::common::{constants::API_PATH_SHORTEN, helpers::pick_error_fields, test_app};
use chrono::{Duration, Utc};
use hyper::StatusCode;
use url_shortener::api::{
    error::{ApiError, ApiErrorKind},
    handlers::short_url::CreateShortUrlResponse,
};

pub mod common;

#[tokio::test]
async fn create_shorturl_from_input_succeeds() {
    let sut = test_app::spawn().await;

    test_app::migrate_test_db(&sut.state).await;

    let url = sut.build_path(API_PATH_SHORTEN);

    let client = reqwest::Client::new();

    let expected = "http://create.me".to_string();
    let input = serde_json::json!( {
        "long_url": expected,
        "expires_at": null,
    });

    let res = client.post(url).json(&input).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let actual = res.json::<CreateShortUrlResponse>().await.unwrap();
    assert_eq!(actual.long_url, expected)
}

#[tokio::test]
async fn get_after_create_shorturl_succeeds() {
    let sut = test_app::spawn().await;

    test_app::migrate_test_db(&sut.state).await;

    let create_url = sut.build_path(API_PATH_SHORTEN);
    let client = reqwest::Client::new();

    let expected = "http://read.me";
    let input = serde_json::json!( {
        "long_url": "http://read.me".to_string(),
        "expires_at": null,
    });

    let create = client.post(create_url).json(&input).send().await.unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);
    let created = create.json::<CreateShortUrlResponse>().await.unwrap();

    let get_by_id_url = sut.build_path(format!("{}/{}", API_PATH_SHORTEN, created.id).as_str());

    let read = client.get(get_by_id_url).send().await.unwrap();

    assert_eq!(read.status(), StatusCode::OK);

    let actual = read.json::<CreateShortUrlResponse>().await.unwrap();

    assert_eq!(actual.long_url, expected)
}

#[tokio::test]
async fn empty_long_url_returns_correct_error() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);

    let input = serde_json::json!({
        "long_url": "",
    });

    let actual = client.post(url).json(&input).send().await.unwrap();
    let status = actual.status();
    let err: ApiError = actual.json().await.unwrap();

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err.kind, ApiErrorKind::ValidationError);
    assert!(err.detail.is_some_and(|s| s.to_string().contains("empty")));
}

#[tokio::test]
async fn excessively_long_url_returns_correct_error() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);
    let long_input = "abc".repeat(1000);
    let input = serde_json::json!({
        "long_url": long_input
    });

    let actual = client.post(url).json(&input).send().await.unwrap();
    let status = actual.status();
    let err: ApiError = actual.json().await.unwrap();

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err.kind, ApiErrorKind::ValidationError);
    assert!(err.detail.is_some());
    assert!(
        err.detail
            .is_some_and(|s| s.to_string().contains("too many characters"))
    );
}

#[tokio::test]
async fn expires_at_in_the_past_returns_correct_error() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);
    let yesterday = Utc::now() - Duration::days(1);
    let input = serde_json::json!({
        "long_url": "http://www.valid.com",
        "expires_at": yesterday
    });

    let actual = client.post(url).json(&input).send().await.unwrap();
    let status = actual.status();
    let err: ApiError = actual.json().await.unwrap();

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err.kind, ApiErrorKind::ValidationError);

    assert!(err.detail.is_some());

    assert_eq!(
        pick_error_fields(&err, "invalid_input_url", "code"),
        vec!["in_past"]
    );
}

#[tokio::test]
async fn scheme_must_be_http_or_https() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);
    let input = serde_json::json!({
        "long_url": "ftp://www.valid.com",
    });

    let actual = client.post(url).json(&input).send().await.unwrap();
    let status = actual.status();
    let err: ApiError = actual.json().await.unwrap();

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err.kind, ApiErrorKind::ValidationError);

    assert!(err.detail.is_some());

    assert_eq!(
        pick_error_fields(&err, "invalid_input_url", "code"),
        vec!["scheme"]
    );
}

#[tokio::test]
async fn input_url_must_have_host() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);
    // todo: http:///path-only fails this test because url::Url parses 'path-only' as the host.
    // Might need to add more checks if I don't want `localhost` or IP inputs
    let input = serde_json::json!({
        "long_url": "http://",
    });

    let actual = client.post(url).json(&input).send().await.unwrap();
    let status = actual.status();
    let err: ApiError = actual.json().await.unwrap();

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err.kind, ApiErrorKind::ValidationError);

    assert!(err.detail.unwrap().to_string().contains("host"));
}

#[tokio::test]
async fn input_url_cannot_contain_password() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);
    let input = serde_json::json!({
        "long_url": "https://user:pass@example.com",
    });

    let actual = client.post(url).json(&input).send().await.unwrap();
    let status = actual.status();
    let err: ApiError = actual.json().await.unwrap();

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err.kind, ApiErrorKind::ValidationError);

    assert!(err.detail.is_some());

    assert_eq!(
        pick_error_fields(&err, "invalid_input_url", "code"),
        vec!["password"]
    );
}

#[tokio::test]
async fn delete_shorturl_by_id_succeeds() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let create_url = sut.build_path(API_PATH_SHORTEN);

    let expected = "http://delete.me";
    let input = serde_json::json!( {
        "long_url": expected.to_string(),
        "expires_at": null,
    });

    let create = client.post(create_url).json(&input).send().await.unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);

    let created = create.json::<CreateShortUrlResponse>().await.unwrap();
    let url_with_id_path_param =
        sut.build_path(format!("{}/{}", API_PATH_SHORTEN, created.id).as_str());

    let delete = client.delete(url_with_id_path_param).send().await.unwrap();

    assert_eq!(delete.status(), StatusCode::OK);

    let delete_response = delete.json::<bool>().await.unwrap();
    assert_eq!(delete_response, true);
}

#[tokio::test]
async fn get_shorturl_by_nosuch_id_returns_404() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let no_such_id = -1;
    let url = sut.build_path(format!("{}/{}", API_PATH_SHORTEN, no_such_id).as_str());

    let res = client.get(url).send().await.unwrap();
    let status = res.status();
    let err: ApiError = res.json().await.unwrap();

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(err.kind, ApiErrorKind::ResourceNotFound);
}

#[tokio::test]
async fn delete_shorturl_by_nosuch_id_returns_404() {
    let sut = test_app::spawn().await;
    test_app::migrate_test_db(&sut.state).await;
    let client = reqwest::Client::new();

    let no_such_id = -1;
    let url = sut.build_path(format!("{}/{}", API_PATH_SHORTEN, no_such_id).as_str());

    let res = client.delete(url).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
