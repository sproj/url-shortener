use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    api::{
        error::ApiError,
        handlers::short_url::{
            ValidatedCreateShortUrlRequest, create_short_url_request::CreateShortUrlRequest,
            create_short_url_response::CreateShortUrlResponse,
        },
    },
    application::service::short_url::short_url_service,
    application::state::SharedState,
    domain::errors::ShortUrlError,
    domain::models::short_url::ShortUrl,
};

pub async fn get_all(State(state): State<SharedState>) -> Result<Json<Vec<ShortUrl>>, ApiError> {
    let short_urls = short_url_service::get_all(&state.db_pool).await?;
    tracing::debug!(?short_urls, "get all ok");
    Ok(Json(short_urls))
}

pub async fn get_one_by_uuid(
    State(state): State<SharedState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<ShortUrl>, ApiError> {
    tracing::debug!(%uuid, "get one by uuid");
    if let Some(short) = short_url_service::get_by_uuid(&state.db_pool, uuid).await? {
        tracing::debug!(?short, "ok");
        Ok(Json(short))
    } else {
        tracing::warn!(%uuid, "not found");
        Err(ApiError::from(ShortUrlError::NotFound(uuid.to_string())))
    }
}

pub async fn get_one_by_code(
    State(state): State<SharedState>,
    Path(code): Path<String>,
) -> Result<Json<ShortUrl>, ApiError> {
    tracing::debug!(%code, "get one by code");
    if let Some(short) = short_url_service::get_by_code(&state.db_pool, &code).await? {
        tracing::debug!(%short, "ok");
        Ok(Json(short))
    } else {
        tracing::warn!(%code, "not found");
        Err(ApiError::from(ShortUrlError::NotFound(code)))
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

    let dto: ValidatedCreateShortUrlRequest = parsed_input.try_into().map_err(ApiError::from)?;

    let created = short_url_service::add_one(
        &state.db_pool,
        state.code_generator.clone(),
        state.max_retries,
        dto,
    )
    .await?;

    let payload: CreateShortUrlResponse = CreateShortUrlResponse::from(created);
    tracing::debug!(%payload, "ok");

    Ok((StatusCode::CREATED, Json(payload)))
}

pub async fn delete_one_by_uuid(
    State(state): State<SharedState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<String>, ApiError> {
    if short_url_service::delete_one_by_uuid(&state.db_pool, state.redirect_cache.clone(), uuid)
        .await?
    {
        tracing::debug!(%uuid, "ok");
        Ok(Json(uuid.to_string()))
    } else {
        tracing::warn!(%uuid, "not found");
        Err(ApiError::from(ShortUrlError::NotFound(uuid.to_string())))
    }
}
