use std::sync::Arc;

use crate::{
    api::handlers::short_url::{
        CreateShortUrlRequest, ShortUrlError, ValidatedCreateShortUrlRequest,
    },
    application::{
        repository::{database_error::DatabaseError, short_url_repository::ShortUrlRepository},
        service::short_url::{
            ShortUrlSpec,
            code_generator::{CodeGenerator, RandomCodeGenerator},
        },
    },
    domain::models::short_url::ShortUrl,
};

const SHORT_URL_CODE_KEY_CONSTRAINT_NAME: &str = "short_url_code_key";

pub struct ShortUrlService {
    code_generator: Arc<dyn CodeGenerator>,
    max_retries: u8,
    repository: Arc<ShortUrlRepository>,
}
impl ShortUrlService {
    pub fn new(repository: ShortUrlRepository) -> Self {
        Self {
            repository: Arc::new(repository),
            code_generator: Arc::new(RandomCodeGenerator),
            max_retries: 5,
        }
    }

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

    pub async fn delete_one_by_id(&self, id: i64) -> Result<Option<bool>, DatabaseError> {
        self.repository.delete_one_by_id(id).await
    }

    pub async fn add_one(&self, input: CreateShortUrlRequest) -> Result<ShortUrl, ShortUrlError> {
        println!("shorturl_service::add_one called with {:?}", input);

        let dto: ValidatedCreateShortUrlRequest = input.try_into()?;

        println!("shorturl_service::add_one created dto {:?}", dto);

        // uuid is stable across insert attempts. `code` is re-generated on conflict (should be very rare but is possible).
        let uuid = uuid::Uuid::now_v7();
        for attempt in 1..=self.max_retries {
            let mut spec: ShortUrlSpec = ShortUrlSpec::from(dto.clone());
            println!("shorturl_service::add_one created spec {:?}", spec);

            let code = self.code_generator.next_code();

            spec.code = Some(code);
            spec.uuid = Some(uuid);

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
}
