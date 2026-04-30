use crate::{
    api::{
        error::ApiError,
        handlers::users::{
            create_user_request::CreateUserRequest, update_password_request::UpdatePasswordRequest,
            user_response::UserResponse,
        },
    },
    application::{service::user::create_user_params::CreateUserParams, state::SharedState},
    domain::errors::UserError,
};
use auth::jwt::{AccessClaims, ClaimsMethods};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
};
use tracing::instrument;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/users",
    tag = "users",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "List all users", body = [UserResponse]),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 500, description = "Unexpected data access error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims))]
pub async fn get_all(
    State(state): State<SharedState>,
    access_claims: AccessClaims,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    access_claims.validate_role_admin()?;
    let users = state.user_service.list_all().await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

#[utoipa::path(
    get,
    path = "/users/{uuid}",
    tag = "users",
    security(("bearerAuth" = [])),
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "User found", body = UserResponse),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 403, description = "Resource access is forbidden", body = ApiError),
        (status = 404, description = "User was not found", body = ApiError),
        (status = 500, description = "Unexpected data access error", body = ApiError)
    )
)]
#[instrument(skip(state))]
pub async fn get_one_by_uuid(
    State(state): State<SharedState>,
    Path(subject_uuid): Path<Uuid>,
    access_claims: AccessClaims,
) -> Result<Json<UserResponse>, ApiError> {
    access_claims.assert_is_subject_or_admin(subject_uuid)?;

    tracing::debug!(%subject_uuid, "get user by uuid");
    match state.user_service.get_one_by_uuid(subject_uuid).await? {
        Some(user) => Ok(Json(user.into())),
        None => {
            tracing::warn!(%subject_uuid, "user not found");
            Err(ApiError::from(UserError::NotFound(
                subject_uuid.to_string(),
            )))
        }
    }
}

#[utoipa::path(
    delete,
    path = "/users/{uuid}",
    tag = "users",
    security(("bearerAuth" = [])),
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "User deleted", body = String),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 403, description = "Resource access is forbidden", body = ApiError),
        (status = 500, description = "Unexpected deletion error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims))]
pub async fn delete_one_by_uuid(
    State(state): State<SharedState>,
    Path(subject_uuid): Path<Uuid>,
    access_claims: AccessClaims,
) -> Result<Json<String>, ApiError> {
    access_claims.assert_is_subject_or_admin(subject_uuid)?;

    state.user_service.delete_one_by_uuid(subject_uuid).await?;
    tracing::debug!(%subject_uuid, "user deleted");
    Ok(Json(subject_uuid.to_string()))
}

#[utoipa::path(
    put,
    path = "/users/{uuid}/password",
    tag = "users",
    security(("bearerAuth" = [])),
    params(
        ("uuid" = Uuid, Path, description = "User UUID")
    ),
    request_body = UpdatePasswordRequest,
    responses(
        (status = 200, description = "Password updated"),
        (status = 401, description = "Bearer token is missing or invalid", body = ApiError),
        (status = 403, description = "Resource access is forbidden", body = ApiError),
        (status = 422, description = "Request body could not be parsed", body = ApiError),
        (status = 500, description = "Unexpected update error", body = ApiError)
    )
)]
#[instrument(skip(state, access_claims, req_payload))]
pub async fn update_password(
    State(state): State<SharedState>,
    Path(subject_uuid): Path<Uuid>,
    access_claims: AccessClaims,
    req_payload: Result<Json<UpdatePasswordRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(parsed_input) =
        req_payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    access_claims.assert_is_subject_or_admin(subject_uuid)?;

    state
        .user_service
        .update_password_by_uuid(parsed_input.password, subject_uuid)
        .await?;

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/users",
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 400, description = "Request content failed validation", body = ApiError),
        (status = 409, description = "User already exists", body = ApiError),
        (status = 422, description = "Request body could not be parsed", body = ApiError),
        (status = 500, description = "Unexpected creation error", body = ApiError)
    )
)]
#[instrument(skip(state))]
pub async fn create_user(
    State(state): State<SharedState>,
    req_payload: Result<Json<CreateUserRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let Json(parsed_input) =
        req_payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    let dto: CreateUserParams = parsed_input.into();

    let created = state.user_service.add_user(dto).await?;
    let res = created.into();

    Ok((StatusCode::CREATED, Json(res)))
}
