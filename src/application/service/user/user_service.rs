use argon2::{
    Argon2,
    password_hash::{Error as HashError, PasswordHasher, SaltString},
};
use rand_core::OsRng;

use std::sync::Arc;
use uuid::{Uuid, uuid};

use crate::{
    application::{
        repository::users_repository::UserRepository, service::user::user_spec::UserSpec,
    },
    domain::{errors::user_error::UserError, models::user::User},
    infrastructure::database::database_error::DatabaseError,
};

pub struct UserService {
    repository: Arc<UserRepository>,
}

impl UserService {
    pub async fn list_all(&self) -> Result<Vec<User>, DatabaseError> {
        self.repository.get_all().await
    }

    pub async fn get_one_by_uuid(&self, uuid: Uuid) -> Result<Option<User>, DatabaseError> {
        self.repository.get_user_by_uuid(uuid).await
    }

    pub async fn delete_one_by_uuid(&self, uuid: Uuid) -> Result<bool, DatabaseError> {
        self.repository.delete_user_by_uuid(uuid).await
    }

    pub async fn add_user(&self) -> Result<User, UserError> {
        let salt = SaltString::generate(&mut OsRng);

        let spec = UserSpec {
            // uuid: Uuid::new_v4(),
            uuid: uuid!("0ca4906b-15f5-4365-841d-07e5eb431ef1"),
            username: "admin".to_string(),
            email: "admin@admin.com".to_string(),
            password_salt: salt.to_string(),
            password_hash: Self::generate_password_hash("pass1234".as_bytes(), &salt)
                .map_err(UserError::HashingError)?,
            active: true,
            roles: "admin".to_string(),
        };

        self.repository
            .add_user(spec)
            .await
            .map_err(UserError::Storage)
    }

    pub async fn update_password_by_uuid(
        &self,
        new_pass: String,
        uuid: Uuid,
    ) -> Result<bool, UserError> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Self::generate_password_hash(new_pass.as_bytes(), &salt)
            .map_err(UserError::HashingError)?;

        self.repository
            .update_password_by_uuid(uuid, &password_hash, salt.as_str())
            .await
            .map_err(UserError::Storage)
    }

    fn generate_password_hash(pw: &[u8], salt: &SaltString) -> Result<String, HashError> {
        let argon2 = Argon2::default();

        let password_hash = argon2.hash_password(pw, salt)?.to_string();
        Ok(password_hash)
    }
}
