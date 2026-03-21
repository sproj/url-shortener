use serde::Deserialize;

use crate::application::service::user::login_params::LoginParams;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

impl From<LoginRequest> for LoginParams {
    fn from(req: LoginRequest) -> Self {
        Self {
            username: req.username,
            password: req.password,
        }
    }
}
