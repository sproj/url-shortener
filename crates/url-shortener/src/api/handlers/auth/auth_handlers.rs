use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    response::IntoResponse,
};
use serde_json::json;
use tracing::instrument;
use uuid::Uuid;

use crate::application::security::claims::{AccessClaims, RefreshClaims};
use crate::{
    api::{
        error::ApiError,
        handlers::{auth::login_request::LoginRequest, users::user_response::UserResponse},
        swagger::LoginResponse,
    },
    application::{service::user::login_params::LoginParams, state::SharedState},
    domain::errors::UserError,
};
use auth::{auth_error::AuthError, jwt::JwtTokens};

#[utoipa::path(
    post,
    path = "/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated successfully", body = LoginResponse),
        (status = 401, description = "Credentials are invalid", body = ApiError),
        (status = 422, description = "Request body could not be parsed", body = ApiError),
        (status = 500, description = "Unexpected authentication error", body = ApiError)
    )
)]
#[instrument(skip(state))]
pub async fn login(
    State(state): State<SharedState>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(parsed_login_request) =
        payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    let login_params = LoginParams::from(&parsed_login_request);

    let tokens = state.auth_service.verify_login(login_params).await?;

    let res = Json(json!({
        "access_token": tokens.access_token,
        "refresh_token": tokens.refresh_token,
        "token_type": "Bearer"
    }));

    tracing::debug!(%parsed_login_request, "login successful, tokens issued");
    Ok(res)
}

#[utoipa::path(
    post,
    path = "/logout",
    tag = "auth",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Refresh token revoked"),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 500, description = "Unexpected token revocation error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims))]
pub async fn logout(
    State(state): State<SharedState>,
    access_claims: AccessClaims,
) -> Result<(), ApiError> {
    tracing::debug!(sub=&access_claims.sub, jti=%&access_claims.jti, "logging out");
    state
        .auth_service
        .revoke_refresh(&access_claims.jti)
        .await
        .map_err(ApiError::from)
}

#[utoipa::path(
    post,
    path = "/refresh",
    tag = "auth",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Issued a fresh access and refresh token pair", body = JwtTokens),
        (status = 401, description = "Refresh token is missing, expired, or invalid", body = ApiError),
        (status = 500, description = "Unexpected token refresh error", body = ApiError)
    )
)]
#[instrument(skip(state, refresh_claims))]
pub async fn refresh(
    State(state): State<SharedState>,
    refresh_claims: RefreshClaims,
) -> Result<Json<JwtTokens>, ApiError> {
    tracing::debug!(sub=&refresh_claims.sub, jti=%&refresh_claims.jti, prf=%&refresh_claims.prf, "refreshing claims");
    let tokens = state.auth_service.refresh(refresh_claims).await?;

    Ok(Json(tokens))
}

#[utoipa::path(
    post,
    path = "/me",
    tag = "auth",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Your user response", body = UserResponse),
        (status = 401, description = "Refresh token is missing, expired, or invalid", body = ApiError),
        (status = 500, description = "Unexpected error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims))]
pub async fn user_info(
    State(state): State<SharedState>,
    access_claims: AccessClaims,
) -> Result<Json<UserResponse>, ApiError> {
    tracing::debug!(
        sub = &access_claims.sub,
        jti = &access_claims.jti,
        "reading user info"
    );
    let user_uuid = Uuid::parse_str(&access_claims.sub).map_err(|e| {
        tracing::warn!(%access_claims.sub, %e, "failed to parse uuid from access token sub");
        AuthError::InvalidToken
    })?;
    if let Some(user) = state.user_service.get_one_by_uuid(user_uuid).await? {
        Ok(Json(user.into()))
    } else {
        Err(ApiError::from(UserError::NotFound(user_uuid.to_string())))
    }
}
