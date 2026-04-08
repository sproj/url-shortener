use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    api::{
        error::ApiError,
        extractors::OptionalAccessClaims,
        handlers::short_url::{
            CreateShortUrlRequest, CreateShortUrlResponse, CreateVanityUrlRequest,
            UpdateShortUrlRequest,
        },
    },
    application::{
        security::{
            auth_error::AuthError,
            jwt::{AccessClaims, ClaimsMethods},
        },
        service::short_url::{ValidatedCreateShortUrlRequest, ValidatedUpdateShortUrlRequest},
        state::SharedState,
    },
    domain::{errors::ShortUrlError, models::short_url::ShortUrl},
};

#[utoipa::path(
    get,
    path = "/shorten",
    tag = "short-url",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "List all short URLs", body = [ShortUrl]),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 500, description = "Unexpected data access error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims))]
pub async fn get_all(
    access_claims: AccessClaims,
    State(state): State<SharedState>,
) -> Result<Json<Vec<ShortUrl>>, ApiError> {
    access_claims.validate_role_admin()?;
    let short_urls = state.short_url_service.get_all().await?;
    tracing::debug!(?short_urls, "get all ok");
    Ok(Json(short_urls))
}

#[utoipa::path(
    post,
    path = "/shorten",
    tag = "short-url",
    request_body = CreateShortUrlRequest,
    responses(
        (status = 201, description = "Short URL created", body = CreateShortUrlResponse),
        (status = 400, description = "Request content failed validation", body = ApiError),
        (status = 422, description = "Request body could not be parsed", body = ApiError),
        (status = 500, description = "Unexpected creation error", body = ApiError)
    )
)]
#[instrument(skip(state))]
pub async fn create_short_url(
    State(state): State<SharedState>,
    optional_acces_claims: OptionalAccessClaims,
    req_payload: Result<Json<CreateShortUrlRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreateShortUrlResponse>), ApiError> {
    // if payload argument is `Json(payload): Json<CreateShortUrl>`
    // then if the payload is mal-formed or cannot map to target axum replies with a 400 instead of a 422.
    // Also the returned error is not an ApiError, so no details or error code as the user can expect of other error paths.
    // So do the parsing step manually and map the parsing error to the same error structure as the rest of the api.
    let Json(parsed_input) =
        req_payload.map_err(|e| ShortUrlError::UnprocessableInput(e.to_string()))?;

    let mut dto: ValidatedCreateShortUrlRequest =
        parsed_input.try_into().map_err(ApiError::from)?;

    if let Some(claims) = optional_acces_claims.0 {
        let user_uuid = Uuid::parse_str(&claims.sub)
            .map_err(|_| ApiError::from(AuthError::IncorrectCredentials))?;
        dto.user_uuid = Some(user_uuid);
    }

    let created = state.short_url_service.add_generated_code(dto).await?;

    let payload: CreateShortUrlResponse = CreateShortUrlResponse::from(created);
    tracing::debug!(%payload, "ok");

    Ok((StatusCode::CREATED, Json(payload)))
}

