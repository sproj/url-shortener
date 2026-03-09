use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{
    api::handlers::short_url::{
        add_one, delete_one_by_id, get_all, get_one_by_code, get_one_by_id,
    },
    application::state::SharedState,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_all))
        .route("/", post(add_one))
        .route("/{id}", get(get_one_by_id))
        .route("/{id}", delete(delete_one_by_id))
        .route("/getByCode/{code}", get(get(get_one_by_code)))
}
