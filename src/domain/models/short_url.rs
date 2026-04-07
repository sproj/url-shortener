use std::fmt::{Display, Formatter};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use utoipa::ToSchema;

use crate::{
    domain::errors::RepositoryError, infrastructure::database::database_error::DatabaseError,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShortUrl {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub code: String,
    pub long_url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub user_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ShortUrl {
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|ts| ts <= Utc::now()) && (!self.is_deleted())
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl Display for ShortUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "uuid: {}, code: {}, long_url: {}, expires_at: {:?}, deleted_at: {:?}",
            self.uuid, self.code, self.long_url, self.expires_at, self.deleted_at
        )
    }
}

impl TryFrom<Row> for ShortUrl {
    type Error = RepositoryError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Self::try_from(&row)
    }
}

impl TryFrom<&Row> for ShortUrl {
    type Error = RepositoryError;
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
            user_id: row
                .try_get::<_, Option<i64>>("user_id")
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
