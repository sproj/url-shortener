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

pub async fn get_all(State(state): State<SharedState>) -> Result<Json<Vec<ShortUrl>>, ApiError> {
    println!("shorturl_handler::get_all called");
    let short_urls = short_url_repository::get_all(state).await?;
    println!("shorturl_handler::get_all returning");
    Ok(Json(short_urls))
}

pub async fn get_one_by_id(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<ShortUrl>, ApiError> {
    println!("shorturl_handler::get_one_by_id called with {}", id);
    if let Some(short) = short_url_repository::get_by_id(state, id).await? {
        println!("shorturl_handler::get_one_by_id returning Ok");
        Ok(Json(short))
    } else {
        eprintln!("shorturl_handler::get_one_by_id returning ShortUrlError");
        Err(ApiError::from(ShortUrlError::NotFound(id)))
    }
}

pub async fn add_one(
    State(state): State<SharedState>,
    Json(input_url): Json<String>,
) -> Result<impl IntoResponse, ApiError> {
    println!("shorturl_handler::add_one called with {}", input_url);
    let created = short_url_repository::add_one(state, input_url).await?;
    println!("shorturl_handler::add_one returning Ok");
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn delete_one_by_id(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<bool>, ApiError> {
    println!("shorturl_handler::delete_one called with {}", id);
    if let Some(deleted_count) = short_url_repository::delete_one_by_id(state, id).await? {
        println!("shorturl_handler::delete_one returning Ok");
        Ok(Json(deleted_count))
    } else {
        eprintln!("shorturl_handler::delete_one returning ShortUrlError");
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
        let short_url_error_message = &short_url_error.to_string();
        eprintln!("ShortUrlError: {:?}", &short_url_error);
        match short_url_error {
            ShortUrlError::NotFound(id) => ApiError::new(short_url_error_message)
                .kind(ApiErrorKind::ResourceNotFound)
                .detail(serde_json::json!({ "short_url_id": id })),
            ShortUrlError::InvalidLongUrl(long) => ApiError::new(short_url_error_message)
                .kind(ApiErrorKind::ValidationError)
                .detail(serde_json::json!({"invalid url": long})),
            ShortUrlError::Storage(_e) => {
                ApiError::new("internal database error").kind(ApiErrorKind::Internal)
            }
        }
    }
}
