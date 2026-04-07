use reqwest::StatusCode;
use serde_json::json;
use url_shortener::api::handlers::short_url::CreateShortUrlResponse;

use crate::common::{
    constants::{API_PATH_REDIRECT, API_PATH_SHORTEN, API_PATH_SHORTEN_BY_UUID, API_PATH_VANITY},
    helpers::create_user_and_login,
    test_app::TestApp,
};

pub mod common;

fn no_redirect_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

async fn create_owned_vanity_url(
    client: &reqwest::Client,
    sut: &TestApp,
    token: &str,
    code: &str,
    long_url: &str,
) -> CreateShortUrlResponse {
    let res = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(token)
        .json(&json!({
            "long_url": long_url,
            "vanity_url": code,
            "expires_at": null,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::CREATED,
        "setup: create vanity url failed"
    );
    res.json::<CreateShortUrlResponse>().await.unwrap()
}

// --- Auth & access ---

#[tokio::test]
async fn update_requires_auth() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "upd_requires_auth").await;
    let created =
        create_owned_vanity_url(&client, &sut, &token, "upd-auth-test", "http://example.com").await;

    let res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .json(&json!({ "long_url": "http://example.org" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn update_forbidden_for_non_owner() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // User A creates a vanity URL
    let token_a = create_user_and_login(&client, &sut, "upd_owner_a").await;
    let created = create_owned_vanity_url(
        &client,
        &sut,
        &token_a,
        "upd-owner-test",
        "http://example.com",
    )
    .await;

    // User B attempts to update it
    let token_b = create_user_and_login(&client, &sut, "upd_owner_b").await;
    let res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token_b)
        .json(&json!({ "long_url": "http://evil.example.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn update_forbidden_for_unowned_url() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    // Create an anonymous (unowned) short URL — no auth token
    let anonymous_create = client
        .post(sut.build_path(API_PATH_SHORTEN))
        .json(&json!({ "long_url": "http://example.com/anonymous" }))
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous_create.status(), StatusCode::CREATED);
    let anonymous_url = anonymous_create
        .json::<CreateShortUrlResponse>()
        .await
        .unwrap();

    // A logged-in user should still be forbidden from updating it — nobody owns it
    let token = create_user_and_login(&client, &sut, "upd_unowned").await;
    let res = client
        .patch(
            sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, anonymous_url.uuid).as_str()),
        )
        .bearer_auth(&token)
        .json(&json!({ "long_url": "http://example.com/hijacked" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

// --- Happy path ---

#[tokio::test]
async fn update_long_url_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = no_redirect_client();

    let token = create_user_and_login(&client, &sut, "upd_long_url").await;
    let original_url = "http://example.com/original";
    let updated_url = "http://example.com/updated";
    let created =
        create_owned_vanity_url(&client, &sut, &token, "upd-long-url-code", original_url).await;

    // Verify redirect points to the original destination before the update
    let redirect_before = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, created.code).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(redirect_before.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        redirect_before
            .headers()
            .get(reqwest::header::LOCATION)
            .unwrap(),
        original_url
    );

    // Update the long_url, leaving the code unchanged
    let update_res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "long_url": updated_url }))
        .send()
        .await
        .unwrap();
    assert_eq!(update_res.status(), StatusCode::OK);
    let updated = update_res.json::<CreateShortUrlResponse>().await.unwrap();
    assert_eq!(updated.long_url, updated_url);
    assert_eq!(updated.code, created.code, "code should be unchanged");

    // Verify redirect now points to the updated destination
    let redirect_after = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, created.code).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(redirect_after.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        redirect_after
            .headers()
            .get(reqwest::header::LOCATION)
            .unwrap(),
        updated_url
    );
}

#[tokio::test]
async fn update_code_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = no_redirect_client();

    let token = create_user_and_login(&client, &sut, "upd_code").await;
    let long_url = "http://example.com/code-change-target";
    let old_code = "upd-code-old";
    let new_code = "upd-code-new";
    let created = create_owned_vanity_url(&client, &sut, &token, old_code, long_url).await;

    // Update the vanity code
    let update_res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "code": new_code }))
        .send()
        .await
        .unwrap();
    assert_eq!(update_res.status(), StatusCode::OK);
    let updated = update_res.json::<CreateShortUrlResponse>().await.unwrap();
    assert_eq!(updated.code, new_code);

    // The old code should no longer resolve to anything
    let old_redirect = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, old_code).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(old_redirect.status(), StatusCode::NOT_FOUND);

    // The new code should redirect to the same long URL
    let new_redirect = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, new_code).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(new_redirect.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        new_redirect
            .headers()
            .get(reqwest::header::LOCATION)
            .unwrap(),
        long_url
    );
}

#[tokio::test]
async fn update_expires_at_succeeds() {
    let sut = TestApp::builder().build().await;
    let client = no_redirect_client();

    let token = create_user_and_login(&client, &sut, "upd_expires").await;
    let created = create_owned_vanity_url(
        &client,
        &sut,
        &token,
        "upd-expires-code",
        "http://example.com/expiry",
    )
    .await;
    assert!(
        created.expires_at.is_none(),
        "setup: url should start with no expiry"
    );

    // With no expiry the redirect is permanent
    let redirect_before = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, created.code).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(redirect_before.status(), StatusCode::MOVED_PERMANENTLY);

    let future_expiry = chrono::Utc::now() + chrono::Duration::days(7);

    // Set a future expiry
    let update_res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "expires_at": future_expiry }))
        .send()
        .await
        .unwrap();
    assert_eq!(update_res.status(), StatusCode::OK);
    let updated = update_res.json::<CreateShortUrlResponse>().await.unwrap();
    assert!(
        updated.expires_at.is_some(),
        "updated url should have an expiry"
    );

    // With an expiry the redirect is now temporary
    let redirect_after = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, created.code).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(redirect_after.status(), StatusCode::FOUND);
}