#[utoipa::path(
    post,
    path = "/shorten/vanity",
    tag = "short-url",
    security(("bearerAuth" = [])),
    request_body = CreateVanityUrlRequest,
    responses(
        (status = 201, description = "Vanity short URL created", body = CreateShortUrlResponse),
        (status = 400, description = "Request content failed validation", body = ApiError),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 409, description = "Requested vanity code already exists", body = ApiError),
        (status = 422, description = "Request body could not be parsed", body = ApiError),
        (status = 500, description = "Unexpected creation error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims))]
pub async fn create_vanity_url(
    access_claims: AccessClaims,
    State(state): State<SharedState>,
    req_payload: Result<Json<CreateVanityUrlRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreateShortUrlResponse>), ApiError> {
    // if payload argument is `Json(payload): Json<CreateShortUrl>`
    // then if the payload is mal-formed or cannot map to target axum replies with a 400 instead of a 422.
    // Also the returned error is not an ApiError, so no details or error code as the user can expect of other error paths.
    // So do the parsing step manually and map the parsing error to the same error structure as the rest of the api.
    let Json(parsed_input) =
        req_payload.map_err(|e| ShortUrlError::UnprocessableInput(e.to_string()))?;

    let user_uuid =
        Uuid::parse_str(&access_claims.sub).map_err(|_| ApiError::from(AuthError::InvalidToken))?;

    let dto: ValidatedCreateShortUrlRequest = (parsed_input, user_uuid)
        .try_into()
        .map_err(ApiError::from)?;

    let created = state.short_url_service.add_vanity_url(dto).await?;

    let payload: CreateShortUrlResponse = CreateShortUrlResponse::from(created);
    tracing::debug!(%payload, "ok");

    Ok((StatusCode::CREATED, Json(payload)))
}

#[utoipa::path(
    get,
    path = "/shorten/uuid/{uuid}",
    tag = "short-url",
    params(
        ("uuid" = Uuid, Path, description = "Short URL UUID")
    ),
    responses(
        (status = 200, description = "Short URL found", body = ShortUrl),
        (status = 404, description = "Short URL was not found", body = ApiError),
        (status = 500, description = "Unexpected data access error", body = ApiError)
    )
)]
#[instrument(skip(state))]
pub async fn get_one_by_uuid(
    State(state): State<SharedState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<ShortUrl>, ApiError> {
    tracing::debug!(%uuid, "get one by uuid");
    if let Some(short) = state.short_url_service.get_by_uuid(uuid).await? {
        tracing::debug!(?short, "ok");
        Ok(Json(short))
    } else {
        tracing::warn!(%uuid, "not found");
        Err(ApiError::from(ShortUrlError::NotFound(uuid.to_string())))
    }
}

#[utoipa::path(
    patch,
    path = "/shorten/uuid/{uuid}",
    tag = "short-url",
    security(("bearerAuth" = [])),
    params(
        ("uuid" = Uuid, Path, description = "Short URL UUID")
    ),
    request_body = UpdateShortUrlRequest,
    responses(
        (status = 200, description = "Short URL updated", body = CreateShortUrlResponse),
        (status = 400, description = "Request content failed validation", body = ApiError),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 403, description = "Resource access is forbidden", body = ApiError),
        (status = 404, description = "Short URL was not found", body = ApiError),
        (status = 422, description = "Request body could not be parsed", body = ApiError),
        (status = 500, description = "Unexpected update error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims))]
pub async fn update_one_by_uuid(
    Path(uuid): Path<Uuid>,
    access_claims: AccessClaims,
    State(state): State<SharedState>,
    req_payload: Result<Json<UpdateShortUrlRequest>, JsonRejection>,
) -> Result<Json<CreateShortUrlResponse>, ApiError> {
    let Json(parsed_input) =
        req_payload.map_err(|e| ShortUrlError::UnprocessableInput(e.to_string()))?;

    let dto: ValidatedUpdateShortUrlRequest = parsed_input.try_into()?;

    let user_uuid = Uuid::parse_str(&access_claims.sub).map_err(|e| {
        tracing::warn!(%e, "failed to parse a sub to a uuid from a parsed access token");
        AuthError::InvalidToken
    })?;

    let is_admin = access_claims.validate_role_admin().is_ok();
    let updated = state
        .short_url_service
        .update_one_by_uuid(uuid, user_uuid, is_admin, dto)
        .await?;

    let payload: CreateShortUrlResponse = CreateShortUrlResponse::from(updated);
    Ok(Json(payload))
}

#[utoipa::path(
    delete,
    path = "/shorten/uuid/{uuid}",
    tag = "short-url",
    security(("bearerAuth" = [])),
    params(
        ("uuid" = Uuid, Path, description = "Short URL UUID")
    ),
    responses(
        (status = 200, description = "Short URL deleted", body = String),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 403, description = "Resource access is forbidden", body = ApiError),
        (status = 404, description = "Short URL was not found", body = ApiError),
        (status = 500, description = "Unexpected deletion error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims))]
pub async fn delete_one_by_uuid(
    access_claims: AccessClaims,
    State(state): State<SharedState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<String>, ApiError> {
    let user_uuid = Uuid::parse_str(&access_claims.sub).map_err(|_| AuthError::InvalidToken)?;
    let is_admin = access_claims.validate_role_admin().is_ok();

    if state
        .short_url_service
        .delete_one_by_uuid(uuid, user_uuid, is_admin)
        .await?
    {
        tracing::debug!(%uuid, "ok");
        Ok(Json(uuid.to_string()))
    } else {
        tracing::warn!(%uuid, "not found");
        Err(ApiError::from(ShortUrlError::NotFound(uuid.to_string())))
    }
}

#[utoipa::path(
    get,
    path = "/shorten/code/{code}",
    tag = "short-url",
    params(
        ("code" = String, Path, description = "Short URL code")
    ),
    responses(
        (status = 200, description = "Short URL found", body = ShortUrl),
        (status = 404, description = "Short URL was not found", body = ApiError),
        (status = 500, description = "Unexpected data access error", body = ApiError)
    )
)]
#[instrument(skip(state))]
pub async fn get_one_by_code(
    State(state): State<SharedState>,
    Path(code): Path<String>,
) -> Result<Json<ShortUrl>, ApiError> {
    tracing::debug!(%code, "get one by code");
    match state.short_url_service.get_by_code(&code).await? {
        None => {
            tracing::warn!(%code, "not found");
            Err(ApiError::from(ShortUrlError::NotFound(code)))
        }
        Some(short) => {
            tracing::debug!(%short, "ok");
            Ok(Json(short))
        }
    }
}
