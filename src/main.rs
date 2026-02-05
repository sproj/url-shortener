use axum::{
    Router, extract::Request, http::StatusCode, response::IntoResponse, routing::get, routing::post,
};
use url_shortener::application::app;

const MAPPINGS: [(&'static str, &'static str); 1] = [("goo.gl", "http://www.google.com")];

#[tokio::main]
async fn main() {
    // let app = Router::new()
    //     .route("/", get(hello_world))
    //     .route("shorten", post(hello_world))
    //     .fallback(error_404_handler);

    // let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    // axum::serve(listener, app).await.unwrap();
    app::run().await;
}

async fn hello_world() -> &'static str {
    "hello world"
}

async fn short_to_long_url(user_input: &str) -> Option<&str> {
    for (short, long) in MAPPINGS {
        if user_input == short {
            return Some(long);
        }
    }
    None
}
