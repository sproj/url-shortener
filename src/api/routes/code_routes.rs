use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    api::handlers::codes_handlers::{add_code_handler, list_codes_handler},
    application::state::SharedState,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_codes_handler))
        .route("/", post(add_code_handler))
}
