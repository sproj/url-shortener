use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    response::IntoResponse,
};

use crate::{
    api::{error::ApiError, handlers::auth::login_request::LoginRequest},
    application::{
        security::jwt::{AccessClaims, JwtTokens, RefreshClaims, tokens_to_response},
        service::user::login_params::LoginParams,
        state::SharedState,
    },
    domain::errors::UserError,
};

pub async fn login(
    State(state): State<SharedState>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(parsed_login_request) =
        payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    let login_params = LoginParams::from(&parsed_login_request);

    let tokens = state.auth_service.verify_login(login_params).await?;

    let res = tokens_to_response(tokens);

    tracing::debug!(%parsed_login_request, "login successful, tokens issued");
    Ok(res)
}

pub async fn logout(
    State(state): State<SharedState>,
    access_claims: AccessClaims,
) -> Result<(), ApiError> {
    state
        .auth_service
        .revoke_refresh(&access_claims.jti)
        .await
        .map_err(ApiError::from)
}

pub async fn refresh(
    State(state): State<SharedState>,
    refresh_claims: RefreshClaims,
) -> Result<Json<JwtTokens>, ApiError> {
    let tokens = state.auth_service.refresh(refresh_claims).await?;

    Ok(Json(tokens))
}
