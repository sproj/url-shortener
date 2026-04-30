use reqwest::StatusCode;
use serde_json::json;
use url_shortener::api::{
    error::{ApiError, ApiErrorKind},
    handlers::users::user_response::UserResponse,
};
use uuid::Uuid;

use crate::common::{
    constants::API_PATH_USERS,
    helpers::{create_user_and_login, login_as_admin},
    test_app::TestApp,
};

pub mod common;

// --- Unauthenticated (public) operations ---

#[tokio::test]
async fn create_user_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "create_user_succeeds",
            "email": "create_user_succeeds@test.com",
            "password": "test"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let actual = res.json::<UserResponse>().await.unwrap();
    assert_eq!(actual.username, "create_user_succeeds");
    assert_eq!(actual.email, "create_user_succeeds@test.com");
    assert_eq!(actual.active, true);
    assert_eq!(actual.roles, "user");
    assert!(actual.deleted_at.is_none());
}

#[tokio::test]
async fn mal_formed_payload_fails() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let actual = client
        .post(sut.build_path(API_PATH_USERS))
        .header("content-type", "application/json")
        .body(r#"{"username":"durkadurr","email": durk@durr.com, password: }"#)
        .send()
        .await
        .unwrap();
    let status = actual.status();
    let err = actual.json::<ApiError>().await.unwrap();

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err.kind, ApiErrorKind::UnprocessableInput);
}

// --- get_all ---

#[tokio::test]
async fn get_all_requires_auth() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let res = client
        .get(sut.build_path(API_PATH_USERS))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_all_requires_admin() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "get_all_non_admin").await;

    let res = client
        .get(sut.build_path(API_PATH_USERS))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_all_users_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // Create a regular user to confirm they appear in the list
    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "get_all_list_member",
            "email": "get_all_list_member@test.com",
            "password": "test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let user = create_res.json::<UserResponse>().await.unwrap();

    let token = login_as_admin(&client, &sut).await;

    let res = client
        .get(sut.build_path(API_PATH_USERS))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let actual = res.json::<Vec<UserResponse>>().await.unwrap();
    assert!(actual.iter().any(|ur| ur.uuid == user.uuid));
}

// --- get_one ---

