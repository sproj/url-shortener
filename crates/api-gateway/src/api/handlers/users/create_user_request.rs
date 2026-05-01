use std::fmt::Debug;

use serde::Deserialize;
use utoipa::ToSchema;

use crate::application::service::user::create_user_params::CreateUserParams;

#[derive(Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl From<CreateUserRequest> for CreateUserParams {
    fn from(req: CreateUserRequest) -> Self {
        Self {
            active: true,
            roles: "user".to_string(),
            username: req.username,
            email: req.email,
            password: req.password,
        }
    }
}

impl Debug for CreateUserRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "username: {}, email: {}", self.username, self.email)
    }
}
