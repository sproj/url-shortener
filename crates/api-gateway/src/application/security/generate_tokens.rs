use auth::{auth_error::AuthError, jwt::JwtTokenType};
use tracing::instrument;
use uuid::Uuid;

use crate::application::security::claims::{AccessClaims, RefreshClaims};

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
        iss: "api-gateway".to_string(),
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

#[cfg(test)]
mod tests {
    use auth::{
        auth_error::AuthError,
        decode_token, encode_tokens,
        jwt::{JwtKeys, JwtTokenType},
    };
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn test_keys() -> JwtKeys {
        JwtKeys::new(b"test-secret-for-unit-tests-only-32b")
    }

    fn test_sub() -> Uuid {
        Uuid::now_v7()
    }

    fn test_roles() -> String {
        "user,admin".to_string()
    }

    // --- generate_claims ---

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
        let keys = test_keys();
        let claims = generate_claims(300, 900, test_sub(), test_roles()).unwrap();

        let actual =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        assert!(!actual.access_token.is_empty());
        assert!(!actual.refresh_token.is_empty());
    }

    // --- generate_claims + encode_tokens + decode_token roundtrip ---

    #[test]
    fn token_sub_matches_user_uuid() {
        let keys = test_keys();
        let sub = test_sub();
        let expected_uuid = sub.to_string();

        let claims = generate_claims(120, 750, sub, test_roles()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert_eq!(actual.sub, expected_uuid);
    }

    #[test]
    fn token_roles_match_user_roles() {
        let keys = test_keys();
        let roles = "admin,user".to_string();

        let claims = generate_claims(120, 750, test_sub(), roles).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert_eq!(actual.roles, "admin,user");
    }

    #[test]
    fn token_exp_is_approximately_now_plus_expiry() {
        let keys = test_keys();
        let expiry_seconds = 300;
        let before = Utc::now().timestamp();

        let claims = generate_claims(expiry_seconds, -750, test_sub(), test_roles()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        let after = Utc::now().timestamp() as usize;
        assert!(actual.exp >= before as usize + expiry_seconds as usize);
        assert!(actual.exp <= after + expiry_seconds as usize);
    }

    #[test]
    fn token_aud_and_iss_are_set() {
        let keys = test_keys();
        let claims = generate_claims(120, 750, test_sub(), test_roles()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();
        let actual: AccessClaims = decode_token(&tokens.access_token, &keys.decoding).unwrap();

        assert_eq!(actual.aud, "url-shortener");
        assert_eq!(actual.iss, "api-gateway");
    }

    #[test]
    fn token_jti_is_non_empty() {
        let keys = test_keys();
        let claims = generate_claims(120, 750, test_sub(), test_roles()).unwrap();
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
        let claims = generate_claims(-120, -60, test_sub(), test_roles()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        let result = decode_token::<AccessClaims>(&tokens.access_token, &keys.decoding);

        assert!(matches!(result, Err(AuthError::ExpiredSignature(_))));
    }

    #[test]
    fn decode_rejects_wrong_key() {
        let keys = test_keys();
        let other_keys = JwtKeys::new(b"a-completely-different-secret-key");

        let claims = generate_claims(120, 750, test_sub(), test_roles()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        let result = decode_token::<AccessClaims>(&tokens.access_token, &other_keys.decoding);

        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }

    #[test]
    fn decode_rejects_tampered_payload() {
        let keys = test_keys();

        let claims = generate_claims(120, 750, test_sub(), test_roles()).unwrap();
        let tokens =
            encode_tokens(&keys.encoding, claims.access_claims, claims.refresh_claims).unwrap();

        let parts: Vec<&str> = tokens.access_token.split('.').collect();
        let tampered = format!("{}.dGFtcGVyZWQ.{}", parts[0], parts[2]);

        let result = decode_token::<AccessClaims>(&tampered, &keys.decoding);
        assert!(matches!(result, Err(AuthError::InvalidToken)));
    }
}
