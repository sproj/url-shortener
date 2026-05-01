use crate::application::security::claims::RefreshClaims;
use crate::application::service::user::login_params::LoginParams;
use auth::{auth_error::AuthError, jwt::JwtTokens};

/// Trait that defines the auth service contract, independent of infrastructure.
///
/// Each method mirrors a function in `auth_service` with `Pool`, `EncodingKey`,
/// token-expiry seconds, and `RefreshTokenCacheTrait` dependencies removed — those
/// are held by the concrete implementation struct.
#[async_trait::async_trait]
pub trait AuthServiceTrait: Send + Sync {
    async fn verify_login(&self, params: LoginParams) -> Result<JwtTokens, AuthError>;
    async fn cache_refresh_token(&self, claims: &RefreshClaims) -> Result<(), AuthError>;
    async fn refresh(&self, claims: RefreshClaims) -> Result<JwtTokens, AuthError>;
    async fn revoke_refresh(&self, access_token_jti: &str) -> Result<(), AuthError>;
}
