use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::api::handlers::short_url::ValidatedCreateShortUrlRequest;

#[derive(Debug)]
pub struct ShortUrlSpec {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub uuid: Option<Uuid>,
    pub code: Option<String>,
}

impl From<ValidatedCreateShortUrlRequest> for ShortUrlSpec {
    fn from(value: ValidatedCreateShortUrlRequest) -> Self {
        Self {
            long_url: value.long_url,
            expires_at: value.expires_at,
            uuid: None,
            code: None,
        }
    }
}

impl ShortUrlSpec {
    // pub fn new(dto: ShortUrlSpec, code: String, uuid: Uuid) -> Self {
    //     Self { uuid, code, ..spec }
    // }

    pub fn long_url(&self) -> &str {
        &self.long_url
    }

    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}
