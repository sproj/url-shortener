use deadpool_postgres::Pool;

use crate::{
    application::{
        repository::users_repository,
        security::auth::{compare_password_hashes, generate_password_hash, generate_salt},
        service::user::login_params::LoginParams,
    },
    domain::{errors::user_error::UserError, models::user::User},
};

pub async fn verify_login(db_pool: &Pool, params: LoginParams) -> Result<User, UserError> {
    match users_repository::get_user_by_username(db_pool, &params.username).await? {
        Some(user) => {
            let true_hash = &user.password_hash;
            compare_password_hashes(true_hash, params.password)
                .map_err(UserError::AuthenticationError)
                .map(|()| user)
        }
        None => {
            tracing::warn!(%params.username, "login attempt user not found");
            // constant-time dummy work to prevent timing-based enumeration
            let _ = generate_password_hash(params.password.as_bytes(), &generate_salt());
            Err(UserError::NotFound(params.username))
        }
    }
}
