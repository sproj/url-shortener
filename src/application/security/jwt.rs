use axum::{Json, response::IntoResponse};
use jsonwebtoken::{DecodingKey, EncodingKey, errors::ErrorKind};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;

use crate::application::{
    constants::USER_ROLE_ADMIN,
    security::{auth_error::AuthError, roles},
};

#[derive(Clone)]
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl std::fmt::Debug for JwtKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtKeys").finish()
    }
}

impl JwtKeys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

pub struct JwtTokens {
    pub access_token: String,
    pub refresh_token: String,
}

pub fn tokens_to_response(jwt_tokens: JwtTokens) -> impl IntoResponse {
    let json = json!({
        "access_token": jwt_tokens.access_token,
        "refresh_token": jwt_tokens.refresh_token,
        "token_type": "Bearer"
    });
    Json(json)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub aud: String,
    pub iss: String,
    pub iat: usize,
    pub exp: usize,
    pub jti: String,
    pub roles: String,
    pub typ: u8,
}

impl Display for AccessClaims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "sub: {}, iss: {}, aud: {}, iat: {}, exp: {}",
            self.sub, self.iss, self.aud, self.iat, self.exp
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    /// Subject.
    pub sub: String,
    /// JWT ID.
    pub jti: String,
    /// Issued time.
    pub iat: usize,
    /// Expiration time.
    pub exp: usize,
    /// Reference to paired access token,
    pub prf: String,
    /// Expiration time of paired access token,
    pub pex: usize,
    /// Token type.
    pub typ: u8,
    /// Roles.
    pub roles: String,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum JwtTokenType {
    AccessToken,
    RefreshToken,
    UnknownToken,
}
impl From<u8> for JwtTokenType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::AccessToken,
            1 => Self::RefreshToken,
            _ => Self::UnknownToken,
        }
    }
}

pub trait ClaimsMethods {
    fn validate_role_admin(&self) -> Result<(), AuthError>;
    fn get_sub(&self) -> &str;
    fn get_exp(&self) -> usize;
    fn get_iat(&self) -> usize;
    fn get_jti(&self) -> &str;
}

impl ClaimsMethods for AccessClaims {
    fn validate_role_admin(&self) -> Result<(), AuthError> {
        if self
            .roles
            .split(',')
            .any(|role| role.trim().eq(USER_ROLE_ADMIN))
        {
            Ok(())
        } else {
            tracing::warn!("admin action attempted without admin privilege");
            Err(AuthError::Forbidden)
        }
    }
    fn get_sub(&self) -> &str {
        &self.sub
    }

    fn get_iat(&self) -> usize {
        self.iat
    }

    fn get_exp(&self) -> usize {
        self.exp
    }

    fn get_jti(&self) -> &str {
        &self.jti
    }
}

impl ClaimsMethods for RefreshClaims {
    fn validate_role_admin(&self) -> Result<(), AuthError> {
        roles::is_role_admin(&self.roles)
    }
    fn get_sub(&self) -> &str {
        &self.sub
    }

    fn get_iat(&self) -> usize {
        self.iat
    }

    fn get_exp(&self) -> usize {
        self.exp
    }

