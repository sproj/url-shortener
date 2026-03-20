use serde::Deserialize;

use crate::application::service::user::create_user_params::CreateUserParams;

#[derive(Debug, Deserialize)]
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
