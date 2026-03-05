use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct ValidatedCreateShortUrlRequest {
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}
