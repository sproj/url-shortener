use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    api::handlers::short_url::ValidatedCreateShortUrlRequest,
    application::{
        ShortUrlError,
        repository::short_url_repository::ShortUrlRepository,
        service::short_url::{
            ShortUrlSpec, code_generator::CodeGenerator, redirect_cache::RedirectCacheChecker,
            redirect_cache_trait::RedirectCache,
        },
    },
    domain::models::short_url::ShortUrl,
    infrastructure::database::database_error::DatabaseError,
};

const SHORT_URL_CODE_KEY_CONSTRAINT_NAME: &str = "short_url_code_key";

pub struct ShortUrlService {
    code_generator: Arc<dyn CodeGenerator>,
    max_retries: u8,
    repository: Arc<ShortUrlRepository>,
    redirect_cache: Arc<dyn RedirectCache>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RedirectDecision {
    Permanent { long_url: String },
    Temporary { long_url: String },
    Gone,
    NotFound,
}

impl ShortUrlService {
    pub fn new(
        repository: ShortUrlRepository,
        code_generator: Arc<dyn CodeGenerator>,
        max_retries: u8,
        redirect_cache: RedirectCacheChecker,
    ) -> Self {
        ShortUrlService {
            code_generator,
            max_retries,
            repository: Arc::new(repository),
            redirect_cache: Arc::new(redirect_cache),
        }
    }

    pub async fn get_all(&self) -> Result<Vec<ShortUrl>, DatabaseError> {
        self.repository.get_all().await
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<ShortUrl>, ShortUrlError> {
        self.repository
            .get_by_id(id)
            .await
            .map_err(ShortUrlError::Storage)
    }

    pub async fn get_by_code(&self, code: &str) -> Result<Option<ShortUrl>, DatabaseError> {
        self.repository.get_by_code(code).await
    }

    pub async fn delete_one_by_id(&self, id: i64) -> Result<bool, DatabaseError> {
        self.repository.delete_one_by_id(id).await
    }

    pub async fn add_one(
        &self,
        dto: ValidatedCreateShortUrlRequest,
    ) -> Result<ShortUrl, ShortUrlError> {
        // uuid is stable across insert attempts. `code` is re-generated on conflict (should be very rare but is possible).
        let uuid = uuid::Uuid::now_v7();

        for attempt in 1..=self.max_retries {
            let spec = ShortUrlSpec {
                long_url: dto.long_url.clone(),
                expires_at: dto.expires_at,
                uuid,
                code: self.code_generator.next_code(),
            };

            tracing::debug!(%attempt, %spec);

            match self.repository.add_one(spec).await {
                Ok(created) => {
                    return Ok(created);
                }
                Err(DatabaseError::Conflict {
                    state,
                    constraint,
                    message,
                }) => {
                    tracing::warn!(
                        %attempt, ?state, %message, "conflict"
                    );
                    let is_code_conflict = matches!(
                        constraint.as_deref(),
                        Some(SHORT_URL_CODE_KEY_CONSTRAINT_NAME)
                    );
                    if is_code_conflict {
                        continue;
                    } else {
                        return Err(ShortUrlError::Storage(DatabaseError::Conflict {
                            state,
                            constraint,
                            message,
                        }));
                    }
                }
                Err(e) => {
                    tracing::error!(%e, "short url insertion error");
                    return Err(ShortUrlError::Storage(e));
                }
            }
        }
        tracing::error!(%dto, "code generation exhausted");
        Err(ShortUrlError::CodeGenerationExhausted)
    }

    pub async fn resolve_redirect_decision(
        &self,
        code: &str,
    ) -> Result<RedirectDecision, ShortUrlError> {
        let cache_result = self.redirect_cache.get(code).await;
        if let Ok(Some(cache_hit)) = cache_result {
            tracing::info!(%code, "cache hit");
            return Ok(cache_hit);
        }

        tracing::info!(%code, "cache miss - checking db");
        let record = self.get_by_code(code).await?;
        match record {
            None => Ok(RedirectDecision::NotFound),
            Some(short) if short.is_deleted() => Ok(RedirectDecision::Gone),
            Some(short) if short.is_expired() => Ok(RedirectDecision::Gone),
            Some(short) if short.expires_at.is_none() => Ok(RedirectDecision::Permanent {
                long_url: short.long_url,
            }),
            Some(short) => Ok(RedirectDecision::Temporary {
                long_url: short.long_url,
            }),
        }
    }
}
