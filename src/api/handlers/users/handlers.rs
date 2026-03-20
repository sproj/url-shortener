use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    api::{
        error::ApiError,
        handlers::users::{
            create_user_request::CreateUserRequest, update_password_request::UpdatePasswordRequest,
            user_response::UserResponse,
        },
    },
    application::{service::user::create_user_params::CreateUserParams, state::SharedState},
    domain::errors::user_error::UserError,
};

pub async fn get_all(
    State(state): State<SharedState>,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    let users = state.users.list_all().await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

pub async fn get_one_by_uuid(
    State(state): State<SharedState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<UserResponse>, ApiError> {
    tracing::debug!(%uuid, "get user by uuid");
    match state.users.get_one_by_uuid(uuid).await? {
        Some(user) => Ok(Json(user.into())),
        None => {
            tracing::warn!(%uuid, "user not found");
            Err(ApiError::from(UserError::NotFound(uuid.to_string())))
        }
    }
}

pub async fn delete_one_by_uuid(
    State(state): State<SharedState>,
    Path(uuid): Path<Uuid>,
) -> Result<Json<String>, ApiError> {
    if state.users.delete_one_by_uuid(uuid).await? {
        tracing::debug!(%uuid, "user deleted");
        Ok(Json(uuid.to_string()))
    } else {
        tracing::warn!(%uuid, "user not found for delete");
        Err(ApiError::from(UserError::NotFound(uuid.to_string())))
    }
}

pub async fn update_password(
    State(state): State<SharedState>,
    Path(uuid): Path<Uuid>,
    req_payload: Result<Json<UpdatePasswordRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(parsed_input) =
        req_payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    if state
        .users
        .update_password_by_uuid(parsed_input.password, uuid)
        .await?
    {
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::from(UserError::NotFound(uuid.to_string())))
    }
}

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
