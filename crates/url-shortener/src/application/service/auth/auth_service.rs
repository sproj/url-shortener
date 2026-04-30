use jsonwebtoken::EncodingKey;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

use crate::application::{
    security::{
        auth::{
            compare_password_hashes, encode_tokens, generate_claims, generate_password_hash,
            generate_salt, validate_token_type,
        },
        auth_error::AuthError,
        jwt::{ClaimsMethods, JwtTokenType, JwtTokens, RefreshClaims},
    },
    service::{
        auth::{
            auth_service_trait::AuthServiceTrait, refresh_token_cache_trait::RefreshTokenCacheTrait,
        },
        user::{login_params::LoginParams, user_service_trait::UserServiceTrait},
    },
};

pub struct AuthService {
    user_service: Arc<dyn UserServiceTrait>,
    refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
    jwt_access_token_seconds: i64,
    jwt_refresh_token_seconds: i64,
    jwt_encoding_key: EncodingKey,
}

impl AuthService {
    pub fn new(
        user_service: Arc<dyn UserServiceTrait>,
        refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
        jwt_access_token_seconds: i64,
        jwt_refresh_token_seconds: i64,
        jwt_encoding_key: EncodingKey,
    ) -> Self {
        Self {
            user_service,
            refresh_token_cache,
            jwt_access_token_seconds,
            jwt_refresh_token_seconds,
            jwt_encoding_key,
        }
    }
}

#[async_trait::async_trait]
impl AuthServiceTrait for AuthService {
    #[instrument(skip(self), fields(params.username = %params.username))]
    async fn verify_login(&self, params: LoginParams) -> Result<JwtTokens, AuthError> {
        let claims = match self
            .user_service
            .get_one_by_username(&params.username)
            .await
            .map_err(|e| {
                tracing::error!(%e, %params.username, "database error on verifying user for login");
                AuthError::IncorrectCredentials
            })? {
            Some(user) => {
                let true_hash = &user.password_hash;
                compare_password_hashes(true_hash, params.password)?;

                generate_claims(
                    self.jwt_access_token_seconds,
                    self.jwt_refresh_token_seconds,
                    user,
                )
            }
            None => {
                tracing::warn!(%params.username, "login attempt user not found");
                // constant-time dummy work to prevent timing-based enumeration
                let _ = generate_password_hash(params.password.as_bytes(), &generate_salt());
                Err(AuthError::IncorrectCredentials)
            }
        }?;

        tracing::debug!(%params.username, "login successful - caching refresh claim");
        self.cache_refresh_token(&claims.refresh_claims).await?;

        tracing::debug!(%params.username, "login refresh cached - encoding tokens for response");
        encode_tokens(
            &self.jwt_encoding_key,
            claims.access_claims,
            claims.refresh_claims,
        )
    }

    #[instrument(skip(self), fields(sub = %refresh_claims.sub))]
    async fn cache_refresh_token(&self, refresh_claims: &RefreshClaims) -> Result<(), AuthError> {
        let refresh_exp_secs = refresh_claims.exp as u64;
        let now = chrono::Utc::now().timestamp();
        let ttl =
            std::time::Duration::from_secs(refresh_exp_secs.saturating_sub(now.max(0) as u64));
        // SET EX 0 will be rejected by redis. If an exp passes between validation and now (what are the odds) it will look like a caching error rather than a coincidence.
        // In any case the refresh token is now definitely expired so no new tokens for you.
        if ttl.is_zero() {
            return Err(AuthError::ExpiredSignature(
                "refresh token expired between validation and refresh. Uncanny.".to_string(),
            ));
        }

        self.refresh_token_cache
            .set(&refresh_claims.prf, refresh_claims, ttl)
            .await
            .map_err(|e| {
                tracing::error!(%e, "auth service cache operation failed");
                AuthError::Internal
            })?;

        Ok(())
    }

