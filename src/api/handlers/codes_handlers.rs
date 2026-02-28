use axum::{Json, extract::State, response::IntoResponse};
use hyper::StatusCode;

use crate::{
    api::error::ApiError,
    application::{repository::short_url_repository, state::SharedState},
    domain::models::short_url::{ShortUrl},
};

pub async fn list_codes_handler(
    State(state): State<SharedState>,
) -> Result<Json<Vec<ShortUrl>>, ApiError> {
    let codes = short_url_repository::list(state).await.unwrap(); //TODO: need ApiError FROM DatabaseError in order to use `?` instead of `unwrap`
    Ok(Json(codes))
}

pub async fn add_code_handler(
    State(state): State<SharedState>,
    Json(input_url): Json<String>,
) -> Result<impl IntoResponse, ApiError> {
    let _code = short_url_repository::add(input_url, state).await.unwrap();
    Ok(StatusCode::CREATED)
}
