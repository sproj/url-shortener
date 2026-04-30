use redis::{AsyncTypedCommands, aio::MultiplexedConnection};
use tracing::instrument;

use crate::application::security::claims::RefreshClaims;
use crate::{
    application::service::auth::refresh_token_cache_trait::RefreshTokenCacheTrait,
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

#[cfg(test)]
pub mod mocks {
    use std::{
        collections::HashMap,
        sync::{Mutex, MutexGuard},
    };

    use crate::application::security::claims::RefreshClaims;

    use super::*;

    #[derive(Default)]
    struct MockRefreshTokenCacheState {
        entries: HashMap<String, RefreshClaims>,
        set_calls: Vec<(String, RefreshClaims, std::time::Duration)>,
        revoked: Vec<String>,
        fail_get: bool,
        fail_set: bool,
        fail_revoke: bool,
    }

    pub struct MockRefreshTokenCache {
        state: Mutex<MockRefreshTokenCacheState>,
    }

    impl MockRefreshTokenCache {
        pub fn new(entries: Vec<(String, RefreshClaims)>) -> Self {
            let mut state = MockRefreshTokenCacheState::default();
            for (key, value) in entries {
                state.entries.insert(key, value);
            }
            Self {
                state: Mutex::new(state),
            }
        }

        pub fn empty() -> Self {
            Self::new(vec![])
        }

        pub fn with_get_failure(self) -> Self {
            self.lock_state().fail_get = true;
            self
        }

        pub fn with_set_failure(self) -> Self {
            self.lock_state().fail_set = true;
            self
        }

        pub fn with_revoke_failure(self) -> Self {
            self.lock_state().fail_revoke = true;
            self
        }

        pub fn set_calls(&self) -> Vec<(String, RefreshClaims, std::time::Duration)> {
            self.lock_state().set_calls.clone()
        }

        pub fn revoked(&self) -> Vec<String> {
            self.lock_state().revoked.clone()
        }

        fn lock_state(&self) -> MutexGuard<'_, MockRefreshTokenCacheState> {
            self.state.lock().unwrap()
        }

        fn synthetic_error() -> CacheError {
            CacheError::Serialization("forced mock cache failure".to_string())
        }
    }

    #[async_trait::async_trait]
    impl RefreshTokenCacheTrait for MockRefreshTokenCache {
        async fn get(&self, access_token_jti: &str) -> Result<Option<RefreshClaims>, CacheError> {
            let state = self.lock_state();
            if state.fail_get {
                return Err(Self::synthetic_error());
            }
            Ok(state.entries.get(access_token_jti).cloned())
        }

        async fn set(
            &self,
            access_token_jti: &str,
            value: &RefreshClaims,
            ttl: std::time::Duration,
        ) -> Result<(), CacheError> {
            let mut state = self.lock_state();
            if state.fail_set {
                return Err(Self::synthetic_error());
            }
            state
                .entries
                .insert(access_token_jti.to_string(), value.clone());
            state
                .set_calls
                .push((access_token_jti.to_string(), value.clone(), ttl));
            Ok(())
        }

        async fn revoke(&self, access_token_jti: &str) -> Result<(), CacheError> {
            let mut state = self.lock_state();
            if state.fail_revoke {
                return Err(Self::synthetic_error());
            }
            state.revoked.push(access_token_jti.to_string());
            state.entries.remove(access_token_jti);
            Ok(())
        }
    }
}
