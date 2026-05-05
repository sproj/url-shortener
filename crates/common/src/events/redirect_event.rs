use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedirectType {
    Permanent,
    Temporary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectEvent {
    pub code: String,
    pub timestamp: DateTime<Utc>,
    pub redirect_type: RedirectType,
}
