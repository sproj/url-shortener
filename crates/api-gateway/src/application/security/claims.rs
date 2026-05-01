use std::fmt::Display;

use auth::{
    auth_error::AuthError,
    jwt::{ClaimsMethods, JwtTokenType},
    roles::{USER_ROLE_ADMIN, is_role_admin},
};
use serde::{Deserialize, Serialize};

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
        is_role_admin(&self.roles)
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
    use auth::{
        auth_error::AuthError,
        jwt::{ClaimsMethods, JwtTokenType},
    };
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

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
        assert!(claims.assert_is_subject_or_admin(subject_uuid).is_ok());
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
        assert!(claims.assert_is_subject_or_admin(Uuid::now_v7()).is_ok());
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
        assert!(matches!(
            claims.assert_is_subject_or_admin(Uuid::now_v7()),
            Err(AuthError::Forbidden)
        ));
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
        assert!(matches!(
            claims.assert_is_subject_or_admin(Uuid::now_v7()),
            Err(AuthError::InvalidToken)
        ));
    }
}
