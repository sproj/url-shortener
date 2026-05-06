use common::{events::redirect_event::RedirectEvent, repository_error::RepositoryError};

#[async_trait::async_trait]
pub trait RedirectRepositoryTrait: Send + Sync {
    async fn save(&self, event: &RedirectEvent) -> Result<(), RepositoryError>;
}
