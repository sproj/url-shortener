use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use hyper::StatusCode;
use thiserror::Error;

use crate::{
    api::error::{ApiError, ApiErrorKind},
    application::{
        repository::{database_error::DatabaseError, short_url_repository},
        state::SharedState,
    },
    domain::models::short_url::ShortUrl,
};

pub async fn get_all_shorturls_handler(
    State(state): State<SharedState>,
) -> Result<Json<Vec<ShortUrl>>, ApiError> {
    let short_urls = short_url_repository::get_all(state).await?;

    Ok(Json(short_urls))
}

pub async fn get_one_shorturl_by_id_handler(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<ShortUrl>, ApiError> {
    if let Some(short) = short_url_repository::get_by_id(state, id).await? {
        Ok(Json(short))
    } else {
        Err(ApiError::from(ShortUrlError::NotFound(id)))
    }
}

pub async fn add_one_shorturl_handler(
    State(state): State<SharedState>,
    Json(input_url): Json<String>,
) -> Result<impl IntoResponse, ApiError> {
    let created = short_url_repository::add_one(state, input_url).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn delete_one_shorturl_handler(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<bool>, ApiError> {
    if let Some(deleted_count) = short_url_repository::delete_one_by_id(state, id).await? {
        Ok(Json(deleted_count))
    } else {
        Err(ApiError::from(ShortUrlError::NotFound(id)))
    }
}

#[derive(Debug, Error)]
pub enum ShortUrlError {
    #[error("short_url not found: {0}")] // todo: I do not get how this macro works
    NotFound(i64), // todo: short_url should have a uuid so the database id is not exposed
    #[error("invalid input url: {0}")]
    InvalidLongUrl(String),
    #[error("data layer error: {0}")]
    Storage(DatabaseError),
}

impl From<DatabaseError> for ShortUrlError {
    fn from(e: DatabaseError) -> Self {
        Self::Storage(e)
    }
}

impl From<ShortUrlError> for ApiError {
    fn from(short_url_error: ShortUrlError) -> Self {
        ApiError::from(&short_url_error)
    }
}

impl From<&ShortUrlError> for ApiError {
    fn from(short_url_error: &ShortUrlError) -> Self {
        eprintln!("ShortUrlError: {:?}", &short_url_error);
        match short_url_error {
            ShortUrlError::NotFound(id) => ApiError::from(short_url_error)
                .kind(ApiErrorKind::ResourceNotFound)
                .detail(serde_json::json!({ "short_url_id": id })),
            ShortUrlError::InvalidLongUrl(long) => ApiError::from(short_url_error)
                .kind(ApiErrorKind::ValidationError)
                .detail(serde_json::json!({"invalid url": long})),
            ShortUrlError::Storage(_e) => {
                ApiError::new("internal database error").kind(ApiErrorKind::Internal)
            }
        }
    }
}
