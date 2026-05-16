use std::sync::Arc;

use deadpool_postgres::Pool;
use jsonwebtoken::DecodingKey;

use crate::application::{
    config::ProxyRoute,
    service::{
        auth::auth_service_trait::AuthServiceTrait, user::user_service_trait::UserServiceTrait,
    },
};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub db_pool: Pool,
    pub jwt_decoding_key: DecodingKey,
    pub user_service: Arc<dyn UserServiceTrait>,
    pub auth_service: Arc<dyn AuthServiceTrait>,
    pub proxy_routes: Vec<ProxyRoute>,
    pub http_client: reqwest::Client,
}
