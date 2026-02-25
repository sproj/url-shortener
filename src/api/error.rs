use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorKind {
    ResourceNotFound,
    ValidationError,
}

impl Display for ApiErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            serde_json::json!(self).as_str().unwrap_or_default()
        )
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub status: u16,
    pub errors: Vec<ApiErrorEntry>,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let api_error = serde_json::to_string_pretty(&self).unwrap_or_default();
        write!(f, "{}", api_error)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ApiErrorEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl ApiErrorEntry {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
            timestamp: Utc::now(),
            ..Default::default()
        }
    }

    pub fn code<S: ToString>(mut self, code: S) -> Self {
        self.code = Some(code.to_string());
        self
    }

    pub fn kind<S: ToString>(mut self, kind: S) -> Self {
        self.kind = Some(kind.to_string());
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_owned());
        self
    }

    pub fn detail(mut self, detail: serde_json::Value) -> Self {
        self.detail = Some(detail);
        self
    }
}

impl From<StatusCode> for ApiErrorEntry {
    fn from(status_code: StatusCode) -> Self {
        let error_message = status_code.to_string();
        let error_code = error_message.replace(' ', "_").to_lowercase();
        Self::new(&error_message).code(error_code)
    }
}

impl From<(StatusCode, Vec<ApiErrorEntry>)> for ApiError {
    fn from(error_from: (StatusCode, Vec<ApiErrorEntry>)) -> Self {
        let (status_code, errors) = error_from;
        Self {
            status: status_code.as_u16(),
            errors,
        }
    }
}

impl From<(StatusCode, ApiErrorEntry)> for ApiError {
    fn from(error_from: (StatusCode, ApiErrorEntry)) -> Self {
        let (status_code, error_entry) = error_from;
        Self {
            status: status_code.as_u16(),
            errors: vec![error_entry],
        }
    }
}

impl From<StatusCode> for ApiError {
    fn from(status_code: StatusCode) -> Self {
        Self {
            status: status_code.as_u16(),
            errors: vec![status_code.into()],
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::error!("Error response: {:?}", self);
        let status_code =
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status_code, Json(self)).into_response()
    }
}
