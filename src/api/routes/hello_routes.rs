use crate::{
    api::handlers::hello_world_handlers::hello_world_handler, application::state::SharedState,
};
use axum::{Router, routing::get};

pub fn routes() -> Router<SharedState> {
    Router::new().route("/", get(hello_world_handler))
}
