use std::sync::Arc;

use auth::auth::{generate_password_hash, generate_salt};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    application::service::user::{
        create_user_params::CreateUserParams, user_service_trait::UserServiceTrait,
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
    async fn delete_one_by_uuid(&self, user_uuid: Uuid) -> Result<(), UserError> {
        if self
            .users_repository
            .soft_delete_user_by_uuid(user_uuid)
            .await
            .map_err(UserError::from)?
        {
            Ok(())
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
    async fn update_password_by_uuid(&self, new_pass: String, uuid: Uuid) -> Result<(), UserError> {
        let salt = generate_salt();
        let password_hash = generate_password_hash(new_pass.as_bytes(), &salt)
            .map_err(UserError::AuthenticationError)?;

        match self
            .users_repository
            .update_password_by_uuid(uuid, &password_hash, salt.as_str())
            .await
        {
            Ok(true) => Ok(()),
            Ok(false) => Err(UserError::NotFound(uuid.to_string())),
            Err(e) => Err(UserError::Storage(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{errors::RepositoryError, traits::InMemoryMockUsersRepository};

    use super::*;

    fn make_params(username: &str, email: &str) -> CreateUserParams {
        CreateUserParams {
            username: username.to_string(),
            email: email.to_string(),
            password: "does not matter".to_string(),
            roles: "user".to_string(),
            active: true,
        }
    }

    #[tokio::test]
    async fn list_all_returns_empty_correctly() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let actual = sut.list_all().await.unwrap();
        assert!(actual.is_empty());
    }

    #[tokio::test]
    async fn add_one_succeeds() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = make_params("add_one_succeeds", "add_one@succeeds");

        let actual = sut.add_user(params.clone()).await.unwrap();
        assert_eq!(&actual.username, &params.username);
        assert_eq!(&actual.email, &params.email);
        assert!(!&actual.uuid.is_nil())
    }

    #[tokio::test]
    async fn add_one_fails_on_duplicate_email() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = make_params(
            "add_one_fails_on_duplicate_email",
            "add_one_fails@on_duplicate_email",
        );

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
    async fn list_all_returns_added_users() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let first = make_params("list_all_returns_added_users_1", "list_all_1@example.com");
        let second = make_params("list_all_returns_added_users_2", "list_all_2@example.com");

        let first_added = sut.add_user(first.clone()).await.unwrap();
        let second_added = sut.add_user(second.clone()).await.unwrap();

        let actual = sut.list_all().await.unwrap();

        assert_eq!(actual.len(), 2);
        assert!(actual.iter().any(|user| user.uuid == first_added.uuid));
        assert!(actual.iter().any(|user| user.uuid == second_added.uuid));
    }

    #[tokio::test]
    async fn get_one_by_uuid_returns_added_user() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = make_params(
            "get_one_by_uuid_returns_added_user",
            "get_one_by_uuid@example.com",
        );

        let user = sut.add_user(params.clone()).await.unwrap();

        let actual = sut.get_one_by_uuid(user.uuid).await.unwrap();

        assert!(actual.is_some());
        let actual = actual.unwrap();
        assert_eq!(actual.uuid, user.uuid);
        assert_eq!(actual.username, params.username);
        assert_eq!(actual.email, params.email);
    }

    #[tokio::test]
    async fn get_one_by_uuid_returns_none_for_missing_user() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));

        let actual = sut.get_one_by_uuid(Uuid::now_v7()).await.unwrap();

        assert!(actual.is_none());
    }

    #[tokio::test]
    async fn get_one_by_username_returns_added_user() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = make_params(
            "get_one_by_username_returns_added_user",
            "get_one_by_username@example.com",
        );

        let user = sut.add_user(params.clone()).await.unwrap();

        let actual = sut.get_one_by_username(&params.username).await.unwrap();

        assert!(actual.is_some());
        let actual = actual.unwrap();
        assert_eq!(actual.uuid, user.uuid);
        assert_eq!(actual.username, params.username);
        assert_eq!(actual.email, params.email);
    }

    #[tokio::test]
    async fn get_one_by_username_returns_none_for_missing_user() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));

        let actual = sut
            .get_one_by_username("get_one_by_username_returns_none_for_missing_user")
            .await
            .unwrap();

        assert!(actual.is_none());
    }

    #[tokio::test]
    async fn delete_one_succeeds() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = make_params("delete_one_succeeds", "delete_one_succeeds@example.com");

        let user = sut.add_user(params.clone()).await.unwrap();

        assert!(user.deleted_at.is_none());

        let user_uuid = user.uuid;

        let actual = sut.delete_one_by_uuid(user.uuid).await;

        assert!(actual.is_ok());

        let check = sut.get_one_by_uuid(user_uuid).await.unwrap();
        assert!(check.is_some_and(|usr| usr.deleted_at.is_some()));
    }

    #[tokio::test]
    async fn delete_one_returns_not_found_for_missing_user() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));

        let actual = sut.delete_one_by_uuid(Uuid::now_v7()).await;

        assert!(actual.is_err());
        assert!(matches!(actual.unwrap_err(), UserError::NotFound(..)));
    }

    #[tokio::test]
    async fn update_password_by_uuid_succeeds() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));
        let params = make_params(
            "update_password_by_uuid_succeeds",
            "update_password_by_uuid_succeeds@example.com",
        );

        let user = sut.add_user(params).await.unwrap();
        let original_password_hash = user.password_hash.clone();
        let original_password_salt = user.password_salt.clone();

        let actual = sut
            .update_password_by_uuid("new-password".to_string(), user.uuid)
            .await;

        assert!(actual.is_ok());

        let updated_user = sut.get_one_by_uuid(user.uuid).await.unwrap().unwrap();
        assert_ne!(updated_user.password_hash, original_password_hash);
        assert_ne!(updated_user.password_salt, original_password_salt);
    }

    #[tokio::test]
    async fn update_password_by_uuid_returns_not_found_error_for_missing_user() {
        let sut = UsersService::new(Arc::new(InMemoryMockUsersRepository::new(vec![])));

        let actual = sut
            .update_password_by_uuid("new-password".to_string(), Uuid::now_v7())
            .await;

        assert!(matches!(actual.unwrap_err(), UserError::NotFound(..)));
    }
}
