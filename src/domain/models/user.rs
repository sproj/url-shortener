use chrono::{DateTime, Utc};
use std::fmt::{Display, Formatter};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::infrastructure::database::database_error::DatabaseError;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub uuid: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub password_salt: String,
    pub active: bool,
    pub roles: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "uuid: {}, username: {}, email: {}, active: {}, roles: {}",
            self.uuid, self.username, self.email, self.active, self.roles
        )
    }
}

impl TryFrom<Row> for User {
    type Error = DatabaseError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Self::try_from(&row)
    }
}

impl TryFrom<&Row> for User {
    type Error = DatabaseError;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row
                .try_get::<_, i64>("id")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            uuid: row
                .try_get::<_, uuid::Uuid>("uuid")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            username: row
                .try_get("username")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            email: row
                .try_get("email")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            password_hash: row
                .try_get("password_hash")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            password_salt: row
                .try_get("password_salt")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            active: row
                .try_get("active")
                .map_err(|e| DatabaseError::Mapping(e.to_string()))?,
            roles: row
                .try_get("roles")
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
