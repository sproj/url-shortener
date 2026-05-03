#![allow(dead_code)]
use auth::jwt::JwtTokenType;
use chrono::Utc;
use jsonwebtoken::EncodingKey;
use uuid::Uuid;

use url_shortener::{api::error::ApiError, application::security::claims::AccessClaims};

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
