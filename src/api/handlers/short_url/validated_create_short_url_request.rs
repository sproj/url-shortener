use std::fmt::Display;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ValidatedCreateShortUrlRequest {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Display for ValidatedCreateShortUrlRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "long_url: {}, expires_at: {:?}",
            self.long_url, self.expires_at
        )
    }
}
