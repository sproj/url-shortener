use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{
    api::handlers::short_url::{add_one, delete_one_by_uuid, get_all, get_one_by_uuid},
    application::state::SharedState,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_all))
        .route("/", post(add_one))
        .route("/{uuid}", get(get_one_by_uuid))
        .route("/{uuid}", delete(delete_one_by_uuid))
}
