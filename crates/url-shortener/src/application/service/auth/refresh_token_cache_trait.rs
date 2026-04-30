use crate::{
    application::security::jwt::RefreshClaims, infrastructure::redis::cache_error::CacheError,
};

#[async_trait::async_trait]
pub trait RefreshTokenCacheTrait: Send + Sync {
    async fn get(&self, access_token_jti: &str) -> Result<Option<RefreshClaims>, CacheError>;
    async fn set(
        &self,
        access_token_jti: &str,
        value: &RefreshClaims,
        ttl: std::time::Duration,
    ) -> Result<(), CacheError>;
    async fn revoke(&self, access_token_jti: &str) -> Result<(), CacheError>;
}

pub struct NoopRefreshTokenCache;

#[async_trait::async_trait]
impl RefreshTokenCacheTrait for NoopRefreshTokenCache {
    async fn get(&self, _access_token_jti: &str) -> Result<Option<RefreshClaims>, CacheError> {
        Ok(None)
    }

    async fn set(
        &self,
        __access_token_jti: &str,
        __value: &RefreshClaims,
        __ttl: std::time::Duration,
    ) -> Result<(), CacheError> {
        Ok(())
    }

    async fn revoke(&self, _access_token_jti: &str) -> Result<(), CacheError> {
        Ok(())
    }
}
