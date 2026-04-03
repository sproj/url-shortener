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
    application::{
        security::{
            auth_error::AuthError,
            jwt::{AccessClaims, ClaimsMethods},
        },
        service::user::{create_user_params::CreateUserParams, user_service},
        state::SharedState,
    },
    domain::errors::user_error::UserError,
};

pub async fn get_all(
    State(state): State<SharedState>,
    access_claims: AccessClaims,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    access_claims.validate_role_admin()?;
    let users = user_service::list_all(&state.db_pool).await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

pub async fn get_one_by_uuid(
    State(state): State<SharedState>,
    Path(subject_uuid): Path<Uuid>,
    access_claims: AccessClaims,
) -> Result<Json<UserResponse>, ApiError> {
    user_is_requestor_or_admin(access_claims, subject_uuid)?;

    tracing::debug!(%subject_uuid, "get user by uuid");
    match user_service::get_one_by_uuid(&state.db_pool, subject_uuid).await? {
        Some(user) => Ok(Json(user.into())),
        None => {
            tracing::warn!(%subject_uuid, "user not found");
            Err(ApiError::from(UserError::NotFound(
                subject_uuid.to_string(),
            )))
        }
    }
}

pub async fn delete_one_by_uuid(
    State(state): State<SharedState>,
    Path(subject_uuid): Path<Uuid>,
    access_claims: AccessClaims,
) -> Result<Json<String>, ApiError> {
    user_is_requestor_or_admin(access_claims, subject_uuid)?;

    user_service::delete_one_by_uuid(&state.db_pool, subject_uuid).await?;
    tracing::debug!(%subject_uuid, "user deleted");
    Ok(Json(subject_uuid.to_string()))
}

pub async fn update_password(
    State(state): State<SharedState>,
    Path(subject_uuid): Path<Uuid>,
    access_claims: AccessClaims,
    req_payload: Result<Json<UpdatePasswordRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(parsed_input) =
        req_payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    user_is_requestor_or_admin(access_claims, subject_uuid)?;

    if user_service::update_password_by_uuid(&state.db_pool, parsed_input.password, subject_uuid)
        .await?
    {
        Ok(StatusCode::OK)
    } else {
        Err(ApiError::from(UserError::NotFound(
            subject_uuid.to_string(),
        )))
    }
}

pub async fn create_user(
    State(state): State<SharedState>,
    req_payload: Result<Json<CreateUserRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let Json(parsed_input) =
        req_payload.map_err(|e| UserError::UnprocessableInput(e.to_string()))?;

    let dto: CreateUserParams = parsed_input.into();

    let created = user_service::add_user(&state.db_pool, dto).await?;
    let res = created.into();

    Ok((StatusCode::CREATED, Json(res)))
}

fn user_is_requestor_or_admin(
    access_claims: AccessClaims,
    subject_uuid: Uuid,
) -> Result<(), AuthError> {
    let requestor_uuid = Uuid::parse_str(&access_claims.sub).map_err(|e| {
        tracing::warn!(%e, "failed to parse sub from access claims");
        AuthError::InvalidToken
    })?;

    if requestor_uuid != subject_uuid {
        let is_admin = access_claims.validate_role_admin().is_ok();
        if !is_admin {
            tracing::warn!(%requestor_uuid, %subject_uuid, "non-admin requestor attempting to operate on a user other than themselves");
            return Err(AuthError::Forbidden);
        }
    }
    Ok(())
}
