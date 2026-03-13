use crate::{
    application::service::short_url::short_url_service::RedirectDecision,
    infrastructure::redis::cache_error::CacheError,
};

#[async_trait::async_trait]
pub trait RedirectCache: Send + Sync {
    async fn get(&self, code: &str) -> Result<Option<RedirectDecision>, CacheError>;
    async fn set(
        &self,
        code: &str,
        value: &RedirectDecision,
        ttl: std::time::Duration,
    ) -> Result<(), CacheError>;
    async fn delete(&self, code: &str) -> Result<(), CacheError>;
}

pub struct NoopRedirectCache;

#[async_trait::async_trait]
impl RedirectCache for NoopRedirectCache {
    async fn get(&self, _code: &str) -> Result<Option<RedirectDecision>, CacheError> {
        Ok(None)
    }

    async fn set(
        &self,
        _code: &str,
        _value: &RedirectDecision,
        _ttl: std::time::Duration,
    ) -> Result<(), CacheError> {
        Ok(())
    }

    async fn delete(&self, _code: &str) -> Result<(), CacheError> {
        Ok(())
    }
}
