use crate::{api::routes::hello_routes, application::config::Config};
use axum::{
    Json, Router, extract::Request, http::StatusCode, response::IntoResponse, routing::get,
};
use serde_json::json;
use tokio::net::TcpListener;

pub async fn start(config: Config) {
    let router = Router::new()
        .route("/health", get(health_handler))
        .nest("/hello", hello_routes::routes())
        .fallback(error_404_handler);

    let addr = config.service_socket_address();
    let listener = TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, router).await.unwrap();
}

// health request handler
async fn health_handler() -> Result<impl IntoResponse, ()> {
    Ok(Json(json!({"status": "healthy"})))
}

// 404 handler
async fn error_404_handler(request: Request) -> impl IntoResponse {
    println!("route not found: {:?}", request);
    StatusCode::NOT_FOUND
}