#[tokio::test]
async fn get_one_requires_auth() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "get_one_no_auth",
            "email": "get_one_no_auth@test.com",
            "password": "test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let user = create_res.json::<UserResponse>().await.unwrap();

    let res = client
        .get(sut.build_path(format!("{}/{}", API_PATH_USERS, user.uuid).as_str()))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_one_forbidden_for_other_user() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // Create user A — the target
    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "get_one_target",
            "email": "get_one_target@test.com",
            "password": "test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let target = create_res.json::<UserResponse>().await.unwrap();

    // User B attempts to read user A's record
    let token_b = create_user_and_login(&client, &sut, "get_one_intruder").await;

    let res = client
        .get(sut.build_path(format!("{}/{}", API_PATH_USERS, target.uuid).as_str()))
        .bearer_auth(&token_b)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn get_user_by_uuid_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // The helper doesn't return the UUID, so we do the create+login steps manually
    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "get_by_uuid_self_fetch",
            "email": "get_by_uuid_self_fetch@test.com",
            "password": "test_password"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let user = create_res.json::<UserResponse>().await.unwrap();

    let login_res = client
        .post(sut.build_path(crate::common::constants::API_PATH_LOGIN))
        .json(&json!({ "username": "get_by_uuid_self_fetch", "password": "test_password" }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let body: serde_json::Value = login_res.json().await.unwrap();
    let own_token = body["access_token"].as_str().unwrap().to_string();

    let res = client
        .get(sut.build_path(format!("{}/{}", API_PATH_USERS, user.uuid).as_str()))
        .bearer_auth(&own_token)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let actual = res.json::<UserResponse>().await.unwrap();
    assert_eq!(actual.username, "get_by_uuid_self_fetch");
    assert_eq!(actual.email, "get_by_uuid_self_fetch@test.com");
    assert_eq!(actual.active, true);
}

#[tokio::test]
async fn get_user_by_uuid_404() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = login_as_admin(&client, &sut).await;

    let res = client
        .get(sut.build_path(format!("{}/{}", API_PATH_USERS, Uuid::nil()).as_str()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// --- delete ---

#[tokio::test]
async fn delete_requires_auth() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "delete_no_auth",
            "email": "delete_no_auth@test.com",
            "password": "test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let user = create_res.json::<UserResponse>().await.unwrap();

    let res = client
        .delete(sut.build_path(format!("{}/{}", API_PATH_USERS, user.uuid).as_str()))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_forbidden_for_other_user() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // Create user A — the target
    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "delete_target",
            "email": "delete_target@test.com",
            "password": "test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let target = create_res.json::<UserResponse>().await.unwrap();

    // User B attempts to delete user A
    let token_b = create_user_and_login(&client, &sut, "delete_intruder").await;

    let res = client
        .delete(sut.build_path(format!("{}/{}", API_PATH_USERS, target.uuid).as_str()))
        .bearer_auth(&token_b)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn delete_user_by_uuid_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // Create the user and log in as them
    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "delete_self",
            "email": "delete_self@test.com",
            "password": "test_password"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let user = create_res.json::<UserResponse>().await.unwrap();

    let login_res = client
        .post(sut.build_path(crate::common::constants::API_PATH_LOGIN))
        .json(&json!({ "username": "delete_self", "password": "test_password" }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let body: serde_json::Value = login_res.json().await.unwrap();
    let token = body["access_token"].as_str().unwrap().to_string();

    // Delete own account
    let delete_res = client
        .delete(sut.build_path(format!("{}/{}", API_PATH_USERS, user.uuid).as_str()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(delete_res.status(), StatusCode::OK);
    let deleted_uuid = delete_res.json::<String>().await.unwrap();
    assert_eq!(deleted_uuid, user.uuid.to_string());

    // Token is still valid (JWT not revoked), but the record is soft-deleted — expect 404
    let get_res = client
        .get(sut.build_path(format!("{}/{}", API_PATH_USERS, user.uuid).as_str()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(get_res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_user_by_uuid_404() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = login_as_admin(&client, &sut).await;

    let res = client
        .delete(sut.build_path(format!("{}/{}", API_PATH_USERS, Uuid::nil()).as_str()))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// --- update_password ---

#[tokio::test]
async fn update_password_requires_auth() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "update_pw_no_auth",
            "email": "update_pw_no_auth@test.com",
            "password": "test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let user = create_res.json::<UserResponse>().await.unwrap();

    let res = client
        .put(sut.build_path(format!("{}/{}/password", API_PATH_USERS, user.uuid).as_str()))
        .json(&json!({ "password": "new_password" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn update_password_forbidden_for_other_user() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // Create user A — the target
    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "update_pw_target",
            "email": "update_pw_target@test.com",
            "password": "test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let target = create_res.json::<UserResponse>().await.unwrap();

    // User B attempts to change user A's password
    let token_b = create_user_and_login(&client, &sut, "update_pw_intruder").await;

    let res = client
        .put(sut.build_path(format!("{}/{}/password", API_PATH_USERS, target.uuid).as_str()))
        .bearer_auth(&token_b)
        .json(&json!({ "password": "hacked" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn update_password_200() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "update_pw_self",
            "email": "update_pw_self@test.com",
            "password": "test_password"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let user = create_res.json::<UserResponse>().await.unwrap();

    let login_res = client
        .post(sut.build_path(crate::common::constants::API_PATH_LOGIN))
        .json(&json!({ "username": "update_pw_self", "password": "test_password" }))
        .send()
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let body: serde_json::Value = login_res.json().await.unwrap();
    let token = body["access_token"].as_str().unwrap().to_string();

    let res = client
        .put(sut.build_path(format!("{}/{}/password", API_PATH_USERS, user.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "password": "updated" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn update_password_422() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // Auth is required before payload is parsed, so we need a valid token
    let create_res = client
        .post(sut.build_path(API_PATH_USERS))
        .json(&json!({
            "username": "update_pw_422",
            "email": "update_pw_422@test.com",
            "password": "test_password"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let user = create_res.json::<UserResponse>().await.unwrap();

    let login_res = client
        .post(sut.build_path(crate::common::constants::API_PATH_LOGIN))
        .json(&json!({ "username": "update_pw_422", "password": "test_password" }))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = login_res.json().await.unwrap();
    let token = body["access_token"].as_str().unwrap().to_string();

    let res = client
        .put(sut.build_path(format!("{}/{}/password", API_PATH_USERS, user.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "password": null }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn update_password_404() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = login_as_admin(&client, &sut).await;

    let res = client
        .put(sut.build_path(format!("{}/{}/password", API_PATH_USERS, Uuid::nil()).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "password": "doesn't matter" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
