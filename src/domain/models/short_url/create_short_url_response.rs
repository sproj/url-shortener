use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::models::short_url::ShortUrl;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateShortUrlResponse {
    pub id: i64,
    pub code: String,
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<ShortUrl> for CreateShortUrlResponse {
    fn from(value: ShortUrl) -> Self {
        Self {
            id: value.id,
            code: value.code,
            long_url: value.long_url,
            expires_at: value.expires_at
        }
    }
}