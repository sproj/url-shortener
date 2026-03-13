use redis::{AsyncTypedCommands, aio::MultiplexedConnection};

use crate::{
    application::service::short_url::{
        redirect_cache_trait::RedirectCache, short_url_service::RedirectDecision,
    },
    infrastructure::redis::cache_error::CacheError,
};

pub struct RedirectCacheChecker {
    redis: MultiplexedConnection,
}

impl RedirectCacheChecker {
    pub fn new(conn: MultiplexedConnection) -> Self {
        Self { redis: conn }
    }
}

#[async_trait::async_trait]
impl RedirectCache for RedirectCacheChecker {
    async fn get(&self, code: &str) -> Result<Option<RedirectDecision>, CacheError> {
        let mut conn = self.redis.clone();
        let raw: Option<String> = conn.get(code).await?;
        match raw {
            None => Ok(None),
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
        }
    }

    async fn set(
        &self,
        code: &str,
        redirect: &RedirectDecision,
        ttl: std::time::Duration,
    ) -> Result<(), CacheError> {
        let json = serde_json::to_string(redirect)?;
        let mut conn = self.redis.clone();
        let _: () = conn.set_ex(code, json, ttl.as_secs()).await?;
        Ok(())
    }

    async fn delete(&self, code: &str) -> Result<(), CacheError> {
        let mut conn = self.redis.clone();
        let _ = conn.del(code).await?;
        Ok(())
    }
}
