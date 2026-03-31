use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    response::IntoResponse,
};

use crate::{
    api::{error::ApiError, handlers::auth::login_request::LoginRequest},
    application::{
        security::{auth::encode_tokens, jwt::tokens_to_response},
        service::{auth::auth_service, user::login_params::LoginParams},
        state::SharedState,
    },
    domain::errors::user_error::UserError,
};

pub async fn login(
    State(state): State<SharedState>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Json(parsed_login_request) =
        payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    let login_params = LoginParams::from(&parsed_login_request);

    let claims = auth_service::verify_login(
        &state.db_pool,
        state.jwt_access_token_seconds,
        state.jwt_refresh_token_seconds,
        login_params,
    )
    .await?;

    auth_service::cache_refresh_token(state.refresh_token_cache.clone(), &claims.refresh_claims)
        .await?;
    let tokens = encode_tokens(
        &state.jwt_encoding_key,
        claims.access_claims,
        claims.refresh_claims,
    )?;

    let res = tokens_to_response(tokens);

    tracing::debug!(%parsed_login_request, "login successful, tokens issued");
    Ok(res)
}
