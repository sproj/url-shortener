use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedirectType {
    Permanent,
    Temporary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectEvent {
    pub event_id: Uuid,
    pub code: String,
    pub long_url: String,
    pub timestamp: DateTime<Utc>,
    pub redirect_type: RedirectType,
}
