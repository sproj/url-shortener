use std::{ops::Sub, sync::Arc};

use chrono::{TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    application::{
        security::auth_error::AuthError,
        service::short_url::{
            ValidatedCreateShortUrlRequest, ValidatedUpdateShortUrlRequest,
            code_generator::CodeGenerator, redirect_cache_trait::RedirectCache,
            short_url_service_trait::ShortUrlServiceTrait,
        },
    },
    domain::{
        errors::{RepositoryError, ShortUrlError},
        models::short_url::ShortUrl,
        short_url_spec::ShortUrlSpec,
        traits::{ShortUrlRepositoryTrait, UsersRepositoryTrait},
    },
};

const SHORT_URL_CODE_KEY_CONSTRAINT_NAME: &str = "short_url_code_key";

#[derive(Debug, Serialize, Deserialize)]
pub enum RedirectDecision {
    Permanent { long_url: String },
    Temporary { long_url: String },
    Gone,
    NotFound,
}

pub struct ShortUrlService {
    short_url_repository: Arc<dyn ShortUrlRepositoryTrait>,
    users_repository: Arc<dyn UsersRepositoryTrait>,
    redirect_cache: Arc<dyn RedirectCache>,
    code_generator: Arc<dyn CodeGenerator>,
    max_retries: u8,
}

impl ShortUrlService {
    pub fn new(
        short_url_repository: Arc<dyn ShortUrlRepositoryTrait>,
        users_repository: Arc<dyn UsersRepositoryTrait>,
        redirect_cache: Arc<dyn RedirectCache>,
        code_generator: Arc<dyn CodeGenerator>,
        max_retries: u8,
    ) -> Self {
        Self {
            short_url_repository,
            users_repository,
            redirect_cache,
            code_generator,
            max_retries,
        }
    }