#[tokio::test]
async fn clear_expires_at_makes_redirect_permanent() {
    let sut = TestApp::builder().build().await;
    let client = no_redirect_client();

    let token = create_user_and_login(&client, &sut, "upd_clear_expiry").await;
    let future_expiry = chrono::Utc::now() + chrono::Duration::days(7);

    // Create a vanity URL with a future expiry so the redirect starts as temporary
    let create_res = client
        .post(sut.build_path(API_PATH_VANITY))
        .bearer_auth(&token)
        .json(&json!({
            "long_url": "http://example.com/clear-expiry",
            "vanity_url": "upd-clear-expiry",
            "expires_at": future_expiry,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_res.status(), StatusCode::CREATED);
    let created = create_res.json::<CreateShortUrlResponse>().await.unwrap();
    assert!(
        created.expires_at.is_some(),
        "setup: url should have an expiry"
    );

    let redirect_before = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, created.code).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(
        redirect_before.status(),
        StatusCode::FOUND,
        "redirect should be temporary while an expiry is set"
    );

    // Send expires_at: null to explicitly clear the expiry
    let update_res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "expires_at": null }))
        .send()
        .await
        .unwrap();
    assert_eq!(update_res.status(), StatusCode::OK);
    let updated = update_res.json::<CreateShortUrlResponse>().await.unwrap();
    assert!(
        updated.expires_at.is_none(),
        "expiry should have been cleared"
    );

    // Redirect should now be permanent
    let redirect_after = client
        .get(sut.build_path(format!("{}/{}", API_PATH_REDIRECT, created.code).as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(
        redirect_after.status(),
        StatusCode::MOVED_PERMANENTLY,
        "redirect should be permanent after clearing expiry"
    );
}

// --- Validation ---

#[tokio::test]
async fn update_with_empty_body_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "upd_empty_body").await;
    let created = create_owned_vanity_url(
        &client,
        &sut,
        &token,
        "upd-empty-body-code",
        "http://example.com",
    )
    .await;

    let res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err: url_shortener::api::error::ApiError = res.json().await.unwrap();
    assert_eq!(
        err.kind,
        url_shortener::api::error::ApiErrorKind::ValidationError
    );
}

#[tokio::test]
async fn update_with_invalid_long_url_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "upd_bad_long_url").await;
    let created = create_owned_vanity_url(
        &client,
        &sut,
        &token,
        "upd-bad-long-url",
        "http://example.com",
    )
    .await;

    let res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "long_url": "not-a-url" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err: url_shortener::api::error::ApiError = res.json().await.unwrap();
    assert_eq!(
        err.kind,
        url_shortener::api::error::ApiErrorKind::ValidationError
    );
}

#[tokio::test]
async fn update_with_invalid_code_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "upd_bad_code").await;
    let created = create_owned_vanity_url(
        &client,
        &sut,
        &token,
        "upd-bad-code-orig",
        "http://example.com",
    )
    .await;

    let res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "code": "invalid code!" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err: url_shortener::api::error::ApiError = res.json().await.unwrap();
    assert_eq!(
        err.kind,
        url_shortener::api::error::ApiErrorKind::ValidationError
    );
}

#[tokio::test]
async fn update_with_past_expiry_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "upd_past_expiry").await;
    let created = create_owned_vanity_url(
        &client,
        &sut,
        &token,
        "upd-past-expiry-code",
        "http://example.com",
    )
    .await;
    let yesterday = chrono::Utc::now() - chrono::Duration::days(1);

    let res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, created.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "expires_at": yesterday }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let err: url_shortener::api::error::ApiError = res.json().await.unwrap();
    assert_eq!(
        err.kind,
        url_shortener::api::error::ApiErrorKind::ValidationError
    );
}

// --- Conflict ---

#[tokio::test]
async fn update_code_conflict_rejected() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "upd_conflict").await;
    // Create two vanity URLs with distinct codes
    create_owned_vanity_url(
        &client,
        &sut,
        &token,
        "upd-conflict-taken",
        "http://example.com/a",
    )
    .await;
    let second = create_owned_vanity_url(
        &client,
        &sut,
        &token,
        "upd-conflict-second",
        "http://example.com/b",
    )
    .await;

    // Attempt to change the second URL's code to one that is already taken
    let res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN_BY_UUID, second.uuid).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "code": "upd-conflict-taken" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CONFLICT);
}

// --- Not found ---

#[tokio::test]
async fn update_nonexistent_uuid_returns_not_found() {
    let sut = TestApp::builder().build().await;
    let client = reqwest::Client::new();

    let token = create_user_and_login(&client, &sut, "upd_not_found").await;
    // A v7 UUID that will not exist in the database
    let nonexistent = "01966c57-dead-7000-beef-000000000000";

    let res = client
        .patch(sut.build_path(format!("{}/{}", API_PATH_SHORTEN, nonexistent).as_str()))
        .bearer_auth(&token)
        .json(&json!({ "long_url": "http://example.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
