use crate::api::handlers::hello_world_handlers::hello_world_handler;
use axum::{Router, routing::get};
pub fn routes() -> Router {
    Router::new().route("/", get(hello_world_handler))
}
