use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};

use crate::infrastructure::database::database_error::DatabaseError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ApiErrorKind {
    ResourceNotFound,
    UnprocessableInput,
    ValidationError,
    #[default]
    Internal,
}

impl Display for ApiErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                ApiErrorKind::ResourceNotFound => "resource not found",
                ApiErrorKind::UnprocessableInput => "unprocessable input",
                ApiErrorKind::ValidationError => "invalid request or parameters",
                ApiErrorKind::Internal => "unexpected internal error",
            }
        )
    }
}


impl ApiErrorKind {
    pub fn status_code(self) -> StatusCode {
        match self {
            ApiErrorKind::ResourceNotFound => StatusCode::NOT_FOUND,
            ApiErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorKind::ValidationError => StatusCode::BAD_REQUEST,
            ApiErrorKind::UnprocessableInput => StatusCode::UNPROCESSABLE_ENTITY,
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
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Default::default()
        }
    }

    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_owned();
        self
    }
    pub fn kind(mut self, kind: ApiErrorKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn detail(mut self, detail: serde_json::Value) -> Self {
        self.detail = Some(detail);
        self
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let api_error = serde_json::to_string_pretty(&self).unwrap_or_default();
        write!(f, "{}", api_error)
    }
}

impl std::error::Error for ApiError {}

impl From<DatabaseError> for ApiError {
    fn from(error: DatabaseError) -> Self {
        tracing::error!(%error, "database error",);
        Self {
            message: "database operation failed".to_string(),
            kind: ApiErrorKind::Internal,
            detail: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.kind.status_code();
        (status_code, Json(self)).into_response()
    }
}