    /// Asserts that `user_uuid` owns `short` or is an admin.
    /// Anonymous short URLs (no `user_id`) are accessible by admins only.
    async fn require_owner_or_admin(
        &self,
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

        if self
            .users_repository
            .get_user_by_uuid(user_uuid)
            .await?
            .is_none_or(|user| user.id != owner_db_id)
        {
            return Err(ShortUrlError::Unauthorized(AuthError::Forbidden));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl ShortUrlServiceTrait for ShortUrlService {
    #[instrument(skip(self))]
    async fn get_all(&self) -> Result<Vec<ShortUrl>, ShortUrlError> {
        self.short_url_repository
            .get_all()
            .await
            .map_err(ShortUrlError::Storage)
    }

    #[instrument(skip(self))]
    async fn get_by_uuid(&self, short_url_uuid: Uuid) -> Result<Option<ShortUrl>, ShortUrlError> {
        self.short_url_repository
            .get_by_uuid(short_url_uuid)
            .await
            .map_err(ShortUrlError::Storage)
    }

    /// Like `get_by_uuid` but enforces that the caller either owns the URL or is an admin.
    /// Anonymous short URLs (no `user_id`) are accessible by admins only.
    #[instrument(skip(self))]
    async fn get_by_uuid_for_user(
        &self,
        short_url_uuid: Uuid,
        user_uuid: Uuid,
        is_admin: bool,
    ) -> Result<Option<ShortUrl>, ShortUrlError> {
        let short = match self
            .short_url_repository
            .get_by_uuid(short_url_uuid)
            .await?
        {
            None => return Ok(None),
            Some(s) => s,
        };
        self.require_owner_or_admin(&short, user_uuid, is_admin)
            .await?;
        Ok(Some(short))
    }

    #[instrument(skip(self))]
    async fn get_by_code(&self, code: &str) -> Result<Option<ShortUrl>, ShortUrlError> {
        self.short_url_repository
            .get_by_code(code)
            .await
            .map_err(ShortUrlError::Storage)
    }

    #[instrument(skip(self))]
    async fn delete_one_by_uuid(
        &self,
        short_url_uuid: Uuid,
        user_uuid: Uuid,
        is_admin: bool,
    ) -> Result<bool, ShortUrlError> {
        let rec = match self.short_url_repository.get_by_uuid(short_url_uuid).await {
            Ok(Some(short)) => short,
            Ok(None) => return Err(ShortUrlError::NotFound(short_url_uuid.to_string())),
            Err(e) => return Err(ShortUrlError::from(e)),
        };

        self.require_owner_or_admin(&rec, user_uuid, is_admin)
            .await?;

        let deleted_code = rec.code;

        tracing::info!(%short_url_uuid, "soft deleting ShortUrl with uuid");
        let delete_result = self
            .short_url_repository
            .delete_one_by_uuid(short_url_uuid)
            .await?;

        tracing::info!(%deleted_code, "removing code from cache");
        match self.redirect_cache.delete(&deleted_code).await {
            Ok(()) => Ok(delete_result),
            Err(e) => {
                tracing::error!(%e, %deleted_code, "Failed to invalidate cache after deleting record");
                Ok(delete_result)
            }
        }
    }

    #[instrument(skip(self))]
    async fn add_generated_code(
        &self,
        dto: ValidatedCreateShortUrlRequest,
    ) -> Result<ShortUrl, ShortUrlError> {
        // uuid is stable across insert attempts. `code` is re-generated on conflict (should be very rare but is possible).
        let uuid = uuid::Uuid::now_v7();

        let mut user_id: Option<i64> = None;
        if let Some(user_uuid) = dto.user_uuid {
            if let Some(user) = self.users_repository.get_user_by_uuid(user_uuid).await? {
                user_id = Some(user.id);
            } else {
                tracing::error!(user_uuid = %user_uuid, "Failed to find user record for owned generated code redirect request");
                return Err(ShortUrlError::NotFound(
                    "failed to find user creating a vanity url".to_string(),
                ));
            }
        }
        for attempt in 1..=self.max_retries {
            let spec = ShortUrlSpec {
                long_url: dto.long_url.clone(),
                expires_at: dto.expires_at,
                uuid,
                code: match dto.code {
                    None => self.code_generator.next_code(),
                    Some(ref vanity_url) => vanity_url.clone(),
                },
                user_id,
            };

            tracing::debug!(%attempt, %spec);

            match self.short_url_repository.add_one(spec).await {
                Ok(created) => {
                    return Ok(created);
                }
                Err(RepositoryError::Conflict {
                    constraint,
                    message,
                }) => {
                    tracing::warn!(
                        %attempt, %message, "conflict"
                    );
                    let is_code_conflict = matches!(
                        constraint.as_deref(),
                        Some(SHORT_URL_CODE_KEY_CONSTRAINT_NAME)
                    );
                    if is_code_conflict {
                        continue;
                    } else {
                        return Err(ShortUrlError::Storage(RepositoryError::Conflict {
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

    #[instrument(skip(self))]
    async fn add_vanity_url(
        &self,
        dto: ValidatedCreateShortUrlRequest,
    ) -> Result<ShortUrl, ShortUrlError> {
        // uuid is stable across insert attempts. `code` is re-generated on conflict (should be very rare but is possible).
        let uuid = uuid::Uuid::now_v7();

        let mut user_id: Option<i64> = None;
        if let Some(user_uuid) = dto.user_uuid {
            if let Some(user) = self.users_repository.get_user_by_uuid(user_uuid).await? {
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
                None => self.code_generator.next_code(),
                Some(ref vanity_url) => vanity_url.clone(),
            },
            user_id,
        };

        tracing::debug!(%spec);

        match self.short_url_repository.add_one(spec).await {
            Ok(created) => Ok(created),
            Err(RepositoryError::Conflict {
                constraint,
                message,
            }) => {
                tracing::warn!(
                    %message, constraint, "conflict on vanity url insertion"
                );
                Err(ShortUrlError::Conflict(message))
            }
            Err(e) => {
                tracing::error!(%e, "short url insertion error");
                Err(ShortUrlError::Storage(e))
            }
        }
    }

    #[instrument(skip(self))]
    async fn update_one_by_uuid(
        &self,
        short_uuid: Uuid,
        user_uuid: Uuid,
        dto: ValidatedUpdateShortUrlRequest,
    ) -> Result<ShortUrl, ShortUrlError> {
        let short = match self.short_url_repository.get_by_uuid(short_uuid).await? {
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

        if self
            .users_repository
            .get_user_by_uuid(user_uuid)
            .await?
            .is_none_or(|user| user.id != short_owner_db_id)
        {
            tracing::error!(%user_uuid, "vanity url update could not find user record of url owner");
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

        let update_result = match self.short_url_repository.update_one_by_uuid(spec).await {
            Ok(created) => Ok(created),
            Err(RepositoryError::Conflict {
                constraint,
                message,
            }) => {
                tracing::warn!(
                    %message, constraint, "conflict on vanity url insertion"
                );
                Err(ShortUrlError::Conflict(message))
            }
            Err(e) => {
                tracing::error!(%e, "short url insertion error");
                Err(ShortUrlError::Storage(e))
            }
        }?;

        self.redirect_cache.delete(&old_code).await.map_err(|e| {
            tracing::error!(%e, "cache deletion of updated url failed");
            e
        })?;

        Ok(update_result)
    }

    #[instrument(skip(self))]
    async fn resolve_redirect_decision(
        &self,
        code: &str,
    ) -> Result<RedirectDecision, ShortUrlError> {
        let cache_result = self.redirect_cache.get(code).await;
        tracing::debug!(?cache_result, "cache result");
        if let Ok(Some(cache_hit)) = cache_result {
            tracing::info!(%code, "cache hit");
            return Ok(cache_hit);
        }

        tracing::info!(%code, "cache miss - checking db");
        let record = self.short_url_repository.get_by_code(code).await?;
        match record {
            None => Ok(RedirectDecision::NotFound),
            Some(short) if short.is_deleted() => Ok(RedirectDecision::Gone),
            Some(short) if short.is_expired() => Ok(RedirectDecision::Gone),
            Some(short) if short.expires_at.is_none() => {
                let decision = RedirectDecision::Permanent {
                    long_url: short.long_url,
                };
                if let Err(e) = self
                    .redirect_cache
                    .set(code, &decision, std::time::Duration::from_secs(3600 * 6))
                    .await
                {
                    tracing::error!(%e, ?decision, "failed to write redirect decision to cache");
                }
                // Cache set is best-effort: failure is logged but does not fail the operation
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
                if let Err(e) = self
                    .redirect_cache
                    .set(
                        code,
                        &decision,
                        std::time::Duration::from_secs(expires_seconds.num_seconds() as u64),
                    )
                    .await
                {
                    tracing::error!(%e, ?decision, "failed to write redirect decision to cache");
                }
                // Cache set is best-effort: failure is logged but does not fail the operation
                Ok(decision)
            }
        }
    }
}
