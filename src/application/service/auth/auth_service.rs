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
            .map_err(AuthError::CachingError)?;

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
            .map_err(AuthError::CachingError)?
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
                    .map_err(AuthError::CachingError)?;

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
        self.refresh_token_cache
            .revoke(access_token_jti)
            .await
            .map_err(AuthError::CachingError)
    }
}
