use base64_url::base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use rand::Rng;

use crate::{
    api::handlers::short_url::{
        CreateShortUrlRequest, ShortUrlError, ValidatedCreateShortUrlRequest,
    },
    application::{
        repository::{database_error::DatabaseError, short_url_repository},
        service::short_url::ShortUrlSpec,
        state::SharedState,
    },
    domain::models::short_url::ShortUrl,
};

pub async fn add_one(
    state: SharedState,
    input: CreateShortUrlRequest,
) -> Result<ShortUrl, ShortUrlError> {
    println!("shorturl_service::add_one called with {:?}", input);

    let dto: ValidatedCreateShortUrlRequest = input.try_into()?;

    println!("shorturl_service::add_one created dto {:?}", dto);

    // let mut created: ShortUrl;
    for attempt in 1..=5 {
        let mut spec: ShortUrlSpec = ShortUrlSpec::from(dto.clone());
        println!("shorturl_service::add_one created spec {:?}", spec);

        let uuid = uuid::Uuid::now_v7();
        let code = generate_9_bytes_base64url();

        spec.code = Some(code);
        spec.uuid = Some(uuid);

        println!("Attempt {attempt} insert of new short_url: {:?}", spec);

        match short_url_repository::add_one(&state, spec).await {
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
                    "shorturl_service::add_one conflict on attempt {}: {:?}, {}, {message}",
                    attempt,
                    state,
                    constraint.unwrap_or_default()
                );
                continue;
            }
            Err(e) => {
                eprintln!("database error when inserting short_url: {:?}", e);
                return Err(ShortUrlError::Storage(e));
            }
        }
    }
    Err(ShortUrlError::CodeGenerationExhausted)
}

fn generate_9_bytes_base64url() -> String {
    let mut bytes = vec![0u8; 9];
    rand::rng().fill_bytes(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(&bytes)
}
