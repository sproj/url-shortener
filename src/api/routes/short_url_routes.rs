use axum::{
    Router,
    routing::{delete, get, patch, post},
};

use crate::{
    api::handlers::short_url::{
        create_short_url, create_vanity_url, delete_one_by_uuid, get_all, get_one_by_code,
        get_one_by_uuid, update_one_by_uuid,
    },
    application::state::SharedState,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_all))
        .route("/vanity", post(create_vanity_url))
        .route("/", post(create_short_url))
        .route("/uuid/{uuid}", get(get_one_by_uuid))
        .route("/uuid/{uuid}", delete(delete_one_by_uuid))
        .route("/uuid/{uuid}", patch(update_one_by_uuid))
        .route("/code/{code}", get(get_one_by_code))
}
