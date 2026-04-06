use std::sync::Mutex;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::{
    errors::RepositoryError, models::short_url::ShortUrl, short_url_spec::ShortUrlSpec,
};

#[async_trait::async_trait]
pub trait ShortUrlRepositoryTrait: Send + Sync {
    async fn get_all(&self) -> Result<Vec<ShortUrl>, RepositoryError>;
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ShortUrl>, RepositoryError>;
    /// Looks up by redirect code. Does NOT filter soft-deleted records — callers check deletion.
    async fn get_by_code(&self, code: &str) -> Result<Option<ShortUrl>, RepositoryError>;
    async fn add_one(&self, spec: ShortUrlSpec) -> Result<ShortUrl, RepositoryError>;
    async fn update_one_by_uuid(&self, spec: ShortUrlSpec) -> Result<ShortUrl, RepositoryError>;
    async fn delete_one_by_uuid(&self, uuid: Uuid) -> Result<bool, RepositoryError>;
}

pub struct InMemoryMockShortUrlRepository {
    store: Mutex<Vec<ShortUrl>>,
}

impl InMemoryMockShortUrlRepository {
    pub fn new(store: Vec<ShortUrl>) -> Self {
        Self {
            store: Mutex::new(store),
        }
    }
}

#[async_trait::async_trait]
impl ShortUrlRepositoryTrait for InMemoryMockShortUrlRepository {
    async fn get_all(&self) -> Result<Vec<ShortUrl>, RepositoryError> {
        let lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        Ok(lock.clone())
    }

    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ShortUrl>, RepositoryError> {
        let lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;
        if let Some(hit) = lock.iter().find(|r| r.uuid == uuid) {
            Ok(Some(hit.clone()))
        } else {
            Ok(None)
        }
    }

    async fn get_by_code(&self, code: &str) -> Result<Option<ShortUrl>, RepositoryError> {
        let lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;
        if let Some(hit) = lock.iter().find(|r| r.code == code) {
            Ok(Some(hit.clone()))
        } else {
            Ok(None)
        }
    }

    async fn add_one(&self, spec: ShortUrlSpec) -> Result<ShortUrl, RepositoryError> {
        let mut lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        let short: ShortUrl = ShortUrl {
            id: (lock.len() + 1) as i64,
            uuid: spec.uuid,
            long_url: spec.long_url,
            code: spec.code,
            expires_at: spec.expires_at,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            user_id: spec.user_id,
        };

        if let Some(duplicate) = lock.iter().find(|r| r.code == short.code) {
            return Err(RepositoryError::Conflict {
                constraint: Some("short_url_code_constraint".to_string()),
                message: format!(
                    "mock short_url insert constraint violation with code: {}",
                    duplicate.code
                )
                .to_string(),
            });
        }

        lock.push(short.clone());

        Ok(short)
    }
    async fn delete_one_by_uuid(&self, uuid: Uuid) -> Result<bool, RepositoryError> {
        let mut lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        if let Some(short) = lock.iter_mut().find(|r| r.uuid == uuid) {
            short.deleted_at = Some(Utc::now());
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn update_one_by_uuid(&self, spec: ShortUrlSpec) -> Result<ShortUrl, RepositoryError> {
        let mut lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        if let Some(hit) = lock.iter_mut().find(|r| r.uuid == spec.uuid) {
            hit.long_url = spec.long_url;
            hit.expires_at = spec.expires_at;
            hit.code = spec.code;
            hit.updated_at = Some(Utc::now());

            Ok(hit.clone())
        } else {
            Err(RepositoryError::Internal(
                "test error - failed to find a short url to update, which cannot happen in reality"
                    .to_string(),
            ))
        }
    }
}