    #[instrument(skip(self), fields(sub = %refresh_claims.sub))]
    async fn refresh(&self, refresh_claims: RefreshClaims) -> Result<JwtTokens, AuthError> {
        let jti = refresh_claims.get_jti();
        if !validate_token_type(&refresh_claims, JwtTokenType::RefreshToken) {
            tracing::error!(%jti, "non-refresh token presented for refresh");
            return Err(AuthError::InvalidToken);
        }

        if self
            .refresh_token_cache
            .get(&refresh_claims.prf)
            .await
            .map_err(|e| {
                tracing::error!(%e, "get refresh token - auth service cache operation failed");
                AuthError::Internal
            })?
            .is_none()
        {
            tracing::warn!(%jti, "refresh attempted with revoked token");
            return Err(AuthError::InvalidToken);
        }

        let user_id = refresh_claims.get_sub().parse::<Uuid>().map_err(|_| {
            tracing::error!(%jti, "failed to parse sub from refresh token");
            AuthError::InvalidToken
        })?;

        match self
            .user_service
            .get_one_by_uuid(user_id)
            .await
            .map_err(|e| {
                tracing::error!(%e, "token refresh failed with UserError");
                AuthError::InvalidToken
            })? {
            None => {
                tracing::error!(%user_id, "token refresh attempted with unknown user");
                Err(AuthError::InvalidToken)
            }
            Some(user) => {
                self.refresh_token_cache
                    .revoke(&refresh_claims.prf)
                    .await
                    .map_err(|e| {
                        tracing::error!(%e, "revoke refresh token - auth service cache operation failed");
                        AuthError::Internal
                    })?;

                let claims = generate_claims(
                    self.jwt_access_token_seconds,
                    self.jwt_refresh_token_seconds,
                    user,
                )?;

                self.cache_refresh_token(&claims.refresh_claims).await?;

                let tokens = encode_tokens(
                    &self.jwt_encoding_key,
                    claims.access_claims,
                    claims.refresh_claims,
                )?;

                Ok(tokens)
            }
        }
    }

