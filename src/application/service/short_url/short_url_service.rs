use base64_url::base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use rand::Rng;

use crate::{
    api::handlers::short_url::{
        CreateShortUrlRequest, ShortUrlError, ValidatedCreateShortUrlRequest,
    },
    application::{
        repository::short_url_repository, service::short_url::ShortUrlSpec, state::SharedState,
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

    let mut spec: ShortUrlSpec = ShortUrlSpec::from(dto);
    println!("shorturl_service::add_one created spec {:?}", spec);

    spec.code = Some(generate_9_bytes_base64url());
    spec.uuid = Some(uuid::Uuid::now_v7());

    // let mut created: ShortUrl;
    // for attempt in 1..5 {
    //     println!("Attempt {attempt} insert of new short_url: {:?}", spec);
    let created = short_url_repository::add_one(&state, spec).await?;
    println!("shorturl_service::add_one returning: {:?}", created);
    // }
    Ok(created)
}

fn generate_9_bytes_base64url() -> String {
    let mut bytes = vec![0u8; 9];
    rand::rng().fill_bytes(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(&bytes)
}
