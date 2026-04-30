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

#[cfg(test)]
pub mod mocks {
    use super::*;
    use std::{collections::HashMap, sync::Mutex};

    pub struct RecordingRedirectCache {
        values: Mutex<HashMap<String, RedirectDecision>>,
        deleted: Mutex<Vec<String>>,
        fail_delete: bool,
    }

    impl RecordingRedirectCache {
        pub fn new() -> Self {
            Self {
                values: Mutex::new(HashMap::new()),
                deleted: Mutex::new(Vec::new()),
                fail_delete: false,
            }
        }

        pub fn with_value(code: &str, decision: RedirectDecision) -> Self {
            let mut values = HashMap::new();
            values.insert(code.to_string(), decision);
            Self {
                values: Mutex::new(values),
                deleted: Mutex::new(Vec::new()),
                fail_delete: false,
            }
        }

        pub fn deleted_codes(&self) -> Vec<String> {
            self.deleted.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl RedirectCache for RecordingRedirectCache {
        async fn get(&self, code: &str) -> Result<Option<RedirectDecision>, CacheError> {
            Ok(self.values.lock().unwrap().get(code).cloned())
        }

        async fn set(
            &self,
            code: &str,
            value: &RedirectDecision,
            _ttl: std::time::Duration,
        ) -> Result<(), CacheError> {
            self.values
                .lock()
                .unwrap()
                .insert(code.to_string(), value.clone());
            Ok(())
        }

        async fn delete(&self, code: &str) -> Result<(), CacheError> {
            self.deleted.lock().unwrap().push(code.to_string());
            if self.fail_delete {
                return Err(CacheError::Serialization(
                    "forced delete failure".to_string(),
                ));
            }
            self.values.lock().unwrap().remove(code);
            Ok(())
        }
    }
}