    fn get_jti(&self) -> &str {
        &self.jti
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::{
        application::security::auth::{encode_tokens, generate_claims},
        domain::models::user::User,
    };

    fn test_keys() -> JwtKeys {
        JwtKeys::new(b"test-secret-for-unit-tests-only-32b")
    }

    fn make_test_user() -> User {
        User {
            id: 1,
            uuid: Uuid::now_v7(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            password_salt: "salt".to_string(),
            active: true,
            roles: "user".to_string(),
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
        }
    }

    fn make_claims_with_roles(roles: &str) -> AccessClaims {
        AccessClaims {
            sub: "test-sub".to_string(),
            aud: "url-shortener".to_string(),
            iss: "url-shortener".to_string(),
            iat: 0,
            exp: usize::MAX,
            jti: "test-jti".to_string(),
            roles: roles.to_string(),
            typ: JwtTokenType::AccessToken as u8,
        }
    }

    // --- generate_tokens + decode_token roundtrip ---

    #[test]
    fn token_sub_matches_user_uuid() {
        let keys = test_keys();
        let user = make_test_user();
        let expected_uuid = user.uuid.to_string();

        let claims = generate_claims(120, 750, user).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert_eq!(actual.sub, expected_uuid);
    }

    #[test]
    fn token_roles_match_user_roles() {
        let keys = test_keys();
        let mut user = make_test_user();
        user.roles = "admin,user".to_string();

        let claims = generate_claims(120, 750, user).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert_eq!(actual.roles, "admin,user");
    }

    #[test]
    fn token_exp_is_approximately_now_plus_expiry() {
        let keys = test_keys();
        let user = make_test_user();
        let expiry_seconds = 300;
        let before = Utc::now().timestamp();

        let claims = generate_claims(expiry_seconds, -750, user).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        let after = Utc::now().timestamp() as usize;
        assert!(actual.exp >= before as usize + expiry_seconds as usize);
        assert!(actual.exp <= after as usize + expiry_seconds as usize);
    }

    #[test]
    fn token_aud_and_iss_are_set() {
        let keys = test_keys();

        let claims = generate_claims(120, 750, make_test_user()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert_eq!(actual.aud, "url-shortener");
        assert_eq!(actual.iss, "url-shortener");
    }

    #[test]
    fn token_jti_is_non_empty() {
        let keys = test_keys();

        let claims = generate_claims(120, 750, make_test_user()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert!(!actual.jti.is_empty());
    }

    // --- decode_token rejection cases ---

    #[test]
    fn decode_rejects_expired_token() {
        let keys = test_keys();
        // exp = now - 120s, leeway = 60s, so this is definitely expired
        let claims = generate_claims(-120, -60, make_test_user()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        let result = decode_token::<AccessClaims>(&tokens.access_token, &keys.decoding);

        assert!(matches!(result, Err(AuthError::ExpiredSignature(_))));
    }

    #[test]
    fn decode_rejects_wrong_key() {
        let keys = test_keys();
        let other_keys = JwtKeys::new(b"a-completely-different-secret-key");

        let claims = generate_claims(120, 750, make_test_user()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        let result = decode_token::<AccessClaims>(&tokens.access_token, &other_keys.decoding);

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn decode_rejects_tampered_payload() {
        let keys = test_keys();
        let claims = generate_claims(120, 750, make_test_user()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        let parts: Vec<&str> = tokens.access_token.split('.').collect();
        let tampered = format!("{}.dGFtcGVyZWQ.{}", parts[0], parts[2]);

        let result = decode_token::<AccessClaims>(&tampered, &keys.decoding);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    // --- ClaimsMethods ---

    #[test]
    fn validate_role_admin_accepts_admin_role() {
        let claims = make_claims_with_roles("admin");
        assert!(claims.validate_role_admin().is_ok());
    }

    #[test]
    fn validate_role_admin_accepts_admin_in_multi_role() {
        let claims = make_claims_with_roles("user,admin");
        assert!(claims.validate_role_admin().is_ok());
    }

    #[test]
    fn validate_role_admin_rejects_non_admin() {
        let claims = make_claims_with_roles("user");
        assert!(matches!(
            claims.validate_role_admin(),
            Err(AuthError::Forbidden)
        ));
    }

    #[test]
    fn validate_role_admin_trims_whitespace() {
        let claims = make_claims_with_roles("user, admin");
        assert!(claims.validate_role_admin().is_ok());
    }
}

pub fn decode_token<T: for<'de> serde::Deserialize<'de>>(
    token: &str,
    decoding_key: &DecodingKey,
) -> Result<T, AuthError> {
    let mut validation = jsonwebtoken::Validation::default();
    // validation.leeway = config.jwt.jwt_validation_leeway_seconds as u64;
    // todo: reckon hardcoding better than putting jwt config on State - think it through.
    validation.leeway = 60u64;
    validation.set_audience(&["url-shortener"]);
    validation.set_issuer(&["url-shortener"]);

    let token_data = jsonwebtoken::decode::<T>(token, decoding_key, &validation).map_err(|e| {
        tracing::warn!(%e, "token validation rejected");
        match e.kind() {
            ErrorKind::ExpiredSignature => AuthError::ExpiredSignature(e.to_string()),
            _ => AuthError::InvalidToken,
        }
    })?;

    Ok(token_data.claims)
}