    #[instrument(skip(self))]
    async fn revoke_refresh(&self, access_token_jti: &str) -> Result<(), AuthError> {
        tracing::debug!(%access_token_jti, "revoking access token");
        self.refresh_token_cache
            .revoke(access_token_jti)
            .await
            .map_err(|e| {
                tracing::error!(%e, "revoke refresh - auth service cache operation failed");
                AuthError::Internal
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;

    use crate::{
        application::{
            security::{
                auth::{generate_claims, generate_password_hash, generate_salt},
                jwt::{JwtTokenType, RefreshClaims},
            },
            service::{
                auth::{
                    refresh_token_cache::mocks::MockRefreshTokenCache,
                    refresh_token_cache_trait::NoopRefreshTokenCache,
                },
                user::user_service::UsersService,
            },
        },
        domain::{
            errors::RepositoryError,
            models::user::User,
            traits::{InMemoryMockUsersRepository, UsersRepositoryTrait},
            user_spec::UserSpec,
        },
    };

    use super::*;

    struct FailingUsersRepository {
        fail_get_by_uuid: bool,
        fail_get_by_username: bool,
    }

    #[async_trait::async_trait]
    impl UsersRepositoryTrait for FailingUsersRepository {
        async fn get_all(&self) -> Result<Vec<User>, RepositoryError> {
            unimplemented!()
        }

        async fn get_user_by_uuid(&self, _uuid: Uuid) -> Result<Option<User>, RepositoryError> {
            if self.fail_get_by_uuid {
                Err(RepositoryError::Internal(
                    "forced get_user_by_uuid failure".to_string(),
                ))
            } else {
                Ok(None)
            }
        }

        async fn get_user_by_username(
            &self,
            _username: &str,
        ) -> Result<Option<User>, RepositoryError> {
            if self.fail_get_by_username {
                Err(RepositoryError::Internal(
                    "forced get_user_by_username failure".to_string(),
                ))
            } else {
                Ok(None)
            }
        }

        async fn add_user(&self, _spec: UserSpec) -> Result<User, RepositoryError> {
            unimplemented!()
        }

        async fn soft_delete_user_by_uuid(&self, _uuid: Uuid) -> Result<bool, RepositoryError> {
            unimplemented!()
        }

        async fn update_password_by_uuid(
            &self,
            _uuid: Uuid,
            _hash: &str,
            _salt: &str,
        ) -> Result<bool, RepositoryError> {
            unimplemented!()
        }
    }

    fn test_encoding_key() -> EncodingKey {
        EncodingKey::from_secret(b"test-secret-for-auth-service-unit-tests")
    }

    fn make_password_hash(password: &str) -> String {
        generate_password_hash(password.as_bytes(), &generate_salt()).unwrap()
    }

    fn make_user(id: i64, username: &str, password: &str) -> User {
        User {
            id,
            uuid: Uuid::now_v7(),
            username: username.to_string(),
            email: format!("{username}@example.com"),
            password_hash: make_password_hash(password),
            password_salt: "salt".to_string(),
            active: true,
            roles: "user".to_string(),
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
        }
    }

    fn make_auth_service(
        user_service: Arc<dyn UserServiceTrait>,
        refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
    ) -> AuthService {
        AuthService::new(
            user_service,
            refresh_token_cache,
            300,
            900,
            test_encoding_key(),
        )
    }

    fn make_users_service(users: Vec<User>) -> Arc<dyn UserServiceTrait> {
        Arc::new(UsersService::new(Arc::new(
            InMemoryMockUsersRepository::new(users),
        )))
    }

    fn make_failing_users_service(
        fail_get_by_uuid: bool,
        fail_get_by_username: bool,
    ) -> Arc<dyn UserServiceTrait> {
        Arc::new(UsersService::new(Arc::new(FailingUsersRepository {
            fail_get_by_uuid,
            fail_get_by_username,
        })))
    }

    fn make_refresh_claims(user: &User) -> RefreshClaims {
        generate_claims(300, 900, user.clone())
            .unwrap()
            .refresh_claims
    }

    #[tokio::test]
    async fn verify_login_succeeds_and_caches_refresh_token() {
        let user = make_user(1, "verify_login_succeeds", "correct-password");
        let cache = Arc::new(MockRefreshTokenCache::empty());
        let sut = make_auth_service(make_users_service(vec![user]), cache.clone());

        let actual = sut
            .verify_login(LoginParams {
                username: "verify_login_succeeds".to_string(),
                password: "correct-password".to_string(),
            })
            .await
            .unwrap();

        assert!(!actual.access_token.is_empty());
        assert!(!actual.refresh_token.is_empty());
        assert_eq!(cache.set_calls().len(), 1);
        assert!(cache.set_calls()[0].2.as_secs() > 0);
    }

    #[tokio::test]
    async fn verify_login_returns_incorrect_credentials_for_unknown_user() {
        let sut = make_auth_service(make_users_service(vec![]), Arc::new(NoopRefreshTokenCache));

        let actual = sut
            .verify_login(LoginParams {
                username: "missing-user".to_string(),
                password: "irrelevant".to_string(),
            })
            .await;

        assert!(matches!(actual, Err(AuthError::IncorrectCredentials)));
    }

    #[tokio::test]
    async fn verify_login_returns_incorrect_credentials_for_wrong_password() {
        let user = make_user(1, "verify_login_wrong_password", "correct-password");
        let sut = make_auth_service(
            make_users_service(vec![user]),
            Arc::new(NoopRefreshTokenCache),
        );

        let actual = sut
            .verify_login(LoginParams {
                username: "verify_login_wrong_password".to_string(),
                password: "wrong-password".to_string(),
            })
            .await;

        assert!(matches!(actual, Err(AuthError::IncorrectCredentials)));
    }

    #[tokio::test]
    async fn verify_login_returns_incorrect_credentials_when_user_lookup_fails() {
        let sut = make_auth_service(
            make_failing_users_service(false, true),
            Arc::new(NoopRefreshTokenCache),
        );

        let actual = sut
            .verify_login(LoginParams {
                username: "lookup-fails".to_string(),
                password: "irrelevant".to_string(),
            })
            .await;

        assert!(matches!(actual, Err(AuthError::IncorrectCredentials)));
    }

    #[tokio::test]
    async fn cache_refresh_token_sets_cache_entry() {
        let user = make_user(1, "cache_refresh_token_sets_cache_entry", "password");
        let refresh_claims = make_refresh_claims(&user);
        let cache = Arc::new(MockRefreshTokenCache::empty());
        let sut = make_auth_service(make_users_service(vec![user]), cache.clone());

        sut.cache_refresh_token(&refresh_claims).await.unwrap();

        let calls = cache.set_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, refresh_claims.prf);
        assert!(calls[0].2.as_secs() > 0);
    }

    #[tokio::test]
    async fn cache_refresh_token_rejects_expired_claims() {
        let user = make_user(1, "cache_refresh_token_rejects_expired_claims", "password");
        let mut refresh_claims = make_refresh_claims(&user);
        refresh_claims.exp = (Utc::now().timestamp() - 10) as usize;
        let sut = make_auth_service(
            make_users_service(vec![user]),
            Arc::new(MockRefreshTokenCache::empty()),
        );

        let actual = sut.cache_refresh_token(&refresh_claims).await;

        assert!(matches!(
            actual.unwrap_err(),
            AuthError::ExpiredSignature(..)
        ));
    }

    #[tokio::test]
    async fn refresh_rejects_non_refresh_token() {
        let user = make_user(1, "refresh_rejects_non_refresh_token", "password");
        let mut refresh_claims = make_refresh_claims(&user);
        refresh_claims.typ = JwtTokenType::AccessToken as u8;
        let sut = make_auth_service(
            make_users_service(vec![user]),
            Arc::new(NoopRefreshTokenCache),
        );

        let actual = sut.refresh(refresh_claims).await;

        assert!(matches!(actual, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn refresh_rejects_when_cached_refresh_token_is_missing() {
        let user = make_user(
            1,
            "refresh_rejects_when_cached_refresh_token_is_missing",
            "password",
        );
        let refresh_claims = make_refresh_claims(&user);
        let sut = make_auth_service(
            make_users_service(vec![user]),
            Arc::new(NoopRefreshTokenCache),
        );

        let actual = sut.refresh(refresh_claims).await;

        assert!(matches!(actual, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn refresh_rejects_invalid_subject_uuid() {
        let user = make_user(1, "refresh_rejects_invalid_subject_uuid", "password");
        let mut refresh_claims = make_refresh_claims(&user);
        refresh_claims.sub = "not-a-uuid".to_string();
        let cache = Arc::new(MockRefreshTokenCache::new(vec![(
            refresh_claims.prf.clone(),
            refresh_claims.clone(),
        )]));
        let sut = make_auth_service(make_users_service(vec![user]), cache);

        let actual = sut.refresh(refresh_claims).await;

        assert!(matches!(actual, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn refresh_rejects_unknown_user() {
        let user = make_user(1, "refresh_rejects_unknown_user", "password");
        let refresh_claims = make_refresh_claims(&user);
        let cache = Arc::new(MockRefreshTokenCache::new(vec![(
            refresh_claims.prf.clone(),
            refresh_claims.clone(),
        )]));
        let sut = make_auth_service(make_users_service(vec![]), cache);

        let actual = sut.refresh(refresh_claims).await;

        assert!(matches!(actual, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn refresh_rejects_when_user_lookup_fails() {
        let user = make_user(1, "refresh_rejects_when_user_lookup_fails", "password");
        let refresh_claims = make_refresh_claims(&user);
        let cache = Arc::new(MockRefreshTokenCache::new(vec![(
            refresh_claims.prf.clone(),
            refresh_claims.clone(),
        )]));
        let sut = make_auth_service(make_failing_users_service(true, false), cache);

        let actual = sut.refresh(refresh_claims).await;

        assert!(matches!(actual, Err(AuthError::InvalidToken)));
    }

    #[tokio::test]
    async fn refresh_succeeds_and_rotates_refresh_token() {
        let user = make_user(1, "refresh_succeeds_and_rotates_refresh_token", "password");
        let refresh_claims = make_refresh_claims(&user);
        let old_pairing_reference = refresh_claims.prf.clone();
        let cache = Arc::new(MockRefreshTokenCache::new(vec![(
            old_pairing_reference.clone(),
            refresh_claims.clone(),
        )]));
        let sut = make_auth_service(make_users_service(vec![user]), cache.clone());

        let actual = sut.refresh(refresh_claims).await.unwrap();

        assert!(!actual.access_token.is_empty());
        assert!(!actual.refresh_token.is_empty());
        assert_eq!(cache.revoked(), vec![old_pairing_reference]);
        assert_eq!(cache.set_calls().len(), 1);
    }

    #[tokio::test]
    async fn revoke_refresh_forwards_to_cache() {
        let cache = Arc::new(MockRefreshTokenCache::empty());
        let sut = make_auth_service(make_users_service(vec![]), cache.clone());

        sut.revoke_refresh("access-jti").await.unwrap();

        assert_eq!(cache.revoked(), vec!["access-jti".to_string()]);
    }
}
