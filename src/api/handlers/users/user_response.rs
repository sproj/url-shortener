use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::user::User;

#[derive(Serialize, Deserialize)]
pub struct UserResponse {
    pub uuid: Uuid,
    pub username: String,
    pub email: String,
    pub active: bool,
    pub roles: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl From<User> for UserResponse {
    fn from(value: User) -> Self {
        Self {
            uuid: value.uuid,
            username: value.username,
            email: value.email,
            active: value.active,
            roles: value.roles,
            created_at: value.created_at,
            updated_at: value.updated_at,
            deleted_at: value.deleted_at,
        }
    }
}
