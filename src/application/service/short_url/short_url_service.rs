use std::{ops::Sub, sync::Arc};

use chrono::{TimeDelta, Utc};
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::{
        repository::{short_url_repository as repository, users_repository},
        security::auth_error::AuthError,
        service::short_url::{
            ShortUrlSpec, ValidatedCreateShortUrlRequest, ValidatedUpdateShortUrlRequest,
            code_generator::CodeGenerator, redirect_cache_trait::RedirectCache,
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

/// Like `get_by_uuid` but enforces that the caller either owns the URL or is an admin.
/// Anonymous short URLs (no `user_id`) are accessible by admins only.
pub async fn get_by_uuid_for_user(
    db_pool: &Pool,
    uuid: Uuid,
    user_uuid: Uuid,
    is_admin: bool,
) -> Result<Option<ShortUrl>, ShortUrlError> {
    let short = match get_by_uuid(db_pool, uuid).await? {
        None => return Ok(None),
        Some(s) => s,
    };
    require_owner_or_admin(db_pool, &short, user_uuid, is_admin).await?;
    Ok(Some(short))
}

pub async fn get_by_code(db_pool: &Pool, code: &str) -> Result<Option<ShortUrl>, DatabaseError> {
    repository::get_by_code(db_pool, code).await
}

pub async fn delete_one_by_uuid(
    db_pool: &Pool,
    redirect_cache: Arc<dyn RedirectCache>,
    uuid: Uuid,
    user_uuid: Uuid,
    is_admin: bool,
) -> Result<bool, ShortUrlError> {
    let rec = match repository::get_by_uuid(db_pool, uuid).await {
        Ok(Some(short)) => short,
        Ok(None) => return Err(ShortUrlError::NotFound(uuid.to_string())),
        Err(e) => return Err(ShortUrlError::from(e)),
    };

    require_owner_or_admin(db_pool, &rec, user_uuid, is_admin).await?;

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

/// Asserts that `user_uuid` owns `short` or is an admin.
/// Anonymous short URLs (no `user_id`) are accessible by admins only.
async fn require_owner_or_admin(
    db_pool: &Pool,
    short: &ShortUrl,
    user_uuid: Uuid,
    is_admin: bool,
) -> Result<(), ShortUrlError> {
    if is_admin {
        return Ok(());
    }

    let owner_db_id = match short.user_id {
        Some(id) => id,
        None => return Err(ShortUrlError::Unauthorized(AuthError::Forbidden)),
    };

    if users_repository::get_user_by_uuid(db_pool, user_uuid)
        .await?
        .is_none_or(|user| user.id != owner_db_id)
    {
        return Err(ShortUrlError::Unauthorized(AuthError::Forbidden));
    }

    Ok(())
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

pub async fn update_one_by_uuid(
    short_uuid: Uuid,
    user_uuid: Uuid,
    dto: ValidatedUpdateShortUrlRequest,
    db_pool: &Pool,
    redirect_cache: Arc<dyn RedirectCache>,
) -> Result<ShortUrl, ShortUrlError> {
    let short = match repository::get_by_uuid(db_pool, short_uuid).await? {
        None => {
            return Err(ShortUrlError::NotFound(
                format!("short url with {short_uuid} not found").to_string(),
            ));
        }
        Some(short) => short,
    };

    let short_owner_db_id = match short.user_id {
        Some(user_db_id) => user_db_id,
        None => return Err(ShortUrlError::Unauthorized(AuthError::Forbidden)),
    };

    let old_code = short.code.clone();

    if users_repository::get_user_by_uuid(db_pool, user_uuid)
        .await?
        .is_none_or(|user| user.id != short_owner_db_id)
    {
        return Err(ShortUrlError::Unauthorized(AuthError::Forbidden));
    }

    let spec = ShortUrlSpec {
        uuid: short.uuid,
        user_id: short.user_id,
        long_url: match dto.long_url {
            Some(new_target) => new_target,
            None => short.long_url,
        },
        expires_at: match dto.expires_at {
            Some(None) => None, // expires_at was explicity set to nothing -> updating redirect to permanent
            Some(Some(new_expiry)) => Some(new_expiry), // expires_at as just pushed to some future date
            None => short.expires_at, // no expires_at input from the user - keep the existing one (if any)
        },
        code: match dto.code {
            Some(new_code) => new_code,
            None => short.code,
        },
    };

    let update_result = match repository::update_one_by_uuid(db_pool, spec).await {
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
    }?;

    redirect_cache.delete(&old_code).await?;

    Ok(update_result)
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
