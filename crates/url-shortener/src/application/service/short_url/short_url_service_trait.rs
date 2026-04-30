use uuid::Uuid;

use crate::{
    application::service::short_url::{
        ValidatedCreateShortUrlRequest, ValidatedUpdateShortUrlRequest,
        short_url_service::RedirectDecision,
    },
    domain::{errors::ShortUrlError, models::short_url::ShortUrl},
};

/// Trait that defines the short-URL service contract, independent of infrastructure.
///
/// Each method mirrors a function in `short_url_service` with pool, cache, and generator
/// dependencies removed — those are held by the concrete implementation struct.
///
/// Returning `ShortUrlError` throughout (the two thin pass-throughs in the current free
/// functions that returned `DatabaseError` directly are intentionally unified here).
#[async_trait::async_trait]
pub trait ShortUrlServiceTrait: Send + Sync {
    async fn get_all(&self) -> Result<Vec<ShortUrl>, ShortUrlError>;
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ShortUrl>, ShortUrlError>;
    async fn get_by_code(&self, code: &str) -> Result<Option<ShortUrl>, ShortUrlError>;
    async fn delete_one_by_uuid(
        &self,
        uuid: Uuid,
        user_uuid: Uuid,
        is_admin: bool,
    ) -> Result<bool, ShortUrlError>;
    /// Creates a short URL with a generated code. `max_retries` is a service-level config
    /// value held by the concrete struct, not passed per call.
    async fn add_generated_code(
        &self,
        dto: ValidatedCreateShortUrlRequest,
    ) -> Result<ShortUrl, ShortUrlError>;
    async fn add_vanity_url(
        &self,
        dto: ValidatedCreateShortUrlRequest,
    ) -> Result<ShortUrl, ShortUrlError>;
    async fn update_one_by_uuid(
        &self,
        short_uuid: Uuid,
        user_uuid: Uuid,
        is_admin: bool,
        dto: ValidatedUpdateShortUrlRequest,
    ) -> Result<ShortUrl, ShortUrlError>;
    async fn resolve_redirect_decision(
        &self,
        code: &str,
    ) -> Result<RedirectDecision, ShortUrlError>;
}
