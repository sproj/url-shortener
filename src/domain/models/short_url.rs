use chrono::{DateTime, Utc};
use tokio_postgres::Row;

#[derive(Debug, Clone)]
pub struct ShortUrl {
    pub id: u32,
    pub code: String,
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ShortUrl {
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|ts| ts <= Utc::now())
    }
}

impl From<Row> for ShortUrl {
    fn from(row: Row) -> Self {
        Self {
            id: row.get("id"),
            code: row.get("code"),
            long_url: row.get("long_url"),
            expires_at: row.get("expires_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            deleted_at: row.get("deleted_at"),
        }
    }
}

pub struct ShortUrlDto {
    pub code: String,
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
}
