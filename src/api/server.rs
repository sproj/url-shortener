use crate::{
    api::routes::{redirect_routes, short_url_routes},
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

pub async fn start(config: Config, state: SharedState) -> Result<(), StartupError> {
    let listener = listen(config).await?;

    serve(listener, state).await
}

pub async fn listen(config: Config) -> Result<TcpListener, StartupError> {
    let addr = config.service_socket_address()?;
    TcpListener::bind(addr)
        .await
        .map_err(|e| StartupError::Server(e.to_string()))
}

pub async fn serve(listener: TcpListener, state: SharedState) -> Result<(), StartupError> {
    let router = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .nest("/shorten", short_url_routes::routes())
        .nest("/r", redirect_routes::routes())
        .fallback(error_404_handler)
        .with_state(state);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| StartupError::Server(e.to_string()))
}

// health request handler
async fn health_handler() -> Result<impl IntoResponse, ()> {
    Ok(Json(json!({"status": "healthy"})))
}

// 404 handler
async fn error_404_handler(request: Request) -> impl IntoResponse {
    tracing::warn!(method = %request.method(), path = %request.uri().path(), "route not found");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::config::{AppConfig, Config, DbConfig, RedisConfig};
    use std::net::TcpListener as StdTcpListener;

    #[tokio::test]
    async fn listen_returns_server_error_for_address_in_use() {
        let occupied = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let port = occupied.local_addr().unwrap().port();

        let config = Config {
            app: AppConfig {
                service_host: "127.0.0.1".to_string(),
                service_port: port,
            },
            db: DbConfig {
                postgres_user: "admin".to_string(),
                postgres_password: "password".to_string(),
                postgres_host: "127.0.0.1".to_string(),
                postgres_port: 5432,
                postgres_db: "url_shortener".to_string(),
                postgres_connection_pool: 5,
            },
            redis: RedisConfig {
                redis_host: "127.0.0.1".to_string(),
                redis_port: 6379,
            },
        };

        let result = listen(config).await;

        assert!(matches!(result, Err(StartupError::Server(_))));
    }
}
