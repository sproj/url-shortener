use axum::{Router, routing::any};

use crate::{api::handlers::redirect::redirect, application::state::SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new().route("/{code}", any(redirect))
}
