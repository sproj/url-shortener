use crate::{
    api::routes::{code_routes, hello_routes},
    application::{config::Config, startup_error::StartupError, state::SharedState},
};
use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde_json::json;
use tokio::{net::TcpListener, signal};

pub async fn start(config: Config, state: SharedState) {
    let listener = listen(config).await.unwrap();

    serve(listener, state).await
}

pub async fn listen(config: Config) -> Result<TcpListener, StartupError> {
    let addr = config.service_socket_address();
    TcpListener::bind(addr)
        .await
        .map_err(|e| StartupError::Server(e.to_string()))
}

pub async fn serve(listener: TcpListener, state: SharedState) {
    let router = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .nest("/hello", hello_routes::routes())
        .nest("/shorten", code_routes::routes())
        .fallback(error_404_handler)
        .with_state(state);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap()
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

// ready handler
async fn ready_handler(State(state): State<SharedState>) -> StatusCode {
    match state.db_pool.get().await {
        Ok(client) => {
            if client.query_one("SELECT 1", &[]).await.is_ok() {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            }
        }
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
