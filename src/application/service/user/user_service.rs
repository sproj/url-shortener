use deadpool_postgres::Pool;

use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        repository::users_repository::UsersRepository,
        security::auth::{compare_password_hashes, generate_password_hash, generate_salt},
        service::user::{
            create_user_params::CreateUserParams, login_params::LoginParams, user_spec::UserSpec,
        },
    },
    domain::{errors::user_error::UserError, models::user::User},
};

pub struct UsersService {
    repository: Arc<UsersRepository>,
}

impl UsersService {
    pub fn new(pool: Pool) -> Self {
        Self {
            repository: Arc::new(UsersRepository::new(pool)),
        }
    }
    pub async fn list_all(&self) -> Result<Vec<User>, UserError> {
        self.repository.get_all().await.map_err(UserError::from)
    }

    pub async fn get_one_by_uuid(&self, uuid: Uuid) -> Result<Option<User>, UserError> {
        self.repository
            .get_user_by_uuid(uuid)
            .await
            .map_err(UserError::from)
    }

    pub async fn delete_one_by_uuid(&self, uuid: Uuid) -> Result<bool, UserError> {
        self.repository
            .soft_delete_user_by_uuid(uuid)
            .await
            .map_err(UserError::from)
    }

    pub async fn add_user(&self, params: CreateUserParams) -> Result<User, UserError> {
        let spec = UserSpec::try_from(params)?;

        match self.repository.add_user(spec).await {
            Ok(created) => Ok(created),
            Err(e) => {
                tracing::error!(%e, "create user failed");
                Err(UserError::Storage(e))
            }
        }
    }

    pub async fn update_password_by_uuid(
        &self,
        new_pass: String,
        uuid: Uuid,
    ) -> Result<bool, UserError> {
        let salt = generate_salt();
        let password_hash = generate_password_hash(new_pass.as_bytes(), &salt)
            .map_err(UserError::AuthenticationError)?;

        self.repository
            .update_password_by_uuid(uuid, &password_hash, salt.as_str())
            .await
            .map_err(UserError::Storage)
    }

    pub async fn verify_login(&self, params: LoginParams) -> Result<User, UserError> {
        match self
            .repository
            .get_user_by_username(&params.username)
            .await?
        {
            Some(user) => {
                let true_hash = &user.password_hash;
                match compare_password_hashes(true_hash, params.password)
                    .map_err(UserError::AuthenticationError)
                {
                    Ok(()) => return Ok(user),
                    Err(e) => return Err(e),
                }
            }
            None => {
                tracing::warn!(%params.username, "login attempt user not found");
                // constant-time dummy work to prevent timing-based enumeration
                let _ = generate_password_hash(params.password.as_bytes(), &generate_salt());
                Err(UserError::NotFound(params.username))
            }
        }
    }
}
