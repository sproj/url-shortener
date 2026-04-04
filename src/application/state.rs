use std::sync::Arc;

use deadpool_postgres::Pool;
use jsonwebtoken::DecodingKey;

use crate::application::service::{
    auth::{
        auth_service_trait::AuthServiceTrait, refresh_token_cache_trait::RefreshTokenCacheTrait,
    },
    short_url::{
        code_generator::CodeGenerator, redirect_cache_trait::RedirectCache,
        short_url_service_trait::ShortUrlServiceTrait,
    },
    user::user_service_trait::UserServiceTrait,
};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub db_pool: Pool,
    pub code_generator: Arc<dyn CodeGenerator>,
    pub redirect_cache: Arc<dyn RedirectCache>,
    pub refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
    pub jwt_decoding_key: DecodingKey,
    pub user_service: Arc<dyn UserServiceTrait>,
    pub short_url_service: Arc<dyn ShortUrlServiceTrait>,
    pub auth_service: Arc<dyn AuthServiceTrait>,
}
