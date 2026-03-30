use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    response::IntoResponse,
};

use crate::{
    api::{error::ApiError, handlers::auth::login_request::LoginRequest},
    application::{
        security::{auth::generate_tokens, auth_error::AuthError, jwt::tokens_to_response},
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

    match state.users.verify_login(parsed_login_request.into()).await {
        Ok(user) => {
            let tokens = generate_tokens(
                user,
                &state.jwt_encoding_key,
                state.jwt_access_token_seconds,
                state.jwt_refresh_token_seconds,
            )?;

            Ok(tokens_to_response(tokens))
        }
        Err(UserError::NotFound(e)) => {
            tracing::warn!(%e, "incorrect credentials provided to /login");
            Err(ApiError::from(AuthError::IncorrectCredentials))
        }
        Err(e) => Err(ApiError::from(e)),
    }
}
