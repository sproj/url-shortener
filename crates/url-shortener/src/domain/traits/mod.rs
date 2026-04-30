mod short_url_repository_trait;
mod users_repository_trait;

pub use short_url_repository_trait::ShortUrlRepositoryTrait;
pub use users_repository_trait::UsersRepositoryTrait;

#[cfg(test)]
pub use short_url_repository_trait::mocks::{
    InMemoryMockShortUrlRepository, RetryingShortUrlRepository,
};
#[cfg(test)]
pub use users_repository_trait::mocks::InMemoryMockUsersRepository;
