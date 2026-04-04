use axum::{
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{
    api::error::ApiError,
    application::{service::short_url::short_url_service::RedirectDecision, state::SharedState},
    domain::errors::ShortUrlError,
};

pub async fn redirect(
    State(state): State<SharedState>,
    Path(code): Path<String>,
    method: Method,
) -> Result<Response, ApiError> {
    let decision = state
        .short_url_service
        .resolve_redirect_decision(&code)
        .await?;

    match decision {
        RedirectDecision::Gone => Ok(StatusCode::GONE.into_response()),
        RedirectDecision::NotFound => Err(ApiError::from(ShortUrlError::NotFound(code))),
        RedirectDecision::Permanent { long_url } => match method {
            Method::GET => redirect_response(StatusCode::MOVED_PERMANENTLY, &long_url),
            _ => redirect_response(StatusCode::PERMANENT_REDIRECT, &long_url),
        },
        RedirectDecision::Temporary { long_url } => match method {
            Method::GET => redirect_response(StatusCode::FOUND, &long_url),
            _ => redirect_response(StatusCode::TEMPORARY_REDIRECT, &long_url),
        },
    }
}

fn redirect_response(status: StatusCode, location: &str) -> Result<Response, ApiError> {
    let value = HeaderValue::from_str(location)
        .map_err(|e| ApiError::new(format!("invalid redirect location: {e}")))?;
    Ok((status, [(header::LOCATION, value)]).into_response())
}
