use reqwest::StatusCode;
use serde_json::json;
use url_shortener::api::{
    error::{ApiError, ApiErrorKind},
    handlers::users::user_response::UserResponse,
};
use uuid::Uuid;

use crate::common::{constants::API_PATH_USERS, test_app::TestApp};

pub mod common;

#[tokio::test]
async fn create_user_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();
    let url = sut.build_path(API_PATH_USERS);

    let expected_username = "test";
    let expected_email = "test@test.com";

    let expected = json!({
        "username": expected_username,
        "email": expected_email,
        "password": "test"
    });

    let res = client.post(url).json(&expected).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let actual = res.json::<UserResponse>().await.unwrap();
    assert_eq!(actual.username, expected_username);
    assert_eq!(actual.email, expected_email);
    assert_eq!(actual.active, true);
    assert_eq!(actual.roles, "user");
    assert!(actual.deleted_at.is_none());
}

#[tokio::test]
async fn mal_formed_payload_fails() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let url = sut.build_path(API_PATH_USERS);

    let actual = client
        .post(url)
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

#[tokio::test]
async fn get_all_users_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();
    let url = sut.build_path(API_PATH_USERS);

    let expected_username = "test_get_all_users";
    let expected_email = "test_get_all_users@test.com";

    let expected = json!({
        "username": expected_username,
        "email": expected_email,
        "password": "test"
    });

    let create_res = client
        .post(url.clone())
        .json(&expected)
        .send()
        .await
        .unwrap();

    assert_eq!(create_res.status(), StatusCode::CREATED);

    let user = create_res.json::<UserResponse>().await.unwrap();
    let expected_uuid = user.uuid;

    assert_ne!(expected_uuid, Uuid::nil());
    assert_eq!(user.username, expected_username);
    assert_eq!(user.email, expected_email);
    assert_eq!(user.active, true);
    assert_eq!(user.roles, "user");
    assert!(user.deleted_at.is_none());

    let res = client.get(url).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let actual = res.json::<Vec<UserResponse>>().await.unwrap();

    assert!(actual.iter().any(|ur| ur.uuid == expected_uuid));
}

#[tokio::test]
async fn get_user_by_uuid_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();
    let url = sut.build_path(API_PATH_USERS);

    let expected_username = "test_get_user_by_uuid_succeeds";
    let expected_email = "test_get_user_by_uuid_succeeds@test.com";

    let expected = json!({
        "username": expected_username,
        "email": expected_email,
        "password": "test"
    });

    let create_res = client
        .post(url.clone())
        .json(&expected)
        .send()
        .await
        .unwrap();

    assert_eq!(create_res.status(), StatusCode::CREATED);

    let user = create_res.json::<UserResponse>().await.unwrap();
    let expected_uuid = user.uuid;

    let get_by_id_url = sut.build_path(format!("{}/{}", API_PATH_USERS, expected_uuid).as_str());

    let res = client.get(get_by_id_url).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let actual = res.json::<UserResponse>().await.unwrap();

    assert_eq!(actual.username, expected_username);
    assert_eq!(actual.email, expected_email);
    assert_eq!(actual.active, true);
}

#[tokio::test]
async fn get_user_by_uuid_404() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();
    let url = sut.build_path(format!("{}/{}", API_PATH_USERS, Uuid::nil().to_string()).as_str());

    let actual = client.get(url).send().await.unwrap();

    assert_eq!(actual.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_user_by_uuid_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();
    let url = sut.build_path(API_PATH_USERS);

    let expected_username = "test_delete_user_by_uuid_succeeds";
    let expected_email = "test_delete_user_by_uuid_succeeds@test.com";

    let expected = json!({
        "username": expected_username,
        "email": expected_email,
        "password": "test"
    });

    let create_res = client
        .post(url.clone())
        .json(&expected)
        .send()
        .await
        .unwrap();

    assert_eq!(create_res.status(), StatusCode::CREATED);

    let user = create_res.json::<UserResponse>().await.unwrap();
    let expected_uuid = user.uuid;

    let delete_by_id_url = sut.build_path(format!("{}/{}", API_PATH_USERS, expected_uuid).as_str());

    let res = client.delete(delete_by_id_url).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let deleted_uuid = res.json::<String>().await.unwrap();

    assert_eq!(deleted_uuid, expected_uuid.to_string());

    let get_by_id_url = sut.build_path(format!("{}/{}", API_PATH_USERS, expected_uuid).as_str());
    let get_deleted_res = client.get(get_by_id_url).send().await.unwrap();

    assert_eq!(get_deleted_res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_user_by_uuid_404() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();
    let url = sut.build_path(format!("{}/{}", API_PATH_USERS, Uuid::nil().to_string()).as_str());

    let actual = client.delete(url).send().await.unwrap();

    assert_eq!(actual.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_password_200() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();
    let url = sut.build_path(API_PATH_USERS);

    let expected_username = "update_password_200";
    let expected_email = "update_password_200@test.com";

    let expected = json!({
        "username": expected_username,
        "email": expected_email,
        "password": "test"
    });

    let create_res = client
        .post(url.clone())
        .json(&expected)
        .send()
        .await
        .unwrap();

    assert_eq!(create_res.status(), StatusCode::CREATED);

    let user = create_res.json::<UserResponse>().await.unwrap();
    let expected_uuid = user.uuid;

    let update_password_url =
        sut.build_path(format!("{}/{}/password", API_PATH_USERS, expected_uuid).as_str());

    let update_password_payload = json!({"password": "updated"});
    let res = client
        .put(update_password_url)
        .json(&update_password_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn update_password_422() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let update_password_url =
        sut.build_path(format!("{}/{}/password", API_PATH_USERS, Uuid::now_v7()).as_str());

    let update_password_payload = json!({"password": null});
    let res = client
        .put(update_password_url)
        .json(&update_password_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn update_password_404() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();
    let url = sut.build_path(format!("{}/{}/password", API_PATH_USERS, Uuid::nil()).as_str());

    let input = json!({"password": "doesn't matter"});

    let actual = client.put(url).json(&input).send().await.unwrap();

    assert_eq!(actual.status(), StatusCode::NOT_FOUND);
}
