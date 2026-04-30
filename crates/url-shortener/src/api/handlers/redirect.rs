use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use tracing::instrument;

use crate::{
    api::error::ApiError,
    application::{
        service::{
            analytics::analytics_publisher_trait::{RedirectEvent, RedirectType},
            short_url::short_url_service::RedirectDecision,
        },
        state::SharedState,
    },
    domain::errors::ShortUrlError,
};

#[utoipa::path(
    get,
    path = "/r/{code}",
    tag = "redirect",
    params(
        ("code" = String, Path, description = "Short code to resolve")
    ),
    responses(
        (status = 301, description = "Permanent redirect for GET requests"),
        (status = 302, description = "Temporary redirect for GET requests"),
        (status = 404, description = "Short code was not found", body = ApiError),
        (status = 410, description = "Short code exists but is gone")
    )
)]
#[instrument(skip(state))]
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
        RedirectDecision::Permanent { long_url } => {
            emit_analytics(&state, code.clone(), RedirectType::Permanent);
            match method {
                Method::GET => redirect_response(StatusCode::MOVED_PERMANENTLY, &long_url),
                _ => redirect_response(StatusCode::PERMANENT_REDIRECT, &long_url),
            }
        }
        RedirectDecision::Temporary { long_url } => {
            emit_analytics(&state, code.clone(), RedirectType::Temporary);
            match method {
                Method::GET => redirect_response(StatusCode::FOUND, &long_url),
                _ => redirect_response(StatusCode::TEMPORARY_REDIRECT, &long_url),
            }
        }
    }
}

fn emit_analytics(state: &SharedState, code: String, redirect_type: RedirectType) {
    let publisher = Arc::clone(&state.analytics_publisher);
    let event = RedirectEvent {
        code,
        timestamp: Utc::now(),
        redirect_type,
    };
    tokio::spawn(async move {
        if let Err(e) = publisher.publish(event).await {
            tracing::warn!(%e, "analytics publish failed");
        }
    });
}

fn redirect_response(status: StatusCode, location: &str) -> Result<Response, ApiError> {
    let value = HeaderValue::from_str(location)
        .map_err(|e| ApiError::new(format!("invalid redirect location: {e}")))?;
    Ok((status, [(header::LOCATION, value)]).into_response())
}
