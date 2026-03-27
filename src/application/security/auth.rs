use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString},
};
use jsonwebtoken::EncodingKey;
use rand_core::OsRng;
use uuid::Uuid;

use crate::{
    application::security::{
        auth_error::AuthError,
        jwt::{AccessClaims, JwtTokens},
    },
    domain::models::user::User,
};

pub fn compare_password_hashes(
    true_hash: &str,
    user_input_password: String,
) -> Result<(), AuthError> {
    let parsed_hash = PasswordHash::new(true_hash).map_err(AuthError::HashingError)?;
    Argon2::default()
        .verify_password(user_input_password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::IncorrectCredentials)
}

pub fn generate_salt() -> SaltString {
    SaltString::generate(&mut OsRng)
}

pub fn generate_password_hash(pw: &[u8], salt: &SaltString) -> Result<String, AuthError> {
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(pw, salt)
        .map_err(AuthError::HashingError)?
        .to_string();
    Ok(password_hash)
}

pub fn generate_tokens(
    user: User,
    encoding_key: &EncodingKey,
    expiry_seconds: i64,
) -> Result<JwtTokens, AuthError> {
    let time_now = chrono::Utc::now();
    let iat = time_now.timestamp() as usize;
    let sub = user.uuid.to_string();

    let access_token_id = Uuid::now_v7().to_string();

    let access_token_exp =
        (time_now + chrono::Duration::seconds(expiry_seconds)).timestamp() as usize;

    let access_claims = AccessClaims {
        sub: sub.clone(),
        jti: access_token_id.clone(),
        iat,
        exp: access_token_exp,
        roles: user.roles.clone(),
        aud: "url-shortener".to_string(),
        iss: "url-shortener".to_string(),
    };

    tracing::debug!("JWT: generated claims\naccess {:#?}", access_claims,);

    let access_token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &access_claims,
        encoding_key,
    )
    .map_err(|e| {
        tracing::error!(%e, "failed to encode token");
        AuthError::TokenCreation
    })?;

    tracing::debug!("JWT: generated tokens\naccess success.");

    Ok(JwtTokens { access_token })
}
