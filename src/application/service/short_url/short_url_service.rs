use std::sync::Arc;

use crate::{
    api::handlers::short_url::{
        CreateShortUrlRequest, ShortUrlError, ValidatedCreateShortUrlRequest,
    },
    application::{
        repository::{database_error::DatabaseError, short_url_repository::ShortUrlRepository},
        service::short_url::{ShortUrlSpec, code_generator::CodeGenerator},
    },
    domain::models::short_url::ShortUrl,
};

const SHORT_URL_CODE_KEY_CONSTRAINT_NAME: &str = "short_url_code_key";

pub struct ShortUrlService {
    code_generator: Arc<dyn CodeGenerator>,
    max_retries: u8,
    repository: Arc<ShortUrlRepository>,
}

pub enum RedirectDecision {
    Permanent { long_url: String },
    Temporary { long_url: String },
    Gone,
    NotFound,
}

impl ShortUrlService {
    pub fn new_with_generator(
        repository: ShortUrlRepository,
        code_generator: Arc<dyn CodeGenerator>,
        max_retries: u8,
    ) -> Self {
        ShortUrlService {
            code_generator,
            max_retries,
            repository: Arc::new(repository),
        }
    }

    pub async fn get_all(&self) -> Result<Vec<ShortUrl>, DatabaseError> {
        self.repository.get_all().await
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<ShortUrl>, DatabaseError> {
        self.repository.get_by_id(id).await
    }

    pub async fn get_by_code(&self, code: &str) -> Result<Option<ShortUrl>, DatabaseError> {
        self.repository.get_by_code(code).await
    }

    pub async fn delete_one_by_id(&self, id: i64) -> Result<bool, DatabaseError> {
        self.repository.delete_one_by_id(id).await
    }

    pub async fn add_one(&self, input: CreateShortUrlRequest) -> Result<ShortUrl, ShortUrlError> {
        println!("shorturl_service::add_one called with {:?}", input);

        let dto: ValidatedCreateShortUrlRequest = input.try_into()?;

        println!("shorturl_service::add_one created dto {:?}", dto);

        // uuid is stable across insert attempts. `code` is re-generated on conflict (should be very rare but is possible).
        let uuid = uuid::Uuid::now_v7();

        for attempt in 1..=self.max_retries {
            let spec = ShortUrlSpec {
                long_url: dto.long_url.clone(),
                expires_at: dto.expires_at,
                uuid,
                code: self.code_generator.next_code(),
            };
            println!("shorturl_service::add_one created spec {:?}", spec);

            println!("Attempt {attempt} insert of new short_url: {:?}", spec);

            match self.repository.add_one(spec).await {
                Ok(created) => {
                    println!("shorturl_service::add_one returning: {:?}", created);
                    return Ok(created);
                }
                Err(DatabaseError::Conflict {
                    state,
                    constraint,
                    message,
                }) => {
                    println!(
                        "shorturl_service::add_one conflict on attempt {}: {:?}, {}",
                        attempt, state, &message
                    );
                    let is_code_conflict = matches!(
                        constraint.as_deref(),
                        Some(SHORT_URL_CODE_KEY_CONSTRAINT_NAME)
                    );
                    if is_code_conflict {
                        continue;
                    } else {
                        return Err(ShortUrlError::Storage(DatabaseError::Conflict {
                            state,
                            constraint,
                            message,
                        }));
                    }
                }
                Err(e) => {
                    eprintln!("database error when inserting short_url: {:?}", e);
                    return Err(ShortUrlError::Storage(e));
                }
            }
        }
        Err(ShortUrlError::CodeGenerationExhausted)
    }

    pub async fn resolve_redirect_decision(
        &self,
        code: &str,
    ) -> Result<RedirectDecision, DatabaseError> {
        let record = self.get_by_code(code).await?;
        match record {
            None => Ok(RedirectDecision::NotFound),
            Some(short) if short.is_deleted() => Ok(RedirectDecision::Gone),
            Some(short) if short.is_expired() => Ok(RedirectDecision::Gone),
            Some(short) if short.expires_at.is_none() => Ok(RedirectDecision::Permanent {
                long_url: short.long_url,
            }),
            Some(short) => Ok(RedirectDecision::Temporary {
                long_url: short.long_url,
            }),
        }
    }
}
