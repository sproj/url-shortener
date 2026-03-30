use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::{
    application::{
        repository::users_repository as repository,
        security::auth::{compare_password_hashes, generate_password_hash, generate_salt},
        service::user::{
            create_user_params::CreateUserParams, login_params::LoginParams, user_spec::UserSpec,
        },
    },
    domain::{errors::user_error::UserError, models::user::User},
};

pub async fn list_all(pool: &Pool) -> Result<Vec<User>, UserError> {
    repository::get_all(pool).await.map_err(UserError::from)
}

pub async fn get_one_by_uuid(pool: &Pool, uuid: Uuid) -> Result<Option<User>, UserError> {
    repository::get_user_by_uuid(pool, uuid)
        .await
        .map_err(UserError::from)
}

pub async fn delete_one_by_uuid(pool: &Pool, uuid: Uuid) -> Result<bool, UserError> {
    repository::soft_delete_user_by_uuid(pool, uuid)
        .await
        .map_err(UserError::from)
}

pub async fn add_user(pool: &Pool, params: CreateUserParams) -> Result<User, UserError> {
    let spec = UserSpec::try_from(params)?;

    match repository::add_user(pool, spec).await {
        Ok(created) => Ok(created),
        Err(e) => {
            tracing::error!(%e, "create user failed");
            Err(UserError::Storage(e))
        }
    }
}

pub async fn update_password_by_uuid(
    pool: &Pool,
    new_pass: String,
    uuid: Uuid,
) -> Result<bool, UserError> {
    let salt = generate_salt();
    let password_hash = generate_password_hash(new_pass.as_bytes(), &salt)
        .map_err(UserError::AuthenticationError)?;

    repository::update_password_by_uuid(pool, uuid, &password_hash, salt.as_str())
        .await
        .map_err(UserError::Storage)
}

pub async fn verify_login(pool: &Pool, params: LoginParams) -> Result<User, UserError> {
    match repository::get_user_by_username(pool, &params.username).await? {
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
