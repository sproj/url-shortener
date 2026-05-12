use crate::application::consume_result::ConsumeResult;

#[async_trait::async_trait]
pub trait AnalyticsConsumerTrait: Send + Sync {
    async fn next(&self) -> ConsumeResult;
}
