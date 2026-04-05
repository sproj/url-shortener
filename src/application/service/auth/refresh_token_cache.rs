use redis::{AsyncTypedCommands, aio::MultiplexedConnection};
use tracing::instrument;

use crate::{
    application::{
        security::jwt::RefreshClaims,
        service::auth::refresh_token_cache_trait::RefreshTokenCacheTrait,
    },
    infrastructure::redis::cache_error::CacheError,
};

pub struct RefreshTokenCache {
    redis: MultiplexedConnection,
}

impl RefreshTokenCache {
    pub fn new(conn: MultiplexedConnection) -> Self {
        Self { redis: conn }
    }
}

#[async_trait::async_trait]
impl RefreshTokenCacheTrait for RefreshTokenCache {
    #[instrument(skip(self))]
    async fn get(&self, access_token_jti: &str) -> Result<Option<RefreshClaims>, CacheError> {
        let mut conn = self.redis.clone();
        let raw: Option<String> = conn.get(access_token_jti).await?;
        match raw {
            None => Ok(None),
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
        }
    }

    #[instrument(skip(self, refresh_claims))]
    async fn set(
        &self,
        access_token_jti: &str,
        refresh_claims: &RefreshClaims,
        ttl: std::time::Duration,
    ) -> Result<(), CacheError> {
        let json = serde_json::to_string(refresh_claims)?;
        let mut conn = self.redis.clone();
        let _: () = conn.set_ex(access_token_jti, json, ttl.as_secs()).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn revoke(&self, access_token_jti: &str) -> Result<(), CacheError> {
        let mut conn = self.redis.clone();
        let _ = conn.del(access_token_jti).await?;
        Ok(())
    }
}
