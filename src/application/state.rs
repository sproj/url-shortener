use std::sync::Arc;

use deadpool_postgres::Pool;

pub type SharedState = Arc<AppState>;
pub struct AppState {
    pub db_pool: Pool,
}
