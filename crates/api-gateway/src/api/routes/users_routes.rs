use axum::{
    Router,
    routing::{delete, get, post, put},
};

use crate::{
    api::handlers::{
        auth::auth_handlers::user_info,
        users::handlers::{
            create_user, delete_one_by_uuid, get_all, get_one_by_uuid, update_password,
        },
    },
    application::state::SharedState,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(get_all))
        .route("/", post(create_user))
        .route("/{uuid}", get(get_one_by_uuid))
        .route("/{uuid}", delete(delete_one_by_uuid))
        .route("/{uuid}/password", put(update_password))
        .route("/me", get(user_info))
}
