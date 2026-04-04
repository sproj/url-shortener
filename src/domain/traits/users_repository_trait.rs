use uuid::Uuid;

use crate::domain::{errors::user_error::UserError, models::user::User, user_spec::UserSpec};

#[async_trait::async_trait]
pub trait UsersRepositoryTrait: Send + Sync {
    async fn get_all(&self) -> Result<Vec<User>, UserError>;
    async fn get_user_by_uuid(&self, uuid: Uuid) -> Result<Option<User>, UserError>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, UserError>;
    async fn add_user(&self, spec: UserSpec) -> Result<User, UserError>;
    async fn soft_delete_user_by_uuid(&self, uuid: Uuid) -> Result<bool, UserError>;
    async fn update_password_by_uuid(
        &self,
        uuid: Uuid,
        hash: &str,
        salt: &str,
    ) -> Result<bool, UserError>;
}
