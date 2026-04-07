use std::sync::Arc;

use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use serde::{Deserialize, Serialize};

use crate::{
    api::error::ApiError,
    application::{
        security::{
            auth_error::AuthError,
            jwt::{AccessClaims, ClaimsMethods, JwtTokenType, RefreshClaims, decode_token},
        },
        state::SharedState,
    },
};

impl<S> FromRequestParts<S> for AccessClaims
where
    SharedState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        decode_token_from_request_part(parts, state)
            .await
            .map_err(ApiError::from)
    }
}

impl<S> FromRequestParts<S> for RefreshClaims
where
    SharedState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        decode_token_from_request_part(parts, state)
            .await
            .map_err(ApiError::from)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalAccessClaims(pub Option<AccessClaims>);

impl<S> FromRequestParts<S> for OptionalAccessClaims
where
    SharedState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if parts
            .headers
            .contains_key(axum::http::header::AUTHORIZATION)
        {
            let claims = decode_token_from_request_part(parts, state).await?;
            Ok(OptionalAccessClaims(Some(claims)))
        } else {
            Ok(OptionalAccessClaims(None))
        }
    }
}

async fn decode_token_from_request_part<S, T>(parts: &mut Parts, state: &S) -> Result<T, AuthError>
where
    SharedState: FromRef<S>,
    S: Send + Sync,
    T: for<'de> serde::Deserialize<'de> + std::fmt::Debug + ClaimsMethods + Sync + Send,
{
    // Extract the token from the authorization header.
    let TypedHeader(Authorization(bearer)) = parts
        .extract::<TypedHeader<Authorization<Bearer>>>()
        .await
        .map_err(|_| {
            tracing::debug!("Invalid authorization header");
            AuthError::IncorrectCredentials
        })?;

    // Take the state from a reference.
    let state = Arc::from_ref(state);

    // Decode the token.
    let claims = decode_token::<T>(bearer.token(), &state.jwt_decoding_key)?;

    if !(JwtTokenType::from(claims.get_typ()) == T::EXPECTED_TYPE) {
        tracing::warn!("wrong token type in authorization header");
        return Err(AuthError::InvalidToken);
    }

    Ok(claims)
}
