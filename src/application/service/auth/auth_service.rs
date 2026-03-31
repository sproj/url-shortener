use std::sync::Arc;

use deadpool_postgres::Pool;
use jsonwebtoken::EncodingKey;
use uuid::Uuid;

use crate::application::{
    repository::users_repository,
    security::{
        auth::{
            GeneratedClaimsDto, compare_password_hashes, encode_tokens, generate_claims,
            generate_password_hash, generate_salt, validate_token_type,
        },
        auth_error::AuthError,
        jwt::{ClaimsMethods, JwtTokenType, JwtTokens, RefreshClaims},
    },
    service::{
        auth::refresh_token_cache_trait::RefreshTokenCacheTrait,
        user::{login_params::LoginParams, user_service},
    },
};

pub async fn verify_login(
    db_pool: &Pool,
    jwt_access_token_seconds: i64,
    jwt_refresh_token_seconds: i64,
    params: LoginParams,
) -> Result<GeneratedClaimsDto, AuthError> {
    match users_repository::get_user_by_username(db_pool, &params.username)
        .await
        .map_err(|e| {
            tracing::error!(%e, %params.username, "database error on verifying user for login");
            AuthError::IncorrectCredentials
        })? {
        Some(user) => {
            let true_hash = &user.password_hash;
            compare_password_hashes(true_hash, params.password)?;

            generate_claims(jwt_access_token_seconds, jwt_refresh_token_seconds, user)
        }
        None => {
            tracing::warn!(%params.username, "login attempt user not found");
            // constant-time dummy work to prevent timing-based enumeration
            let _ = generate_password_hash(params.password.as_bytes(), &generate_salt());
            Err(AuthError::IncorrectCredentials)
        }
    }
}

pub async fn cache_refresh_token(
    refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
    refresh_claims: &RefreshClaims,
) -> Result<(), AuthError> {
    let refresh_exp_secs = refresh_claims.exp as u64;
    let now = chrono::Utc::now().timestamp();
    let ttl = std::time::Duration::from_secs(refresh_exp_secs.saturating_sub(now.max(0) as u64));

    refresh_token_cache
        .set(&refresh_claims.prf, refresh_claims, ttl)
        .await
        .map_err(AuthError::CachingError)?;

    Ok(())
}

pub async fn refresh(
    refresh_claims: RefreshClaims,
    db_pool: &Pool,
    access_token_expiry_seconds: i64,
    refresh_token_expiry_seconds: i64,
    jwt_encoding_key: &EncodingKey,
    refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
) -> Result<JwtTokens, AuthError> {
    let jti = refresh_claims.get_jti();
    if !validate_token_type(&refresh_claims, JwtTokenType::RefreshToken) {
        tracing::error!(%jti, "non-refresh token presented for refresh");
        return Err(AuthError::InvalidToken);
    }

    if refresh_token_cache
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

    match user_service::get_one_by_uuid(db_pool, user_id)
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
            refresh_token_cache
                .revoke(&refresh_claims.prf)
                .await
                .map_err(AuthError::CachingError)?;

            let claims = generate_claims(
                access_token_expiry_seconds,
                refresh_token_expiry_seconds,
                user,
            )?;

            cache_refresh_token(refresh_token_cache, &claims.refresh_claims).await?;

            let tokens = encode_tokens(
                jwt_encoding_key,
                claims.access_claims,
                claims.refresh_claims,
            )?;

            Ok(tokens)
        }
    }
}

pub async fn revoke_refresh(
    access_token_jti: &str,
    refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
) -> Result<(), AuthError> {
    refresh_token_cache
        .revoke(access_token_jti)
        .await
        .map_err(AuthError::CachingError)
}
