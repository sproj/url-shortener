use std::sync::Arc;

use axum::http::StatusCode;
use url_shortener::{
    api::{
        error::{ApiError, ApiErrorKind},
        handlers::short_url::CreateShortUrlResponse,
    },
    application::{service::short_url::code_generator::FixedCodeGenerator, state::AppStateBuilder},
};
use uuid::Uuid;

use crate::common::{constants::API_PATH_SHORTEN, test_app};

pub mod common;

#[tokio::test]
async fn add_one_retries_on_code_conflict_then_succeeds() {
    let state_builder = AppStateBuilder::default()
        .with_code_generator(Arc::new(FixedCodeGenerator::new(vec![
            "conflict-code".to_string(),
            "recovered-code".to_string(),
        ])))
        .with_max_retries(5);

    let sut = test_app::TestApp::builder()
        .with_state_builder(state_builder)
        .with_auto_migrate(true)
        .build()
        .await;

    seed_existing_short_url_with_code(&sut, "conflict-code").await;

    let url = sut.build_path(API_PATH_SHORTEN);
    let client = reqwest::Client::new();

    let input = serde_json::json!({
        "long_url": "http://retry-path.example",
        "expires_at": null
    });

    let response = client.post(url).json(&input).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let created = response.json::<CreateShortUrlResponse>().await.unwrap();
    assert_eq!(created.code, "recovered-code");
}

#[tokio::test]
async fn add_one_returns_500_when_code_generation_retries_are_exhausted() {
    let state_builder = AppStateBuilder::default()
        .with_code_generator(Arc::new(FixedCodeGenerator::new(
            vec![
                "always-collides",
                "always-collides",
                "always-collides",
                "always-collides",
                "always-collides",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        )))
        .with_max_retries(5);

    let sut = test_app::TestApp::builder()
        .with_state_builder(state_builder)
        .with_auto_migrate(true)
        .build()
        .await;

    seed_existing_short_url_with_code(&sut, "always-collides").await;

    let url = sut.build_path(API_PATH_SHORTEN);
    let client = reqwest::Client::new();

    let input = serde_json::json!({
        "long_url": "http://retry-exhausted.example",
        "expires_at": null
    });

    let response = client.post(url).json(&input).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let err = response.json::<ApiError>().await.unwrap();
    assert_eq!(err.kind, ApiErrorKind::Internal);
    assert_eq!(err.message, "failed to generate a code");
}

async fn seed_existing_short_url_with_code(sut: &test_app::TestApp, code: &str) {
    let client = sut.state.db_pool.get().await.unwrap();

    client
        .execute(
            "INSERT INTO short_url (uuid, code, long_url, expires_at) VALUES ($1, $2, $3, $4)",
            &[
                &Uuid::now_v7(),
                &code,
                &"http://seeded.example",
                &None::<chrono::DateTime<chrono::Utc>>,
            ],
        )
        .await
        .unwrap();
}
