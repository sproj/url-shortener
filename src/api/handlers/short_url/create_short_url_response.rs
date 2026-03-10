use std::fmt::{Display, Formatter, Result};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::short_url::ShortUrl;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateShortUrlResponse {
    pub id: i64,
    pub uuid: Uuid,
    pub code: String,
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<ShortUrl> for CreateShortUrlResponse {
    fn from(value: ShortUrl) -> Self {
        Self {
            id: value.id,
            uuid: value.uuid,
            code: value.code,
            long_url: value.long_url,
            expires_at: value.expires_at,
        }
    }
}

impl Display for CreateShortUrlResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "uuid: {}, code: {}, long_url: {}, expires_at: {:?}",
            self.uuid, self.code, self.long_url, self.expires_at
        )
    }
}
