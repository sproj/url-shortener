use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::application::{
    repository::users_repository,
    security::{
        auth::{
            GeneratedClaimsDto, compare_password_hashes, generate_claims, generate_password_hash,
            generate_salt,
        },
        auth_error::AuthError,
        jwt::RefreshClaims,
    },
    service::{
        auth::refresh_token_cache_trait::RefreshTokenCacheTrait, user::login_params::LoginParams,
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
    let ttl =
        std::time::Duration::from_secs(refresh_exp_secs.saturating_sub(now.try_into().unwrap()));

    refresh_token_cache
        .set(&refresh_claims.jti, refresh_claims, ttl)
        .await
        .map_err(AuthError::CachingError)?;

    Ok(())
}
