use std::sync::Arc;

use deadpool_postgres::Pool;

use crate::application::service::{
    short_url::short_url_service::ShortUrlService, user::user_service::UsersService,
};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub short_url: Arc<ShortUrlService>,
    pub users: Arc<UsersService>,
    pub db_pool: Pool,
}
