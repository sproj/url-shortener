use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString},
};
use jsonwebtoken::{DecodingKey, EncodingKey, errors::ErrorKind};
use rand_core::OsRng;
use tracing::instrument;

use crate::{auth_error::AuthError, jwt::JwtTokens};

#[instrument(skip_all)]
pub fn compare_password_hashes(
    true_hash: &str,
    user_input_password: String,
) -> Result<(), AuthError> {
    let parsed_hash = PasswordHash::new(true_hash).map_err(AuthError::HashingError)?;
    Argon2::default()
        .verify_password(user_input_password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::IncorrectCredentials)
}

#[instrument]
pub fn generate_salt() -> SaltString {
    SaltString::generate(&mut OsRng)
}

#[instrument(skip_all)]
pub fn generate_password_hash(pw: &[u8], salt: &SaltString) -> Result<String, AuthError> {
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(pw, salt)
        .map_err(AuthError::HashingError)?
        .to_string();
    Ok(password_hash)
}

#[instrument(skip_all)]
pub fn encode_tokens<A, R>(
    jwt_encoding_key: &EncodingKey,
    access_claims: A,
    refresh_claims: R,
) -> Result<JwtTokens, AuthError>
where
    A: serde::Serialize,
    R: serde::Serialize,
{
    let access_token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &access_claims,
        jwt_encoding_key,
    )
    .map_err(|e| {
        tracing::error!(%e, "failed to encode access token");
        AuthError::TokenCreation
    })?;

    let refresh_token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &refresh_claims,
        jwt_encoding_key,
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

pub fn decode_token<T: for<'de> serde::Deserialize<'de>>(
    token: &str,
    decoding_key: &DecodingKey,
) -> Result<T, AuthError> {
    let mut validation = jsonwebtoken::Validation::default();
    // todo: reckon hardcoding better than putting jwt config on State - think it through.
    validation.leeway = 60u64;
    validation.set_audience(&["url-shortener"]);
    validation.set_issuer(&["url-shortener"]);
    validation.set_required_spec_claims(&["prf", "pex"]);

    let token_data = jsonwebtoken::decode::<T>(token, decoding_key, &validation).map_err(|e| {
        tracing::warn!(%e, "token validation rejected");
        match e.kind() {
            ErrorKind::ExpiredSignature => AuthError::ExpiredSignature(e.to_string()),
            _ => AuthError::InvalidToken,
        }
    })?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_error::AuthError;

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

    #[test]
    fn compare_hashes_rejects_invalid_hash_input() {
        let result = compare_password_hashes("not-a-phc-hash", "password".to_string());

        assert!(matches!(result, Err(AuthError::HashingError(..))));
    }
}
