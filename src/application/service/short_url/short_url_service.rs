use std::{ops::Sub, sync::Arc};

use chrono::{TimeDelta, Utc};
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::handlers::short_url::ValidatedCreateShortUrlRequest,
    application::{
        repository::{short_url_repository as repository, users_repository},
        service::short_url::{
            ShortUrlSpec, code_generator::CodeGenerator, redirect_cache_trait::RedirectCache,
        },
    },
    domain::{errors::ShortUrlError, models::short_url::ShortUrl},
    infrastructure::database::database_error::DatabaseError,
};

const SHORT_URL_CODE_KEY_CONSTRAINT_NAME: &str = "short_url_code_key";

#[derive(Debug, Serialize, Deserialize)]
pub enum RedirectDecision {
    Permanent { long_url: String },
    Temporary { long_url: String },
    Gone,
    NotFound,
}

pub async fn get_all(pool: &Pool) -> Result<Vec<ShortUrl>, DatabaseError> {
    repository::get_all(pool).await
}

pub async fn get_by_uuid(db_pool: &Pool, uuid: Uuid) -> Result<Option<ShortUrl>, ShortUrlError> {
    repository::get_by_uuid(db_pool, uuid)
        .await
        .map_err(ShortUrlError::Storage)
}

pub async fn get_by_code(db_pool: &Pool, code: &str) -> Result<Option<ShortUrl>, DatabaseError> {
    repository::get_by_code(db_pool, code).await
}

pub async fn delete_one_by_uuid(
    db_pool: &Pool,
    redirect_cache: Arc<dyn RedirectCache>,
    uuid: Uuid,
) -> Result<bool, ShortUrlError> {
    let rec = match repository::get_by_uuid(db_pool, uuid).await {
        Ok(Some(short)) => short,
        Ok(None) => return Err(ShortUrlError::NotFound(uuid.to_string())),
        Err(e) => return Err(ShortUrlError::from(e)),
    };

    let deleted_code = rec.code;

    tracing::info!(%uuid, "soft deleting ShortUrl with uuid");
    let delete_result = repository::delete_one_by_uuid(db_pool, uuid).await?;

    tracing::info!(%deleted_code, "removing code from cache");
    match redirect_cache.delete(&deleted_code).await {
        Ok(()) => Ok(delete_result),
        Err(e) => {
            tracing::error!(%e, "Failed to invalidate cache after deleting record");
            Ok(delete_result)
        }
    }
}

pub async fn add_generated_code(
    db_pool: &Pool,
    code_generator: Arc<dyn CodeGenerator>,
    max_retries: u8,
    dto: ValidatedCreateShortUrlRequest,
) -> Result<ShortUrl, ShortUrlError> {
    // uuid is stable across insert attempts. `code` is re-generated on conflict (should be very rare but is possible).
    let uuid = uuid::Uuid::now_v7();

    let mut user_id: Option<i64> = None;
    if let Some(user_uuid) = dto.user_uuid {
        if let Some(user) = users_repository::get_user_by_uuid(db_pool, user_uuid).await? {
            user_id = Some(user.id);
        } else {
            return Err(ShortUrlError::NotFound(
                "failed to find user creating a vanity url".to_string(),
            ));
        }
    }
    for attempt in 1..=max_retries {
        let spec = ShortUrlSpec {
            long_url: dto.long_url.clone(),
            expires_at: dto.expires_at,
            uuid,
            code: match dto.code {
                None => code_generator.next_code(),
                Some(ref vanity_url) => vanity_url.clone(),
            },
            user_id,
        };

        tracing::debug!(%attempt, %spec);

        match repository::add_one(db_pool, spec).await {
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

pub async fn add_vanity_url(
    db_pool: &Pool,
    code_generator: Arc<dyn CodeGenerator>,
    dto: ValidatedCreateShortUrlRequest,
) -> Result<ShortUrl, ShortUrlError> {
    // uuid is stable across insert attempts. `code` is re-generated on conflict (should be very rare but is possible).
    let uuid = uuid::Uuid::now_v7();

    let mut user_id: Option<i64> = None;
    if let Some(user_uuid) = dto.user_uuid {
        if let Some(user) = users_repository::get_user_by_uuid(db_pool, user_uuid).await? {
            user_id = Some(user.id);
        } else {
            return Err(ShortUrlError::NotFound(
                "failed to find user creating a vanity url".to_string(),
            ));
        }
    }

    let spec = ShortUrlSpec {
        long_url: dto.long_url.clone(),
        expires_at: dto.expires_at,
        uuid,
        code: match dto.code {
            None => code_generator.next_code(),
            Some(ref vanity_url) => vanity_url.clone(),
        },
        user_id,
    };

    tracing::debug!(%spec);

    match repository::add_one(db_pool, spec).await {
        Ok(created) => Ok(created),
        Err(DatabaseError::Conflict {
            state,
            constraint,
            message,
        }) => {
            tracing::warn!(
                ?state, %message, constraint, "conflict on vanity url insertion"
            );
            Err(ShortUrlError::Conflict(message))
        }
        Err(e) => {
            tracing::error!(%e, "short url insertion error");
            Err(ShortUrlError::Storage(e))
        }
    }
}

pub async fn resolve_redirect_decision(
    db_pool: &Pool,
    redirect_cache: Arc<dyn RedirectCache>,
    code: &str,
) -> Result<RedirectDecision, ShortUrlError> {
    let cache_result = redirect_cache.get(code).await;
    tracing::debug!(?cache_result, "cache result");
    if let Ok(Some(cache_hit)) = cache_result {
        tracing::info!(%code, "cache hit");
        return Ok(cache_hit);
    }

    tracing::info!(%code, "cache miss - checking db");
    let record = repository::get_by_code(db_pool, code).await?;
    match record {
        None => Ok(RedirectDecision::NotFound),
        Some(short) if short.is_deleted() => Ok(RedirectDecision::Gone),
        Some(short) if short.is_expired() => Ok(RedirectDecision::Gone),
        Some(short) if short.expires_at.is_none() => {
            let decision = RedirectDecision::Permanent {
                long_url: short.long_url,
            };
            if let Err(e) = redirect_cache
                .set(code, &decision, std::time::Duration::from_secs(3600 * 6))
                .await
            {
                tracing::error!(%e, ?decision, "failed to write redirect decision to cache");
            }
            Ok(decision)
        }
        Some(short) => {
            let decision = RedirectDecision::Temporary {
                long_url: short.long_url,
            };
            let expires_seconds = match short.expires_at {
                Some(time) => time
                    .sub(Utc::now())
                    .clamp(TimeDelta::seconds(1), TimeDelta::minutes(15)),
                None => TimeDelta::minutes(15),
            };
            if let Err(e) = redirect_cache
                .set(
                    code,
                    &decision,
                    std::time::Duration::from_secs(expires_seconds.num_seconds() as u64),
                )
                .await
            {
                tracing::error!(%e, ?decision, "failed to write redirect decision to cache");
            }
            Ok(decision)
        }
    }
}
