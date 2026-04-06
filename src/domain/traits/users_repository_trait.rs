use std::sync::Mutex;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::{errors::RepositoryError, models::user::User, user_spec::UserSpec};

#[async_trait::async_trait]
pub trait UsersRepositoryTrait: Send + Sync {
    async fn get_all(&self) -> Result<Vec<User>, RepositoryError>;
    async fn get_user_by_uuid(&self, uuid: Uuid) -> Result<Option<User>, RepositoryError>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, RepositoryError>;
    async fn add_user(&self, spec: UserSpec) -> Result<User, RepositoryError>;
    async fn soft_delete_user_by_uuid(&self, uuid: Uuid) -> Result<bool, RepositoryError>;
    async fn update_password_by_uuid(
        &self,
        uuid: Uuid,
        hash: &str,
        salt: &str,
    ) -> Result<bool, RepositoryError>;
}

pub struct InMemoryMockUsersRepository {
    store: Mutex<Vec<User>>,
}

impl InMemoryMockUsersRepository {
    pub fn new(store: Vec<User>) -> Self {
        Self {
            store: Mutex::new(store),
        }
    }
}

#[async_trait::async_trait]
impl UsersRepositoryTrait for InMemoryMockUsersRepository {
    async fn get_all(&self) -> Result<Vec<User>, RepositoryError> {
        let lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        Ok(lock.clone())
    }

    async fn get_user_by_uuid(&self, uuid: Uuid) -> Result<Option<User>, RepositoryError> {
        let lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;
        if let Some(hit) = lock.iter().find(|r| r.uuid == uuid) {
            Ok(Some(hit.clone()))
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, RepositoryError> {
        let lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;
        if let Some(hit) = lock.iter().find(|r| r.username == username) {
            Ok(Some(hit.clone()))
        } else {
            Ok(None)
        }
    }

    async fn add_user(&self, spec: UserSpec) -> Result<User, RepositoryError> {
        let mut lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        let user: User = User {
            id: (lock.len() + 1) as i64,
            uuid: spec.uuid,
            username: spec.username,
            email: spec.email,
            password_hash: spec.password_hash,
            password_salt: spec.password_salt,
            active: true,
            roles: spec.roles,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
        };

        if let Some(duplicate) = lock.iter().find(|r| r.email == user.email) {
            return Err(RepositoryError::Conflict {
                constraint: Some("user_email_constraint".to_string()),
                message: format!(
                    "mock user insert constraint violation with username: {}",
                    duplicate.username
                )
                .to_string(),
            });
        } else {
            lock.push(user.clone());
            return Ok(user.clone());
        }
    }
    async fn soft_delete_user_by_uuid(&self, uuid: Uuid) -> Result<bool, RepositoryError> {
        let mut lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        if let Some(user) = lock.iter_mut().find(|r| r.uuid == uuid) {
            user.deleted_at = Some(Utc::now());
            Ok(true)
        } else {
            Ok(false)
        }
    }
    async fn update_password_by_uuid(
        &self,
        uuid: Uuid,
        hash: &str,
        salt: &str,
    ) -> Result<bool, RepositoryError> {
        let mut lock = self
            .store
            .try_lock()
            .map_err(|e| RepositoryError::Internal(e.to_string()))?;

        if let Some(user) = lock.iter_mut().find(|r| r.uuid == uuid) {
            user.password_hash = hash.to_string();
            user.password_salt = salt.to_string();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
