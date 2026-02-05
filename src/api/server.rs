use crate::api::routes::hello_routes;
use axum::{Router, http::StatusCode, response::IntoResponse, extract::Request};
use tokio::net::TcpListener;

pub async fn start() {
    let router = Router::new()
        .nest("/hello", hello_routes::routes())
        .fallback(error_404_handler);

    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, router).await.unwrap();
}

async fn error_404_handler(request: Request) -> impl IntoResponse {
    StatusCode::NOT_FOUND
}
