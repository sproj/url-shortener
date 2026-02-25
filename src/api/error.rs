use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorKind {
    ResourceNotFound,
    ValidationError,
    Internal,
}

impl Display for ApiErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            // serde_json::json!(self).as_str().unwrap_or_default()
            match self {
                ApiErrorKind::ResourceNotFound => "resource not found",
                ApiErrorKind::Internal => "unexpected internal error",
                ApiErrorKind::ValidationError => "invalid request or parameters",
            }
        )
    }
}

impl Default for ApiErrorKind {
    fn default() -> Self {
        Self::Internal
    }
}

impl ApiErrorKind {
    pub fn status_code(self) -> StatusCode {
        match self {
            ApiErrorKind::ResourceNotFound => StatusCode::NOT_FOUND,
            ApiErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorKind::ValidationError => StatusCode::BAD_REQUEST,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub kind: ApiErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
            ..Default::default()
        }
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let api_error = serde_json::to_string_pretty(&self).unwrap_or_default();
        write!(f, "{}", api_error)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        dbg!("Error response: {:?}", &self);
        let status_code = self.kind.status_code();
        (status_code, Json(self)).into_response()
    }
}
