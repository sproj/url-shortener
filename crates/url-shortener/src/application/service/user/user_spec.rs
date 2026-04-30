use crate::{
    application::{
        security::auth::{generate_password_hash, generate_salt},
        service::user::create_user_params::CreateUserParams,
    },
    domain::{errors::UserError, user_spec::UserSpec},
};

impl TryFrom<CreateUserParams> for UserSpec {
    type Error = UserError;

    fn try_from(params: CreateUserParams) -> Result<Self, Self::Error> {
        let password_salt = generate_salt();
        let password_hash = generate_password_hash(params.password.as_bytes(), &password_salt)
            .map_err(UserError::AuthenticationError)?;

        Ok(Self {
            uuid: uuid::Uuid::now_v7(),
            username: params.username,
            email: params.email,
            active: params.active,
            roles: params.roles,
            password_salt: password_salt.to_string(),
            password_hash,
        })
    }
}
