#![allow(dead_code)]
use reqwest::StatusCode;
use serde_json::json;
use url_shortener::api::error::ApiError;

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
