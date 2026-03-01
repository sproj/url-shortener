use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{
    api::handlers::short_url_handlers::{
        add_one_shorturl_handler, delete_one_shorturl_handler, get_all_shorturls_handler,
        get_one_shorturl_by_id_handler,
    },
    application::state::SharedState,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_all_shorturls_handler))
        .route("/", post(add_one_shorturl_handler))
        .route("/{id}", get(get_one_shorturl_by_id_handler))
        .route("/{id}", delete(delete_one_shorturl_handler))
}
