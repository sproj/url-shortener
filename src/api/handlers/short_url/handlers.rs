use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
};
use hyper::StatusCode;

use crate::{
    api::{
        error::ApiError,
        handlers::short_url::{
            ShortUrlError, create_short_url_request::CreateShortUrlRequest,
            create_short_url_response::CreateShortUrlResponse,
        },
    },
    application::state::SharedState,
    domain::models::short_url::ShortUrl,
};

pub async fn get_all(State(state): State<SharedState>) -> Result<Json<Vec<ShortUrl>>, ApiError> {
    println!("shorturl_handler::get_all called");
    let short_urls = state.short_url.get_all().await?;
    println!("shorturl_handler::get_all returning");
    Ok(Json(short_urls))
}

pub async fn get_one_by_id(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<ShortUrl>, ApiError> {
    println!("shorturl_handler::get_one_by_id called with {}", id);
    if let Some(short) = state.short_url.get_by_id(id).await? {
        println!("shorturl_handler::get_one_by_id returning Ok");
        Ok(Json(short))
    } else {
        eprintln!("shorturl_handler::get_one_by_id returning ShortUrlError");
        Err(ApiError::from(ShortUrlError::NotFound(id)))
    }
}

pub async fn add_one(
    State(state): State<SharedState>,
    req_payload: Result<Json<CreateShortUrlRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreateShortUrlResponse>), ApiError> {
    // if payload argument is `Json(payload): Json<CreateShortUrl>`
    // then if the payload is mal-formed or cannot map to target axum replies with a 400 instead of a 422.
    // Also the returned error is not an ApiError, so no details or error code as the user can expect of other error paths.
    // So do the parsing step manually and map the parsing error to the same error structure as the rest of the api.
    let Json(parsed_input) =
        req_payload.map_err(|e| ShortUrlError::UnprocessableInput(e.to_string()))?;

    let created = state.short_url.add_one(parsed_input).await?;
    println!("shorturl_handler::add_one created: {:?}", created);

    let payload: CreateShortUrlResponse = CreateShortUrlResponse::from(created);
    println!("shorturl_handler::add_one returning Ok: {:?}", payload);

    Ok((StatusCode::CREATED, Json(payload)))
}

pub async fn delete_one_by_id(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<bool>, ApiError> {
    println!("shorturl_handler::delete_one called with {}", id);
    if let Some(deleted_count) = state.short_url.delete_one_by_id(id).await? {
        println!("shorturl_handler::delete_one returning Ok");
        Ok(Json(deleted_count))
    } else {
        eprintln!("shorturl_handler::delete_one returning ShortUrlError");
        Err(ApiError::from(ShortUrlError::NotFound(id)))
    }
}
