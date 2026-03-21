use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};

use crate::{
    api::{error::ApiError, handlers::auth::login_request::LoginRequest},
    application::state::SharedState,
    domain::errors::user_error::UserError,
};

pub async fn login(
    State(state): State<SharedState>,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(parsed_login_request) =
        payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    match state.users.verify_login(parsed_login_request.into()).await {
        Ok(true) => Ok(StatusCode::OK),
        Ok(false) => Ok(StatusCode::UNAUTHORIZED),
        Err(e) => Err(ApiError::from(e)),
    }
}
