use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug)]
pub struct ShortUrlSpec {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub uuid: Uuid,
    pub code: String,
}

impl ShortUrlSpec {
    pub fn long_url(&self) -> &str {
        &self.long_url
    }

    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}
