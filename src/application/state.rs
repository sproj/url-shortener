use std::sync::Arc;

use deadpool_postgres::Pool;
use jsonwebtoken::{DecodingKey, EncodingKey};

use crate::application::service::{
    auth::refresh_token_cache_trait::RefreshTokenCacheTrait,
    short_url::{code_generator::CodeGenerator, redirect_cache_trait::RedirectCache},
};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub db_pool: Pool,
    pub code_generator: Arc<dyn CodeGenerator>,
    pub redirect_cache: Arc<dyn RedirectCache>,
    pub refresh_token_cache: Arc<dyn RefreshTokenCacheTrait>,
    pub max_retries: u8,
    pub jwt_encoding_key: EncodingKey,
    pub jwt_decoding_key: DecodingKey,
    pub jwt_access_token_seconds: i64,
    pub jwt_refresh_token_seconds: i64,
}
