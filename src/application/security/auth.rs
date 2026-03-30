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
        jwt::{AccessClaims, JwtTokenType, JwtTokens, RefreshClaims},
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
    jwt_encoding_key: EncodingKey,
    access_token_expiry_seconds: i64,
    refresh_token_expiry_seconds: i64,
    user: User,
) -> Result<JwtTokens, AuthError> {
    let time_now = chrono::Utc::now();
    let iat = time_now.timestamp() as usize;
    let sub = user.uuid.to_string();

    let access_token_id = Uuid::now_v7().to_string();
    let refresh_token_id = Uuid::now_v7().to_string();

    let access_token_exp =
        (time_now + chrono::Duration::seconds(access_token_expiry_seconds)).timestamp() as usize;
    let refresh_token_exp =
        (time_now + chrono::Duration::seconds(refresh_token_expiry_seconds)).timestamp() as usize;

    let access_claims = AccessClaims {
        sub: sub.clone(),
        jti: access_token_id.clone(),
        iat,
        exp: access_token_exp,
        roles: user.roles.clone(),
        aud: "url-shortener".to_string(),
        iss: "url-shortener".to_string(),
        typ: JwtTokenType::AccessToken as u8,
    };

    let refresh_claims: RefreshClaims = RefreshClaims {
        sub,
        jti: refresh_token_id,
        iat,
        exp: refresh_token_exp,
        prf: access_token_id,
        pex: access_token_exp,
        typ: JwtTokenType::RefreshToken as u8,
        roles: user.roles,
    };

    tracing::debug!("JWT: generated claims\naccess {:#?}", access_claims,);

    let access_token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &access_claims,
        &jwt_encoding_key,
    )
    .map_err(|e| {
        tracing::error!(%e, "failed to encode access token");
        AuthError::TokenCreation
    })?;

    let refresh_token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &refresh_claims,
        &jwt_encoding_key,
    )
    .map_err(|e| {
        tracing::error!(%e, "failed to encode refresh token");
        AuthError::TokenCreation
    })?;

    tracing::debug!("JWT: generated tokens\naccess success.");

    Ok(JwtTokens {
        access_token,
        refresh_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::security::auth_error::AuthError;

    #[test]
    fn compare_hashes_accepts_correct_password() {
        let salt = generate_salt();
        let password = "correct_horse_battery_staple";
        let hash = generate_password_hash(password.as_bytes(), &salt).unwrap();

        assert!(compare_password_hashes(&hash, password.to_string()).is_ok());
    }

    #[test]
    fn compare_hashes_rejects_wrong_password() {
        let salt = generate_salt();
        let hash = generate_password_hash(b"correct_password", &salt).unwrap();

        let result = compare_password_hashes(&hash, "wrong_password".to_string());
        assert!(matches!(result, Err(AuthError::IncorrectCredentials)));
    }

    #[test]
    fn generate_hash_produces_argon2_phc_string() {
        let salt = generate_salt();
        let hash = generate_password_hash(b"any_password", &salt).unwrap();

        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn same_password_different_salts_produce_different_hashes() {
        let password = b"same_password";
        let hash1 = generate_password_hash(password, &generate_salt()).unwrap();
        let hash2 = generate_password_hash(password, &generate_salt()).unwrap();

        assert_ne!(hash1, hash2);
    }
}
