use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

use crate::application::repository::database_error::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortUrl {
    pub id: i64,
    pub uuid: uuid::Uuid,
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

impl TryFrom<Row> for ShortUrl {
    type Error = DatabaseError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Self::try_from(&row)
    }
}

impl TryFrom<&Row> for ShortUrl {
    type Error = DatabaseError;
    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row
                .try_get::<_, i64>("id")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            uuid: row
                .try_get::<_, uuid::Uuid>("uuid")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            code: row
                .try_get("code")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            long_url: row
                .try_get("long_url")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            expires_at: row
                .try_get("expires_at")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            updated_at: row
                .try_get("updated_at")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            deleted_at: row
                .try_get("deleted_at")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
        })
    }
}
