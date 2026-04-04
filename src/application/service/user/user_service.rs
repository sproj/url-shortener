use deadpool_postgres::Pool;
use uuid::Uuid;

use crate::{
    application::{
        repository::users_repository as repository,
        security::auth::{generate_password_hash, generate_salt},
        service::user::create_user_params::CreateUserParams,
    },
    domain::{errors::user_error::UserError, models::user::User, user_spec::UserSpec},
};

pub async fn list_all(pool: &Pool) -> Result<Vec<User>, UserError> {
    repository::get_all(pool).await.map_err(UserError::from)
}

pub async fn get_one_by_uuid(pool: &Pool, uuid: Uuid) -> Result<Option<User>, UserError> {
    repository::get_user_by_uuid(pool, uuid)
        .await
        .map_err(UserError::from)
}

pub async fn delete_one_by_uuid(pool: &Pool, user_uuid: Uuid) -> Result<bool, UserError> {
    if repository::soft_delete_user_by_uuid(pool, user_uuid)
        .await
        .map_err(UserError::from)?
    {
        Ok(true)
    } else {
        tracing::warn!(%user_uuid, "deletion attempted for user with unfound uuid");
        Err(UserError::NotFound(format!(
            "user with uuid {} not found",
            user_uuid
        )))
    }
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
