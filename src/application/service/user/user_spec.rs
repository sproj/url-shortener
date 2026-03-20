use std::fmt::Display;
use uuid::Uuid;

use crate::{
    application::service::user::{
        create_user_params::CreateUserParams,
        user_service::{generate_password_hash, generate_salt},
    },
    domain::errors::user_error::UserError,
};

#[derive(Debug)]
pub struct UserSpec {
    pub uuid: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub password_salt: String,
    pub active: bool,
    pub roles: String,
}

impl Display for UserSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "uuid: {}, username: {}, email: {}, active: {}, roles: {}",
            self.uuid, self.username, self.email, self.active, self.roles
        )
    }
}

impl TryFrom<CreateUserParams> for UserSpec {
    type Error = UserError;

    fn try_from(params: CreateUserParams) -> Result<Self, Self::Error> {
        let password_salt = generate_salt();
        let password_hash = generate_password_hash(params.password.as_bytes(), &password_salt)
            .map_err(UserError::HashingError)?;

        Ok(Self {
            uuid: Uuid::now_v7(),
            username: params.username,
            email: params.email,
            active: params.active,
            roles: params.roles,
            password_salt: password_salt.to_string(),
            password_hash,
        })
    }
}
