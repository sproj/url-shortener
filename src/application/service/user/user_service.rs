use argon2::{
    Argon2,
    password_hash::{Error as HashError, PasswordHasher, SaltString},
};
use deadpool_postgres::Pool;
use rand_core::OsRng;

use std::sync::Arc;
use uuid::Uuid;

use crate::{
    application::{
        repository::users_repository::UsersRepository,
        service::user::{create_user_params::CreateUserParams, user_spec::UserSpec},
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
        let salt = SaltString::generate(&mut OsRng);
        let password_hash =
            generate_password_hash(new_pass.as_bytes(), &salt).map_err(UserError::HashingError)?;

        self.repository
            .update_password_by_uuid(uuid, &password_hash, salt.as_str())
            .await
            .map_err(UserError::Storage)
    }
}

pub(crate) fn generate_salt() -> SaltString {
    SaltString::generate(&mut OsRng)
}

pub(crate) fn generate_password_hash(pw: &[u8], salt: &SaltString) -> Result<String, HashError> {
    let argon2 = Argon2::default();

    let password_hash = argon2.hash_password(pw, salt)?.to_string();
    Ok(password_hash)
}
