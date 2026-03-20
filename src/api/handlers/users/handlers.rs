use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};

use crate::{
    api::{
        error::ApiError,
        handlers::users::{create_user_request::CreateUserRequest, user_response::UserResponse},
    },
    application::{service::user::create_user_params::CreateUserParams, state::SharedState},
    domain::errors::user_error::UserError,
};

pub async fn create_user(
    State(state): State<SharedState>,
    req_payload: Result<Json<CreateUserRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let Json(parsed_input) =
        req_payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    let dto: CreateUserParams = parsed_input.into();

    let created = state.users.add_user(dto).await?;
    let res = created.into();

    Ok((StatusCode::CREATED, Json(res)))
}
