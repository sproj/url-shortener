use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::application::service::short_url::short_url_service::ShortUrlService;

pub type SharedState = Arc<AppState>;
pub struct AppState {
    pub short_url: Arc<ShortUrlService>,
    pub db_pool: Pool,
}
