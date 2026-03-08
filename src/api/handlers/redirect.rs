use axum::{
    extract::{Path, State},
    response::Redirect,
};
use hyper::StatusCode;

use crate::{
    api::{error::ApiError, handlers::short_url::ShortUrlError},
    application::state::SharedState,
};

pub async fn redirect(
    State(state): State<SharedState>,
    Path(code): Path<String>,
) -> Result<(StatusCode, Redirect), ApiError> {
    let record = state.short_url.get_by_code(code.clone()).await?;
    if let Some(short) = record {
        Ok((
            StatusCode::TEMPORARY_REDIRECT,
            Redirect::temporary(&short.long_url),
        ))
    } else {
        Err(ApiError::from(ShortUrlError::NotFound(code)))
    }
}
