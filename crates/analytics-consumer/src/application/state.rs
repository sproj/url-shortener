use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::application::service::analytics_service_trait::AnalyticsServiceTrait;

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub db_pool: Pool,
    pub analytics: Arc<dyn AnalyticsServiceTrait>,
}
