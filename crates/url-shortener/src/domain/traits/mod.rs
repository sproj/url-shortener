mod short_url_repository_trait;

pub use short_url_repository_trait::ShortUrlRepositoryTrait;

#[cfg(test)]
pub use short_url_repository_trait::mocks::{
    InMemoryMockShortUrlRepository, RetryingShortUrlRepository,
};
