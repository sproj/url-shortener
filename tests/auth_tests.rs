use reqwest::StatusCode;
use serde_json::json;
use url_shortener::{
    api::handlers::users::user_response::UserResponse,
    application::security::jwt::{AccessClaims, JwtTokenType, RefreshClaims, decode_token},
};

use crate::common::{
    constants::{API_PATH_LOGIN, API_PATH_USERS},
    test_app::TestApp,
    test_redis,
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
async fn login_returns_two_tokens() {
    let sut = TestApp::builder().build().await;

    let client = reqwest::Client::new();

    let login_url = sut.build_path(API_PATH_LOGIN);
    let create_user_url = sut.build_path(API_PATH_USERS);

    let expected_username = "login_returns_two_tokens";
    let expected_email = "login_returns@two_tokens.com";
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
    let access_token = body["access_token"].as_str().unwrap();
    let refresh_token = body["refresh_token"].as_str().unwrap();

    assert_eq!(access_token.split('.').count(), 3);
    assert_eq!(refresh_token.split('.').count(), 3);
}

#[tokio::test]
async fn login_returns_expected_tokens() {
    let sut = TestApp::builder().build().await;

    let client = reqwest::Client::new();

    let login_url = sut.build_path(API_PATH_LOGIN);
    let create_user_url = sut.build_path(API_PATH_USERS);

    let expected_username = "login_returns_expected_tokens";
    let expected_email = "login_returns@expected_tokens.com";
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
    let access_token_res = body["access_token"].as_str().unwrap();
    let refresh_token_res = body["refresh_token"].as_str().unwrap();

    assert_eq!(access_token_res.split('.').count(), 3);
    assert_eq!(refresh_token_res.split('.').count(), 3);

    let access_token: AccessClaims =
        decode_token(access_token_res, &sut.state.jwt_decoding_key).unwrap();
    let refresh_token: RefreshClaims =
        decode_token(refresh_token_res, &sut.state.jwt_decoding_key).unwrap();

    let created_user_response: UserResponse = create_user_res.json().await.unwrap();

    let actual_sub = uuid::Uuid::parse_str(&access_token.sub).unwrap();

    assert_eq!(actual_sub, created_user_response.uuid);
    assert_eq!(access_token.jti, refresh_token.prf);
    assert_eq!(access_token.sub, refresh_token.sub);
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

#[tokio::test]
async fn login_caches_refresh_token() {
    let redis = test_redis::get_or_create().await;
    let sut = TestApp::builder().with_redis(redis.clone()).build().await;

    let client = reqwest::Client::new();

    let login_url = sut.build_path(API_PATH_LOGIN);
    let create_user_url = sut.build_path(API_PATH_USERS);

    let expected_username = "login_caches_refresh_token";
    let expected_email = "login_caches@refresh_token.com";
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
    let access_token_res = body["access_token"].as_str().unwrap();
    let refresh_token_res = body["refresh_token"].as_str().unwrap();

    assert_eq!(access_token_res.split('.').count(), 3);
    assert_eq!(refresh_token_res.split('.').count(), 3);

    let access_token: AccessClaims =
        decode_token(access_token_res, &sut.state.jwt_decoding_key).unwrap();
    let refresh_token: RefreshClaims =
        decode_token(refresh_token_res, &sut.state.jwt_decoding_key).unwrap();

    let expected_cache_key = access_token.jti;

    let cached_refresh_claims = sut
        .state
        .refresh_token_cache
        .get(&expected_cache_key)
        .await
        .unwrap();

    assert!(cached_refresh_claims.is_some());

    let actual = cached_refresh_claims.unwrap();
    assert_eq!(actual.prf, expected_cache_key);
    assert_eq!(actual.sub, refresh_token.sub);
    assert_eq!(actual.jti, refresh_token.jti);
    assert_eq!(actual.exp, refresh_token.exp);
    assert_eq!(actual.pex, access_token.exp);
    assert_eq!(actual.typ, JwtTokenType::RefreshToken as u8);
}
