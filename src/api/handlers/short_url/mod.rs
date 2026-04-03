mod handlers;

mod create_short_url_request;
mod create_short_url_response;
mod create_vanity_url_request;
mod update_short_url_request;
mod validated_create_short_url_request;

mod input_validation_rules;

pub use create_short_url_request::CreateShortUrlRequest;
pub use create_short_url_response::CreateShortUrlResponse;
pub use create_vanity_url_request::CreateVanityUrlRequest;
pub use update_short_url_request::{UpdateShortUrlRequest, ValidatedUpdateShortUrlRequest};
pub use validated_create_short_url_request::ValidatedCreateShortUrlRequest;

pub use handlers::{
    create_short_url, create_vanity_url, delete_one_by_uuid, get_all, get_one_by_code,
    get_one_by_uuid, update_one_by_uuid,
};
