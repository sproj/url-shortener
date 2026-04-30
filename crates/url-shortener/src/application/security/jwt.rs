use jsonwebtoken::{DecodingKey, EncodingKey, errors::ErrorKind};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct JwtTokens {
    pub access_token: String,
    pub refresh_token: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
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
    const EXPECTED_TYPE: JwtTokenType;
    fn validate_role_admin(&self) -> Result<(), AuthError>;
    fn get_sub(&self) -> &str;
    fn get_exp(&self) -> usize;
    fn get_iat(&self) -> usize;
    fn get_jti(&self) -> &str;
    fn get_typ(&self) -> u8;
}

impl AccessClaims {
    /// Returns `Ok(())` if the requesting user is either the identified subject or an admin.
    /// The subject is identified by `subject_uuid`; the requestor is read from `self.sub`.
    pub fn assert_is_subject_or_admin(&self, subject_uuid: uuid::Uuid) -> Result<(), AuthError> {
        let requestor_uuid = uuid::Uuid::parse_str(&self.sub).map_err(|e| {
            tracing::warn!(%e, "failed to parse sub from access claims");
            AuthError::InvalidToken
        })?;

        if requestor_uuid != subject_uuid && self.validate_role_admin().is_err() {
            tracing::warn!(
                %requestor_uuid,
                %subject_uuid,
                "non-admin requestor attempting to operate on a resource they do not own"
            );
            return Err(AuthError::Forbidden);
        }
        Ok(())
    }
}

impl ClaimsMethods for AccessClaims {
    const EXPECTED_TYPE: JwtTokenType = JwtTokenType::AccessToken;
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

    fn get_typ(&self) -> u8 {
        self.typ
    }
}

impl ClaimsMethods for RefreshClaims {
    const EXPECTED_TYPE: JwtTokenType = JwtTokenType::RefreshToken;
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

    fn get_typ(&self) -> u8 {
        self.typ
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

    fn make_refresh_claims_with_roles(roles: &str) -> RefreshClaims {
        RefreshClaims {
            sub: Uuid::now_v7().to_string(),
            jti: Uuid::now_v7().to_string(),
            iat: Utc::now().timestamp() as usize,
            exp: (Utc::now() + chrono::Duration::minutes(15)).timestamp() as usize,
            prf: Uuid::now_v7().to_string(),
            pex: (Utc::now() + chrono::Duration::minutes(5)).timestamp() as usize,
            typ: JwtTokenType::RefreshToken as u8,
            roles: roles.to_string(),
        }
    }

    #[test]
    fn jwt_token_type_from_maps_expected_variants() {
        assert_eq!(JwtTokenType::from(0), JwtTokenType::AccessToken);
        assert_eq!(JwtTokenType::from(1), JwtTokenType::RefreshToken);
        assert_eq!(JwtTokenType::from(9), JwtTokenType::UnknownToken);
    }

    // --- generate_tokens + decode_token roundtrip ---

    #[test]
    fn token_sub_matches_user_uuid() {
        let keys = test_keys();
        let user = make_test_user();
        let expected_uuid = user.uuid.to_string();

        let claims = generate_claims(120, 750, user.uuid, user.roles).unwrap();
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

        let claims = generate_claims(120, 750, user.uuid, user.roles).unwrap();
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

        let claims = generate_claims(expiry_seconds, -750, user.uuid, user.roles).unwrap();
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
        let user = make_test_user();
        let claims = generate_claims(120, 750, user.uuid, user.roles).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert_eq!(actual.aud, "url-shortener");
        assert_eq!(actual.iss, "url-shortener");
    }

    #[test]
    fn token_jti_is_non_empty() {
        let keys = test_keys();
        let user = make_test_user();
        let claims = generate_claims(120, 750, user.uuid, user.roles).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert!(!actual.jti.is_empty());
    }

    // --- decode_token rejection cases ---

    #[test]
    fn decode_rejects_expired_token() {
        let keys = test_keys();
        let user = make_test_user();
        // exp = now - 120s, leeway = 60s, so this is definitely expired
        let claims = generate_claims(-120, -60, user.uuid, user.roles).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        let result = decode_token::<AccessClaims>(&tokens.access_token, &keys.decoding);

        assert!(matches!(result, Err(AuthError::ExpiredSignature(_))));
    }

    #[test]
    fn decode_rejects_wrong_key() {
        let keys = test_keys();
        let other_keys = JwtKeys::new(b"a-completely-different-secret-key");
        let user = make_test_user();
        let claims = generate_claims(120, 750, user.uuid, user.roles).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        let result = decode_token::<AccessClaims>(&tokens.access_token, &other_keys.decoding);

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn decode_rejects_tampered_payload() {
        let keys = test_keys();
        let user = make_test_user();
        let claims = generate_claims(120, 750, user.uuid, user.roles).unwrap();
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

    #[test]
    fn refresh_claims_validate_role_admin_accepts_admin_role() {
        let claims = make_refresh_claims_with_roles("user,admin");

        assert!(claims.validate_role_admin().is_ok());
    }

    #[test]
    fn refresh_claims_validate_role_admin_rejects_non_admin() {
        let claims = make_refresh_claims_with_roles("user");

        assert!(matches!(
            claims.validate_role_admin(),
            Err(AuthError::Forbidden)
        ));
    }

    #[test]
    fn assert_is_subject_or_admin_accepts_subject_match() {
        let subject_uuid = Uuid::now_v7();
        let claims = AccessClaims {
            sub: subject_uuid.to_string(),
            aud: "url-shortener".to_string(),
            iss: "url-shortener".to_string(),
            iat: 0,
            exp: usize::MAX,
            jti: "test-jti".to_string(),
            roles: "user".to_string(),
            typ: JwtTokenType::AccessToken as u8,
        };

        let actual = claims.assert_is_subject_or_admin(subject_uuid);

        assert!(actual.is_ok());
    }

    #[test]
    fn assert_is_subject_or_admin_accepts_admin_for_other_subject() {
        let claims = AccessClaims {
            sub: Uuid::now_v7().to_string(),
            aud: "url-shortener".to_string(),
            iss: "url-shortener".to_string(),
            iat: 0,
            exp: usize::MAX,
            jti: "test-jti".to_string(),
            roles: "admin".to_string(),
            typ: JwtTokenType::AccessToken as u8,
        };

        let actual = claims.assert_is_subject_or_admin(Uuid::now_v7());

        assert!(actual.is_ok());
    }

    #[test]
    fn assert_is_subject_or_admin_rejects_non_admin_for_other_subject() {
        let claims = AccessClaims {
            sub: Uuid::now_v7().to_string(),
            aud: "url-shortener".to_string(),
            iss: "url-shortener".to_string(),
            iat: 0,
            exp: usize::MAX,
            jti: "test-jti".to_string(),
            roles: "user".to_string(),
            typ: JwtTokenType::AccessToken as u8,
        };

        let actual = claims.assert_is_subject_or_admin(Uuid::now_v7());

        assert!(matches!(actual, Err(AuthError::Forbidden)));
    }

    #[test]
    fn assert_is_subject_or_admin_rejects_invalid_subject_uuid() {
        let claims = AccessClaims {
            sub: "not-a-uuid".to_string(),
            aud: "url-shortener".to_string(),
            iss: "url-shortener".to_string(),
            iat: 0,
            exp: usize::MAX,
            jti: "test-jti".to_string(),
            roles: "admin".to_string(),
            typ: JwtTokenType::AccessToken as u8,
        };

        let actual = claims.assert_is_subject_or_admin(Uuid::now_v7());

        assert!(matches!(actual, Err(AuthError::InvalidToken)));
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
