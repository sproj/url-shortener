#![allow(dead_code)]
use auth::jwt::JwtTokenType;
use chrono::Utc;
use jsonwebtoken::EncodingKey;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use api_gateway::{api::error::ApiError, application::security::claims::AccessClaims};

use crate::common::{
    constants::{API_PATH_LOGIN, API_PATH_USERS},
    test_app::TestApp,
};

pub async fn create_user_and_login(
    client: &reqwest::Client,
    sut: &TestApp,
    username: &str,
) -> String {
    let password = "test_password";
    let create = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": username,
            "email": format!("{}@test.example", username),
            "password": password,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let login = client
        .post(sut.build_path(API_PATH_LOGIN))
        .json(&json!({ "username": username, "password": password }))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);

    let body: serde_json::Value = login.json().await.unwrap();
    body["access_token"].as_str().unwrap().to_string()
}

pub async fn login_as_admin(client: &reqwest::Client, sut: &TestApp) -> String {
    let login = client
        .post(sut.build_path(API_PATH_LOGIN))
        .json(&json!({ "username": "admin", "password": "pass1234" }))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);

    let body: serde_json::Value = login.json().await.unwrap();
    body["access_token"].as_str().unwrap().to_string()
}

/// Mints a signed JWT access token for use in integration tests.
/// Reads JWT_SECRET from the environment (populated from .env.test).
/// Pass `&["user", "admin"]` for admin tokens, `&["user"]` for regular user tokens.
pub fn make_access_token(user_uuid: Uuid, roles: &[&str]) -> String {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET not set in test environment");
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    let now = Utc::now();

    let claims = AccessClaims {
        sub: user_uuid.to_string(),
        aud: "url-shortener".to_string(),
        iss: "api-gateway".to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::seconds(900)).timestamp() as usize,
        jti: Uuid::now_v7().to_string(),
        roles: roles.join(","),
        typ: JwtTokenType::AccessToken as u8,
    };

    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &encoding_key)
        .expect("failed to encode test access token")
}

pub fn pick_error_fields<'a>(
    err: &'a ApiError,
    details_code: &'a str,
    field: &'a str,
) -> Vec<&'a str> {
    err.detail
        .as_ref()
        .and_then(|d| d.get(details_code))
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get(field).and_then(|c| c.as_str()))
                .collect()
        })
        .unwrap_or_default()
}
