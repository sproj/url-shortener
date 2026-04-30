use std::fmt::Display;

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug)]
pub struct ShortUrlSpec {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub uuid: Uuid,
    pub code: String,
    pub user_id: Option<i64>,
}

impl ShortUrlSpec {
    pub fn long_url(&self) -> &str {
        &self.long_url
    }

    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}

impl Display for ShortUrlSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "uuid: {}, code: {}, long_url: {}, expires_at: {:?}",
            self.uuid, self.code, self.long_url, self.expires_at
        )
    }
}
