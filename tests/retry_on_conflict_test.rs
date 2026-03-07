use std::sync::Arc;

use hyper::StatusCode;
use url_shortener::{
    api::handlers::short_url::CreateShortUrlResponse,
    application::{service::short_url::code_generator::FixedCodeGenerator, state::AppStateBuilder},
};
use uuid::Uuid;

use crate::common::{constants::API_PATH_SHORTEN, test_app, test_db};

pub mod common;

#[tokio::test]
async fn add_one_retries_on_code_conflict_then_succeeds() {
    let sut = spawn_with_fixed_codes(vec!["conflict-code", "recovered-code"], 5).await;

    test_app::migrate_test_db(&sut.state).await;
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

async fn spawn_with_fixed_codes(codes: Vec<&str>, max_retries: u8) -> test_app::TestApp {
    let db = test_db::get_or_create().await;

    let mut config = test_app::load_config().await.unwrap();
    config.db.postgres_host = db.host.clone();
    config.db.postgres_port = db.port;
    config.db.postgres_db = db.db_name.clone();
    config.db.postgres_user = db.user.clone();
    config.db.postgres_password = db.password.clone();
    config.service_port = 0;

    let state_builder = AppStateBuilder::default()
        .with_code_generator(Arc::new(FixedCodeGenerator::new(
            codes.into_iter().map(str::to_owned).collect(),
        )))
        .with_max_retries(max_retries);

    test_app::spawn_with_config_and_builder(config, db, state_builder).await
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
