use uuid::Uuid;

use crate::{
    application::service::user::create_user_params::CreateUserParams,
    domain::{errors::UserError, models::user::User},
};

/// Trait that defines the user service contract, independent of infrastructure.
///
/// Each method mirrors a function in `user_service` with the `Pool` dependency removed.
#[async_trait::async_trait]
pub trait UserServiceTrait: Send + Sync {
    async fn list_all(&self) -> Result<Vec<User>, UserError>;
    async fn get_one_by_uuid(&self, uuid: Uuid) -> Result<Option<User>, UserError>;
    async fn delete_one_by_uuid(&self, user_uuid: Uuid) -> Result<bool, UserError>;
    async fn add_user(&self, params: CreateUserParams) -> Result<User, UserError>;
    async fn update_password_by_uuid(
        &self,
        new_pass: String,
        uuid: Uuid,
    ) -> Result<bool, UserError>;
}
