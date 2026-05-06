use crate::application::startup_error::StartupError;

#[async_trait::async_trait]
pub trait AnalyticsServiceTrait: Send + Sync {
    async fn run(&self) -> Result<(), StartupError>;
}
