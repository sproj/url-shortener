use uuid::Uuid;

use crate::domain::{
    errors::ShortUrlError, models::short_url::ShortUrl, short_url_spec::ShortUrlSpec,
};

#[async_trait::async_trait]
pub trait ShortUrlRepositoryTrait: Send + Sync {
    async fn get_all(&self) -> Result<Vec<ShortUrl>, ShortUrlError>;
    async fn get_by_uuid(&self, uuid: Uuid) -> Result<Option<ShortUrl>, ShortUrlError>;
    /// Looks up by redirect code. Does NOT filter soft-deleted records — callers check deletion.
    async fn get_by_code(&self, code: &str) -> Result<Option<ShortUrl>, ShortUrlError>;
    async fn add_one(&self, spec: ShortUrlSpec) -> Result<ShortUrl, ShortUrlError>;
    async fn update_one_by_uuid(&self, spec: ShortUrlSpec) -> Result<ShortUrl, ShortUrlError>;
    async fn delete_one_by_uuid(&self, uuid: Uuid) -> Result<bool, ShortUrlError>;
}
