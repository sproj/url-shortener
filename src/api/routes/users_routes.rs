use axum::{Router, routing::post};

use crate::{api::handlers::users::handlers::create_user, application::state::SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new().route("/", post(create_user))
}
