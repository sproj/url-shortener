use std::sync::Arc;

use deadpool_postgres::Pool;
use jsonwebtoken::DecodingKey;

use crate::application::service::{
    analytics::analytics_publisher_trait::AnalyticsPublisherTrait,
    short_url::short_url_service_trait::ShortUrlServiceTrait,
};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub db_pool: Pool,
    pub jwt_decoding_key: DecodingKey,
    pub short_url_service: Arc<dyn ShortUrlServiceTrait>,
    pub analytics_publisher: Arc<dyn AnalyticsPublisherTrait>,
}
