use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString},
};
use jsonwebtoken::EncodingKey;
use rand_core::OsRng;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    auth_error::AuthError,
    jwt::{AccessClaims, JwtTokenType, JwtTokens, RefreshClaims},
};

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

pub fn validate_token_type(claims: &RefreshClaims, expected_type: JwtTokenType) -> bool {
    if claims.typ == expected_type as u8 {
        true
    } else {
        tracing::error!(
            "Invalid token type. Expected {:?}, Found {:?}",
            expected_type,
            JwtTokenType::from(claims.typ),
        );
        false
    }
}

pub struct GeneratedClaimsDto {
    pub access_claims: AccessClaims,
    pub refresh_claims: RefreshClaims,
}

#[instrument(fields(sub = %sub))]
pub fn generate_claims(
    access_token_expiry_seconds: i64,
    refresh_token_expiry_seconds: i64,
    sub: Uuid,
    roles: String,
) -> Result<GeneratedClaimsDto, AuthError> {
    let time_now = chrono::Utc::now();
    let iat = time_now.timestamp() as usize;
    let sub = sub.to_string();

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
        roles: roles.clone(),
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
        roles,
    };

    tracing::debug!("JWT: generated claims\naccess {:#?}", access_claims,);

    Ok(GeneratedClaimsDto {
        access_claims,
        refresh_claims,
    })
}

#[instrument(skip(jwt_encoding_key))]
pub fn encode_tokens(
    jwt_encoding_key: &EncodingKey,
    access_claims: AccessClaims,
    refresh_claims: RefreshClaims,
) -> Result<JwtTokens, AuthError> {
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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::auth_error::AuthError;

    fn test_encoding_key() -> EncodingKey {
        EncodingKey::from_secret(b"test-secret-for-auth-rs-unit-tests")
    }

    fn make_refresh_claims(token_type: JwtTokenType) -> RefreshClaims {
        RefreshClaims {
            sub: Uuid::now_v7().to_string(),
            jti: Uuid::now_v7().to_string(),
            iat: Utc::now().timestamp() as usize,
            exp: (Utc::now() + chrono::Duration::minutes(15)).timestamp() as usize,
            prf: Uuid::now_v7().to_string(),
            pex: (Utc::now() + chrono::Duration::minutes(5)).timestamp() as usize,
            typ: token_type as u8,
            roles: "user".to_string(),
        }
    }

    fn test_sub() -> Uuid {
        uuid::Uuid::now_v7()
    }

    fn test_roles() -> String {
        "user,admin".to_string()
    }

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

    #[test]
    fn validate_token_type_accepts_matching_type() {
        let claims = make_refresh_claims(JwtTokenType::RefreshToken);

        let actual = validate_token_type(&claims, JwtTokenType::RefreshToken);

        assert!(actual);
    }

    #[test]
    fn validate_token_type_rejects_wrong_type() {
        let claims = make_refresh_claims(JwtTokenType::AccessToken);

        let actual = validate_token_type(&claims, JwtTokenType::RefreshToken);

        assert!(!actual);
    }

    #[test]
    fn generate_claims_sets_subject_pairing_and_token_types() {
        let sub = test_sub();
        let expected_sub = sub.to_string();

        let actual = generate_claims(300, 900, sub, test_roles()).unwrap();

        assert_eq!(actual.access_claims.sub, expected_sub);
        assert_eq!(actual.refresh_claims.sub, expected_sub);
        assert_eq!(actual.access_claims.typ, JwtTokenType::AccessToken as u8);
        assert_eq!(actual.refresh_claims.typ, JwtTokenType::RefreshToken as u8);
        assert_eq!(actual.refresh_claims.prf, actual.access_claims.jti);
        assert_eq!(actual.refresh_claims.pex, actual.access_claims.exp);
    }

    #[test]
    fn generate_claims_preserves_user_roles() {
        let actual = generate_claims(300, 900, test_sub(), test_roles()).unwrap();

        assert_eq!(actual.access_claims.roles, "user,admin");
        assert_eq!(actual.refresh_claims.roles, "user,admin");
    }

    #[test]
    fn generate_claims_sets_expected_expiry_order() {
        let actual = generate_claims(60, 300, test_sub(), test_roles()).unwrap();

        assert!(actual.access_claims.exp > actual.access_claims.iat);
        assert!(actual.refresh_claims.exp > actual.refresh_claims.iat);
        assert!(actual.refresh_claims.exp > actual.access_claims.exp);
    }

    #[test]
    fn encode_tokens_returns_non_empty_tokens() {
        let claims = generate_claims(300, 900, test_sub(), test_roles()).unwrap();

        let actual = encode_tokens(
            &test_encoding_key(),
            claims.access_claims,
            claims.refresh_claims,
        )
        .unwrap();

        assert!(!actual.access_token.is_empty());
        assert!(!actual.refresh_token.is_empty());
    }
}
