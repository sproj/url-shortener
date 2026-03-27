use std::sync::Arc;

use deadpool_postgres::Pool;
use jsonwebtoken::{DecodingKey, EncodingKey};

use crate::application::service::{
    short_url::short_url_service::ShortUrlService, user::user_service::UsersService,
};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub short_url: Arc<ShortUrlService>,
    pub users: Arc<UsersService>,
    pub db_pool: Pool,
    pub jwt_encoding_key: Arc<EncodingKey>,
    pub jwt_decoding_key: Arc<DecodingKey>,
    pub jwt_access_token_seconds: i64,
}
