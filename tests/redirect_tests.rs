pub mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use redis::AsyncTypedCommands;
use url_shortener::api::handlers::short_url::CreateShortUrlResponse;
use url_shortener::infrastructure::redis::connect;
use uuid::Uuid;

use crate::common::{
    constants::{API_PATH_REDIRECT, API_PATH_SHORTEN},
    test_app, test_redis,
};

#[tokio::test]
async fn permanent_get_redirect_succeeds() {
    let sut = test_app::TestApp::builder().build().await;
    let client = no_redirect_client();

    let expected = "http://create.me".to_string();
    let input = serde_json::json!( {
        "long_url": expected,
        "expires_at": null,
    });

    let create = client
        .post(sut.build_path(API_PATH_SHORTEN))
        .json(&input)
        .send()
        .await
        .unwrap();
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

#[tokio::test]
async fn permanent_non_get_redirect_succeeds() {
    let sut = test_app::TestApp::builder().build().await;
    let client = no_redirect_client();

    let short = create_short_url(&client, &sut, "http://permanent-post.me", None).await;
    let redirect_url = sut.build_path(format!("{}/{}", API_PATH_REDIRECT, &short.code).as_str());

    let actual = client.post(redirect_url).send().await.unwrap();

    assert_eq!(actual.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        actual.headers().get(reqwest::header::LOCATION).unwrap(),
        "http://permanent-post.me"
    );
}

#[tokio::test]
async fn temporary_get_redirect_succeeds() {
    let sut = test_app::TestApp::builder().build().await;
    let client = no_redirect_client();

    let future = Utc::now() + Duration::days(1);
    let short = create_short_url(&client, &sut, "http://temporary-get.me", Some(future)).await;
    let redirect_url = sut.build_path(format!("{}/{}", API_PATH_REDIRECT, &short.code).as_str());

    let actual = client.get(redirect_url).send().await.unwrap();

    assert_eq!(actual.status(), StatusCode::FOUND);
    assert_eq!(
        actual.headers().get(reqwest::header::LOCATION).unwrap(),
        "http://temporary-get.me"
    );
}

#[tokio::test]
async fn temporary_non_get_redirect_succeeds() {
    let sut = test_app::TestApp::builder().build().await;
    let client = no_redirect_client();

    let future = Utc::now() + Duration::days(1);
    let short = create_short_url(&client, &sut, "http://temporary-non-get.me", Some(future)).await;
    let redirect_url = sut.build_path(format!("{}/{}", API_PATH_REDIRECT, &short.code).as_str());

    let actual = client.post(redirect_url).send().await.unwrap();

    assert_eq!(actual.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        actual.headers().get(reqwest::header::LOCATION).unwrap(),
        "http://temporary-non-get.me"
    );
}

#[tokio::test]
async fn missing_code_returns_404() {
    let sut = test_app::TestApp::builder().build().await;
    let client = no_redirect_client();

    let actual = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, "no-such-code").as_str()))
        .send()
        .await
        .unwrap();

    assert_eq!(actual.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn expired_code_returns_410() {
    let sut = test_app::TestApp::builder().build().await;
    let client = no_redirect_client();

    let code = "expired-code";
    seed_short_url_record(
        &sut,
        code,
        "http://expired.me",
        Some(Utc::now() - Duration::days(1)),
        None,
        None,
    )
    .await;

    let actual = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, code).as_str()))
        .send()
        .await
        .unwrap();

    assert_eq!(actual.status(), StatusCode::GONE);
}

#[tokio::test]
async fn deleted_code_returns_410() {
    let sut = test_app::TestApp::builder().build().await;
    let client = no_redirect_client();

    let code = "deleted-code";
    seed_short_url_record(
        &sut,
        code,
        "http://deleted.me",
        None,
        Some(Utc::now() - Duration::minutes(1)),
        None,
    )
    .await;

    let actual = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, code).as_str()))
        .send()
        .await
        .unwrap();

    assert_eq!(actual.status(), StatusCode::GONE);
}

#[tokio::test]
async fn redirect_uses_cached_decision_when_db_row_is_gone() {
    let redis = test_redis::get_or_create().await;
    let sut = test_app::TestApp::builder()
        .with_redis(redis.clone())
        .build()
        .await;
    let client = no_redirect_client();

    let short = create_short_url(&client, &sut, "http://cached-redirect.me", None).await;
    let redirect_url = sut.build_path(format!("{}/{}", API_PATH_REDIRECT, &short.code).as_str());

    let first = client.get(redirect_url.clone()).send().await.unwrap();
    assert_eq!(first.status(), StatusCode::MOVED_PERMANENTLY);

    let db = sut.state.db_pool.get().await.unwrap();
    db.execute("DELETE FROM short_url WHERE code = $1", &[&short.code])
        .await
        .unwrap();

    let second = client.get(redirect_url).send().await.unwrap();
    assert_eq!(second.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        second.headers().get(reqwest::header::LOCATION).unwrap(),
        "http://cached-redirect.me"
    );
}

#[tokio::test]
async fn deleting_short_url_invalidates_cached_redirect() {
    let redis = test_redis::get_or_create().await;
    let sut = test_app::TestApp::builder()
        .with_redis(redis.clone())
        .build()
        .await;
    let client = no_redirect_client();

    let short = create_short_url(&client, &sut, "http://delete-invalidates-cache.me", None).await;
    let redirect_url = sut.build_path(format!("{}/{}", API_PATH_REDIRECT, &short.code).as_str());

    let first = client.get(redirect_url.clone()).send().await.unwrap();
    assert_eq!(first.status(), StatusCode::MOVED_PERMANENTLY);

    let cached_before = redis_get(&redis, &short.code).await;
    assert!(cached_before.is_some());

    let delete = client
        .delete(sut.build_path(format!("{}/{}", API_PATH_SHORTEN, short.uuid).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);

    let cached_after = redis_get(&redis, &short.code).await;
    assert!(cached_after.is_none());

    let second = client.get(redirect_url).send().await.unwrap();
    assert_eq!(second.status(), StatusCode::GONE);
}

fn no_redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

async fn create_short_url(
    client: &reqwest::Client,
    sut: &test_app::TestApp,
    long_url: &str,
    expires_at: Option<chrono::DateTime<Utc>>,
) -> CreateShortUrlResponse {
    let input = serde_json::json!({
        "long_url": long_url,
        "expires_at": expires_at,
    });

    let create = client
        .post(sut.build_path(API_PATH_SHORTEN))
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    create.json().await.unwrap()
}

async fn seed_short_url_record(
    sut: &test_app::TestApp,
    code: &str,
    long_url: &str,
    expires_at: Option<chrono::DateTime<Utc>>,
    deleted_at: Option<chrono::DateTime<Utc>>,
    user_id: Option<i64>,
) {
    let client = sut.state.db_pool.get().await.unwrap();

    client
        .execute(
            "INSERT INTO short_url (uuid, code, long_url, expires_at, deleted_at, user_id) VALUES ($1, $2, $3, $4, $5, $6)",
            &[&Uuid::now_v7(), &code, &long_url, &expires_at, &deleted_at, &user_id],
        )
        .await
        .unwrap();
}

async fn redis_get(redis: &test_redis::SharedTestRedis, code: &str) -> Option<String> {
    let mut conn = connect::connect(&redis.config).await.unwrap();
    conn.get(code).await.unwrap()
}
