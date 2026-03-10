use crate::common::{constants::API_PATH_SHORTEN, helpers::pick_error_fields, test_app};
use axum::http::StatusCode;
use chrono::{Duration, Utc};
use url_shortener::{
    api::{
        error::{ApiError, ApiErrorKind},
        handlers::short_url::CreateShortUrlResponse,
    },
    domain::models::short_url::ShortUrl,
};

pub mod common;

#[tokio::test]
async fn create_shorturl_from_input_succeeds() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;

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
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;

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
async fn get_all_succeeds() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;

    let create_url = sut.build_path(API_PATH_SHORTEN);

    let client = reqwest::Client::new();

    let input = serde_json::json!({
        "long_url": "http://read.me/to",
        "expires_at": null
    });

    let create = client
        .post(create_url.clone())
        .json(&input)
        .send()
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);

    let expected_id = create.json::<CreateShortUrlResponse>().await.unwrap().id;

    let read_all = client.get(create_url).send().await.unwrap();

    assert_eq!(read_all.status(), StatusCode::OK);

    let actual = read_all.json::<Vec<ShortUrl>>().await.unwrap();

    assert!(actual.iter().any(|el| el.id == expected_id));
}

#[tokio::test]
async fn mal_formed_json_payload_returns_expected_error() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);

    let actual = client
        .post(url)
        .header("content-type", "application/json")
        .body(r#"{"long_url":"https://example.com","expires_at": }"#)
        .send()
        .await
        .unwrap();
    let status = actual.status();
    let err = actual.json::<ApiError>().await.unwrap();
    dbg!("{:?}", &err);

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err.kind, ApiErrorKind::UnprocessableInput);
    assert_eq!(
        pick_error_fields(&err, "invalid_input_url", "code"),
        vec!["parse_create_short_url_input_fail"]
    );
}

#[tokio::test]
async fn well_formed_json_but_invalid_create_request_returns_expected_error() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);

    let actual = client
        .post(url)
        .header("content-type", "application/json")
        .body(r#"{"long_url":123,"expires_at":null}"#)
        .send()
        .await
        .unwrap();
    let status = actual.status();
    let err = actual.json::<ApiError>().await.unwrap();
    dbg!("{:?}", &err);

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err.kind, ApiErrorKind::UnprocessableInput);
    assert_eq!(
        pick_error_fields(&err, "invalid_input_url", "code"),
        vec!["parse_create_short_url_input_fail"]
    );
}

#[tokio::test]
async fn empty_long_url_returns_correct_error() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
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
    assert_eq!(
        pick_error_fields(&err, "invalid_input_url", "code"),
        vec!["empty"]
    );
}

#[tokio::test]
async fn excessively_long_url_returns_correct_error() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_SHORTEN);
    let mut long_input = "https://abc".repeat(1000);
    long_input.push_str(".com");

    let input = serde_json::json!({
        "long_url": long_input
    });

    let actual = client.post(url).json(&input).send().await.unwrap();
    let status = actual.status();
    let err: ApiError = actual.json().await.unwrap();
    dbg!("{:?}", &err);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err.kind, ApiErrorKind::ValidationError);
    assert_eq!(
        pick_error_fields(&err, "invalid_input_url", "code"),
        vec!["too_long"]
    );
}

#[tokio::test]
async fn expires_at_in_the_past_returns_correct_error() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
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
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
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
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
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

    assert_eq!(
        pick_error_fields(&err, "invalid_input_url", "code"),
        vec!["parse_url"]
    );
}

#[tokio::test]
async fn input_url_cannot_contain_password() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
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
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
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

    let delete_response = delete.json::<String>().await.unwrap();
    assert_eq!(delete_response, created.id.to_string());
}

#[tokio::test]
async fn get_shorturl_by_nosuch_id_returns_404() {
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
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
    let sut = test_app::TestApp::builder()
        .with_auto_migrate(true)
        .build()
        .await;
    let client = reqwest::Client::new();

    let no_such_id = -1;
    let url = sut.build_path(format!("{}/{}", API_PATH_SHORTEN, no_such_id).as_str());

    let res = client.delete(url).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
