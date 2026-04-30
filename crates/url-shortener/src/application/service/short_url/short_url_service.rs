use std::{ops::Sub, sync::Arc};

use auth::auth_error::AuthError;
use chrono::{TimeDelta, Utc};
use metrics::{counter, gauge};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    application::service::short_url::{
        ValidatedCreateShortUrlRequest, ValidatedUpdateShortUrlRequest,
        code_generator::CodeGenerator, redirect_cache_trait::RedirectCache,
        short_url_service_trait::ShortUrlServiceTrait,
    },
    domain::{
        errors::{RepositoryError, ShortUrlError},
        models::short_url::ShortUrl,
        short_url_spec::ShortUrlSpec,
        traits::{ShortUrlRepositoryTrait, UsersRepositoryTrait},
    },
};

const SHORT_URL_CODE_KEY_CONSTRAINT_NAME: &str = "short_url_code_key";

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Anonymous short URLs (no `user_id`) can be deleted or updated by admins only.
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
            Ok(()) => {
                counter!("short_urls_deleted_total").increment(1);
                gauge!("redirect_cache_size").decrement(1);
                Ok(delete_result)
            }
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
                    counter!("short_urls_created_total", "kind" => "random").increment(1);
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
            Ok(created) => {
                counter!("short_urls_created_total", "kind" => "vanity").increment(1);
                Ok(created)
            }
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
        is_admin: bool,
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

        self.require_owner_or_admin(&short, user_uuid, is_admin)
            .await?;

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
            Ok(updated) => {
                counter!("vanity_urls_updated_total").increment(1);
                Ok(updated)
            }
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
        gauge!("redirect_cache_size").decrement(1);
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
            counter!("redirect_cache_hits_total").increment(1);
            return Ok(cache_hit);
        }

        tracing::info!(%code, "cache miss - checking db");
        counter!("redirect_cache_misses_total").increment(1);

        let record = self.short_url_repository.get_by_code(code).await?;
        match record {
            None => {
                counter!("redirects_total", "result" => "not_found").increment(1);
                Ok(RedirectDecision::NotFound)
            }
            Some(short) if short.is_deleted() => {
                counter!("redirects_total", "result" => "gone").increment(1);
                Ok(RedirectDecision::Gone)
            }
            Some(short) if short.is_expired() => {
                counter!("redirects_total", "result" => "gone").increment(1);
                Ok(RedirectDecision::Gone)
            }
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

                counter!("redirects_total", "result" => "permanent").increment(1);
                gauge!("redirect_cache_size").increment(1);

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

                counter!("redirects_total", "result" => "temporary").increment(1);
                gauge!("redirect_cache_size").increment(1);

                // Cache set is best-effort: failure is logged but does not fail the operation
                Ok(decision)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use auth::auth_error::AuthError;
    use chrono::{Duration, Utc};

    use crate::{
        application::service::short_url::{
            code_generator::FixedCodeGenerator,
            redirect_cache_trait::{NoopRedirectCache, mocks::RecordingRedirectCache},
        },
        domain::{
            models::{short_url::ShortUrl, user::User},
            traits::{
                InMemoryMockShortUrlRepository, InMemoryMockUsersRepository,
                RetryingShortUrlRepository,
            },
        },
    };

    use super::*;

    fn make_user(id: i64, uuid: Uuid) -> User {
        User {
            id,
            uuid,
            username: format!("user-{id}"),
            email: format!("user-{id}@example.com"),
            password_hash: "hash".to_string(),
            password_salt: "salt".to_string(),
            active: true,
            roles: "user".to_string(),
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
        }
    }

    fn make_short(uuid: Uuid, code: &str, user_id: Option<i64>) -> ShortUrl {
        ShortUrl {
            id: 1,
            uuid,
            code: code.to_string(),
            long_url: format!("https://example.com/{code}"),
            expires_at: None,
            user_id,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
        }
    }

    fn make_create_request(
        code: Option<&str>,
        user_uuid: Option<Uuid>,
    ) -> ValidatedCreateShortUrlRequest {
        ValidatedCreateShortUrlRequest {
            long_url: "https://example.com/target".to_string(),
            expires_at: None,
            code: code.map(str::to_string),
            user_uuid,
        }
    }

    fn make_service(
        short_urls: Vec<ShortUrl>,
        users: Vec<User>,
        codes: Vec<&str>,
    ) -> ShortUrlService {
        ShortUrlService::new(
            Arc::new(InMemoryMockShortUrlRepository::new(short_urls)),
            Arc::new(InMemoryMockUsersRepository::new(users)),
            Arc::new(NoopRedirectCache),
            Arc::new(FixedCodeGenerator::new(
                codes.into_iter().map(str::to_string).collect(),
            )),
            3,
        )
    }

    #[tokio::test]
    async fn get_all_returns_empty_correctly() {
        let sut = make_service(vec![], vec![], vec!["generated-code"]);

        let actual = sut.get_all().await.unwrap();

        assert!(actual.is_empty());
    }

    #[tokio::test]
    async fn add_generated_code_succeeds() {
        let sut = make_service(vec![], vec![], vec!["generated-code"]);

        let actual = sut
            .add_generated_code(make_create_request(None, None))
            .await
            .unwrap();

        assert_eq!(actual.code, "generated-code");
        assert_eq!(actual.long_url, "https://example.com/target");
        assert!(actual.user_id.is_none());
    }

    #[tokio::test]
    async fn add_generated_code_succeeds_for_known_user() {
        let user_uuid = Uuid::now_v7();
        let sut = make_service(
            vec![],
            vec![make_user(42, user_uuid)],
            vec!["generated-code"],
        );

        let actual = sut
            .add_generated_code(make_create_request(None, Some(user_uuid)))
            .await
            .unwrap();

        assert_eq!(actual.code, "generated-code");
        assert_eq!(actual.user_id, Some(42));
    }

    #[tokio::test]
    async fn add_generated_code_returns_not_found_for_unknown_user() {
        let sut = make_service(vec![], vec![], vec!["generated-code"]);

        let actual = sut
            .add_generated_code(make_create_request(None, Some(Uuid::now_v7())))
            .await;

        assert!(matches!(actual.unwrap_err(), ShortUrlError::NotFound(..)));
    }

    #[tokio::test]
    async fn add_generated_code_retries_on_code_conflict() {
        let sut = ShortUrlService::new(
            Arc::new(RetryingShortUrlRepository::new(vec!["short_url_code_key"])),
            Arc::new(InMemoryMockUsersRepository::new(vec![])),
            Arc::new(NoopRedirectCache),
            Arc::new(FixedCodeGenerator::new(vec![
                "first-code".to_string(),
                "second-code".to_string(),
            ])),
            2,
        );

        let actual = sut
            .add_generated_code(make_create_request(None, None))
            .await
            .unwrap();

        assert_eq!(actual.code, "second-code");
    }

    #[tokio::test]
    async fn add_generated_code_returns_code_generation_exhausted_after_retries() {
        let sut = ShortUrlService::new(
            Arc::new(RetryingShortUrlRepository::new(vec![
                "short_url_code_key",
                "short_url_code_key",
            ])),
            Arc::new(InMemoryMockUsersRepository::new(vec![])),
            Arc::new(NoopRedirectCache),
            Arc::new(FixedCodeGenerator::new(vec![
                "first-code".to_string(),
                "second-code".to_string(),
            ])),
            2,
        );

        let actual = sut
            .add_generated_code(make_create_request(None, None))
            .await;

        assert!(matches!(
            actual.unwrap_err(),
            ShortUrlError::CodeGenerationExhausted
        ));
    }

    #[tokio::test]
    async fn add_vanity_url_returns_conflict_for_duplicate_code() {
        let existing_uuid = Uuid::now_v7();
        let sut = make_service(
            vec![make_short(existing_uuid, "taken-code", None)],
            vec![],
            vec!["unused-code"],
        );

        let actual = sut
            .add_vanity_url(make_create_request(Some("taken-code"), None))
            .await;

        assert!(matches!(actual.unwrap_err(), ShortUrlError::Conflict(..)));
    }

    #[tokio::test]
    async fn get_by_uuid_returns_added_short_url() {
        let created = make_short(Uuid::now_v7(), "get-by-uuid", None);
        let sut = make_service(vec![created.clone()], vec![], vec!["unused-code"]);

        let actual = sut.get_by_uuid(created.uuid).await.unwrap();

        assert!(actual.is_some());
        let actual = actual.unwrap();
        assert_eq!(actual.uuid, created.uuid);
        assert_eq!(actual.code, created.code);
    }

    #[tokio::test]
    async fn get_by_code_returns_added_short_url() {
        let created = make_short(Uuid::now_v7(), "get-by-code", None);
        let sut = make_service(vec![created.clone()], vec![], vec!["unused-code"]);

        let actual = sut.get_by_code(&created.code).await.unwrap();

        assert!(actual.is_some());
        let actual = actual.unwrap();
        assert_eq!(actual.uuid, created.uuid);
        assert_eq!(actual.code, created.code);
    }

    #[tokio::test]
    async fn delete_one_succeeds_for_owner_and_invalidates_cache() {
        let user_uuid = Uuid::now_v7();
        let user = make_user(7, user_uuid);
        let short = make_short(Uuid::now_v7(), "delete-me", Some(user.id));
        let cache = Arc::new(RecordingRedirectCache::new());
        let sut = ShortUrlService::new(
            Arc::new(InMemoryMockShortUrlRepository::new(vec![short.clone()])),
            Arc::new(InMemoryMockUsersRepository::new(vec![user])),
            cache.clone(),
            Arc::new(FixedCodeGenerator::new(vec!["unused-code".to_string()])),
            3,
        );

        let actual = sut
            .delete_one_by_uuid(short.uuid, user_uuid, false)
            .await
            .unwrap();

        assert!(actual);
        assert_eq!(cache.deleted_codes(), vec!["delete-me".to_string()]);

        let deleted = sut.get_by_uuid(short.uuid).await.unwrap().unwrap();
        assert!(deleted.deleted_at.is_some());
    }

    #[tokio::test]
    async fn delete_one_returns_not_found_for_missing_short_url() {
        let sut = make_service(vec![], vec![], vec!["unused-code"]);

        let actual = sut
            .delete_one_by_uuid(Uuid::now_v7(), Uuid::now_v7(), false)
            .await;

        assert!(matches!(actual.unwrap_err(), ShortUrlError::NotFound(..)));
    }

    #[tokio::test]
    async fn update_one_by_uuid_succeeds_for_owner() {
        let user_uuid = Uuid::now_v7();
        let user = make_user(7, user_uuid);
        let short = make_short(Uuid::now_v7(), "old-code", Some(user.id));
        let cache = Arc::new(RecordingRedirectCache::new());
        let sut = ShortUrlService::new(
            Arc::new(InMemoryMockShortUrlRepository::new(vec![short.clone()])),
            Arc::new(InMemoryMockUsersRepository::new(vec![user])),
            cache.clone(),
            Arc::new(FixedCodeGenerator::new(vec!["unused-code".to_string()])),
            3,
        );

        let actual = sut
            .update_one_by_uuid(
                short.uuid,
                user_uuid,
                false,
                ValidatedUpdateShortUrlRequest {
                    long_url: Some("https://example.com/new-target".to_string()),
                    expires_at: Some(None),
                    code: Some("new-code".to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(actual.long_url, "https://example.com/new-target");
        assert_eq!(actual.code, "new-code");
        assert!(actual.updated_at.is_some());
        assert_eq!(cache.deleted_codes(), vec!["old-code".to_string()]);
    }

    #[tokio::test]
    async fn update_one_by_uuid_returns_unauthorized_for_non_owner() {
        let owner = make_user(7, Uuid::now_v7());
        let short = make_short(Uuid::now_v7(), "old-code", Some(owner.id));
        let sut = make_service(vec![short.clone()], vec![owner], vec!["unused-code"]);

        let actual = sut
            .update_one_by_uuid(
                short.uuid,
                Uuid::now_v7(),
                false,
                ValidatedUpdateShortUrlRequest {
                    long_url: Some("https://example.com/new-target".to_string()),
                    expires_at: None,
                    code: None,
                },
            )
            .await;

        assert!(matches!(
            actual.unwrap_err(),
            ShortUrlError::Unauthorized(AuthError::Forbidden)
        ));
    }

    #[tokio::test]
    async fn resolve_redirect_decision_returns_cache_hit() {
        let cache = Arc::new(RecordingRedirectCache::with_value(
            "cached-code",
            RedirectDecision::Permanent {
                long_url: "https://example.com/cached".to_string(),
            },
        ));
        let sut = ShortUrlService::new(
            Arc::new(InMemoryMockShortUrlRepository::new(vec![])),
            Arc::new(InMemoryMockUsersRepository::new(vec![])),
            cache,
            Arc::new(FixedCodeGenerator::new(vec!["unused-code".to_string()])),
            3,
        );

        let actual = sut.resolve_redirect_decision("cached-code").await.unwrap();

        assert!(matches!(
            actual,
            RedirectDecision::Permanent { ref long_url } if long_url == "https://example.com/cached"
        ));
    }

    #[tokio::test]
    async fn resolve_redirect_decision_returns_permanent_for_non_expiring_short_url() {
        let short = make_short(Uuid::now_v7(), "permanent-code", None);
        let sut = make_service(vec![short], vec![], vec!["unused-code"]);

        let actual = sut
            .resolve_redirect_decision("permanent-code")
            .await
            .unwrap();

        assert!(matches!(
            actual,
            RedirectDecision::Permanent { ref long_url } if long_url == "https://example.com/permanent-code"
        ));
    }

    #[tokio::test]
    async fn resolve_redirect_decision_returns_temporary_for_expiring_short_url() {
        let mut short = make_short(Uuid::now_v7(), "temporary-code", None);
        short.expires_at = Some(Utc::now() + Duration::minutes(5));
        let sut = make_service(vec![short], vec![], vec!["unused-code"]);

        let actual = sut
            .resolve_redirect_decision("temporary-code")
            .await
            .unwrap();

        assert!(matches!(
            actual,
            RedirectDecision::Temporary { ref long_url } if long_url == "https://example.com/temporary-code"
        ));
    }

    #[tokio::test]
    async fn resolve_redirect_decision_returns_gone_for_expired_short_url() {
        let mut short = make_short(Uuid::now_v7(), "expired-code", None);
        short.expires_at = Some(Utc::now() - Duration::minutes(5));
        let sut = make_service(vec![short], vec![], vec!["unused-code"]);

        let actual = sut.resolve_redirect_decision("expired-code").await.unwrap();

        assert!(matches!(actual, RedirectDecision::Gone));
    }

    #[tokio::test]
    async fn resolve_redirect_decision_returns_gone_for_deleted_short_url() {
        let mut short = make_short(Uuid::now_v7(), "deleted-code", None);
        short.deleted_at = Some(Utc::now());
        let sut = make_service(vec![short], vec![], vec!["unused-code"]);

        let actual = sut.resolve_redirect_decision("deleted-code").await.unwrap();

        assert!(matches!(actual, RedirectDecision::Gone));
    }

    #[tokio::test]
    async fn resolve_redirect_decision_returns_not_found_for_missing_code() {
        let sut = make_service(vec![], vec![], vec!["unused-code"]);

        let actual = sut.resolve_redirect_decision("missing-code").await.unwrap();

        assert!(matches!(actual, RedirectDecision::NotFound));
    }
}
