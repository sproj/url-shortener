use std::fmt::Display;

use serde::Deserialize;
use utoipa::ToSchema;

use crate::application::service::user::login_params::LoginParams;

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

impl From<&LoginRequest> for LoginParams {
    fn from(req: &LoginRequest) -> Self {
        Self {
            username: req.username.clone(),
            password: req.password.clone(),
        }
    }
}

impl Display for LoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "username: {}", self.username)
    }
}
