use reqwest::StatusCode;
use serde_json::json;

use crate::common::{
    constants::{API_PATH_LOGIN, API_PATH_USERS},
    test_app::TestApp,
};

mod common;

#[tokio::test]
async fn login_succeeds() {
    let sut = TestApp::builder().build().await;

    let client = reqwest::Client::new();

    let login_url = sut.build_path(API_PATH_LOGIN);
    let create_user_url = sut.build_path(API_PATH_USERS);

    let expected_username = "login_succeeds";
    let expected_email = "login@succeeds.com";
    let password = "log_me_in";

    let create_user_input = json!({
        "username": expected_username,
        "email": expected_email,
        "password": password
    });

    let login_input = json!({
        "username": expected_username,
        "password": password
    });

    let create_user_res = client
        .post(create_user_url)
        .json(&create_user_input)
        .send()
        .await
        .unwrap();

    assert_eq!(create_user_res.status(), StatusCode::CREATED);

    let actual = client
        .post(login_url)
        .json(&login_input)
        .send()
        .await
        .unwrap();

    assert_eq!(actual.status(), StatusCode::OK);

    let body: serde_json::Value = actual.json().await.unwrap();
    let token = body["access_token"].as_str().unwrap();

    assert_eq!(token.split('.').count(), 3);
}

#[tokio::test]
async fn login_rejects_bad_pw() {
    let sut = TestApp::builder().build().await;

    let client = reqwest::Client::new();

    let login_url = sut.build_path(API_PATH_LOGIN);
    let create_user_url = sut.build_path(API_PATH_USERS);

    let expected_username = "login_rejects_bad_pw";
    let expected_email = "login_rejects@bad_pw.com";
    let password = "log_me_in";

    let create_user_input = json!({
        "username": expected_username,
        "email": expected_email,
        "password": password
    });

    let login_input = json!({
        "username": expected_username,
        "password": "do_not_log_me_in"
    });

    let create_user_res = client
        .post(create_user_url)
        .json(&create_user_input)
        .send()
        .await
        .unwrap();

    assert_eq!(create_user_res.status(), StatusCode::CREATED);

    let actual = client
        .post(login_url)
        .json(&login_input)
        .send()
        .await
        .unwrap();

    assert_eq!(actual.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_rejects_unknown_username() {
    let sut = TestApp::builder().build().await;

    let client = reqwest::Client::new();

    let login_url = sut.build_path(API_PATH_LOGIN);
    let create_user_url = sut.build_path(API_PATH_USERS);

    let expected_username = "login_rejects_unknown_username";
    let expected_email = "login_rejects@unknown_username.com";
    let password = "log_me_in";

    let create_user_input = json!({
        "username": expected_username,
        "email": expected_email,
        "password": password
    });

    let login_input = json!({
        "username": "stranger_danger",
        "password": password
    });

    let create_user_res = client
        .post(create_user_url)
        .json(&create_user_input)
        .send()
        .await
        .unwrap();

    assert_eq!(create_user_res.status(), StatusCode::CREATED);

    let actual = client
        .post(login_url)
        .json(&login_input)
        .send()
        .await
        .unwrap();

    assert_eq!(actual.status(), StatusCode::UNAUTHORIZED);
}
