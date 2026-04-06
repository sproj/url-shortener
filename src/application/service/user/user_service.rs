use std::sync::Arc;

use tracing::instrument;
use uuid::Uuid;

use crate::{
    application::{
        security::auth::{generate_password_hash, generate_salt},
        service::user::{
            create_user_params::CreateUserParams, user_service_trait::UserServiceTrait,
        },
    },
    domain::{
        errors::UserError, models::user::User, traits::UsersRepositoryTrait, user_spec::UserSpec,
    },
};

pub struct UsersService {
    users_repository: Arc<dyn UsersRepositoryTrait>,
}

impl UsersService {
    pub fn new(users_repository: Arc<dyn UsersRepositoryTrait>) -> Self {
        Self { users_repository }
    }
}

#[async_trait::async_trait]
impl UserServiceTrait for UsersService {
    #[instrument(skip(self))]
    async fn list_all(&self) -> Result<Vec<User>, UserError> {
        self.users_repository
            .get_all()
            .await
            .map_err(UserError::from)
    }

    #[instrument(skip(self))]
    async fn get_one_by_uuid(&self, uuid: Uuid) -> Result<Option<User>, UserError> {
        self.users_repository
            .get_user_by_uuid(uuid)
            .await
            .map_err(UserError::from)
    }

    #[instrument(skip(self))]
    async fn get_one_by_username(&self, user_name: &str) -> Result<Option<User>, UserError> {
        self.users_repository
            .get_user_by_username(user_name)
            .await
            .map_err(UserError::from)
    }

    #[instrument(skip(self))]
    async fn delete_one_by_uuid(&self, user_uuid: Uuid) -> Result<bool, UserError> {
        if self
            .users_repository
            .soft_delete_user_by_uuid(user_uuid)
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

    #[instrument(skip(self), fields(username = %params.username, email = %params.email, roles = %params.roles))]
    async fn add_user(&self, params: CreateUserParams) -> Result<User, UserError> {
        let spec = UserSpec::try_from(params)?;

        match self.users_repository.add_user(spec).await {
            Ok(created) => Ok(created),
            Err(e) => {
                tracing::error!(%e, "create user failed");
                Err(UserError::Storage(e))
            }
        }
    }

    #[instrument(skip(self, new_pass))]
    async fn update_password_by_uuid(
        &self,
        new_pass: String,
        uuid: Uuid,
    ) -> Result<bool, UserError> {
        let salt = generate_salt();
        let password_hash = generate_password_hash(new_pass.as_bytes(), &salt)
            .map_err(UserError::AuthenticationError)?;

        self.users_repository
            .update_password_by_uuid(uuid, &password_hash, salt.as_str())
            .await
            .map_err(UserError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{errors::RepositoryError, traits::InMemoryMockUsersRepository};

    use super::*;

    #[tokio::test]
    async fn list_all_returns_empty_correctly() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let actual = sut.list_all().await.unwrap();
        assert!(actual.is_empty());
    }

    #[tokio::test]
    async fn add_one_succeeds() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = CreateUserParams {
            username: "add_one_succeeds".to_string(),
            email: "add_one@succeeds".to_string(),
            password: "does not matter".to_string(),
            roles: "user".to_string(),
            active: true,
        };

        let actual = sut.add_user(params.clone()).await.unwrap();
        assert_eq!(&actual.username, &params.username);
        assert_eq!(&actual.email, &params.email);
        assert!(!&actual.uuid.is_nil())
    }

    #[tokio::test]
    async fn add_one_fails_on_duplicate_username() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = CreateUserParams {
            username: "add_one_fails_on_duplicate_username".to_string(),
            email: "add_one_fails@on_duplicate_username".to_string(),
            password: "does not matter".to_string(),
            roles: "user".to_string(),
            active: true,
        };

        let add_user = sut.add_user(params.clone()).await.unwrap();
        assert_eq!(&add_user.username, &params.username);
        assert_eq!(&add_user.email, &params.email);
        assert!(!&add_user.uuid.is_nil());

        let actual = sut.add_user(params.clone()).await;

        assert!(actual.is_err());

        let err = actual.unwrap_err();

        assert!(matches!(
            err,
            UserError::Storage(RepositoryError::Conflict { .. })
        ))
    }

    #[tokio::test]
    async fn delete_one_succeeds() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = CreateUserParams {
            username: "add_one_succeeds".to_string(),
            email: "add_one@succeeds".to_string(),
            password: "does not matter".to_string(),
            roles: "user".to_string(),
            active: true,
        };

        let user = sut.add_user(params.clone()).await.unwrap();

        assert!(user.deleted_at.is_none());

        let user_uuid = user.uuid;

        let actual = sut.delete_one_by_uuid(user.uuid).await.unwrap();

        assert!(actual);

        let check = sut.get_one_by_uuid(user_uuid).await.unwrap();
        assert!(check.is_some_and(|usr| usr.deleted_at.is_some()));
    }
}
