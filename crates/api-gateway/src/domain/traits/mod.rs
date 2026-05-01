mod users_repository_trait;
pub use users_repository_trait::UsersRepositoryTrait;

#[cfg(test)]
pub use users_repository_trait::mocks::InMemoryMockUsersRepository;
