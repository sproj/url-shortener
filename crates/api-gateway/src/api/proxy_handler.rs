use auth::decode_token;
use axum::http::StatusCode;
use axum::{
    extract::{Request, State},
    response::{IntoResponse, Response},
};

use crate::application::{security::AccessClaims, state::SharedState};

pub async fn proxy_handler(State(state): State<SharedState>, req: Request) -> Response {
    if let Some(auth_value) = req.headers().get(axum::http::header::AUTHORIZATION) {
        let header_str = match auth_value.to_str() {
            Ok(s) => s,
            Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
        };

        let token = match header_str.strip_prefix("Bearer ") {
            Some(t) => t,
            None => return StatusCode::UNAUTHORIZED.into_response(),
        };

        if decode_token::<AccessClaims>(token, &state.jwt_decoding_key).is_err() {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let path = req.uri().path();

    let matched_route = state
        .proxy_routes
        .iter()
        .filter(|r| path_starts_with_prefix(path, &r.prefix))
        .max_by_key(|r| r.prefix.len());

    let route = match matched_route {
        None => return StatusCode::NOT_FOUND.into_response(),
        Some(r) => r,
    };

    let stripped = &path[route.prefix.len()..];
    let upstream_path = if stripped.is_empty() { "/" } else { stripped };

    let upstream_url = match req.uri().query() {
        Some(q) => format!("{}{}?{}", route.target, upstream_path, q),
        None => format!("{}{}", route.target, upstream_path),
    };

    let method = reqwest::Method::from_bytes(req.method().as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let mut upstream = state.http_client.request(method, &upstream_url);

    // forward headers, skipping hop-by-hop headers that must not be forwarded
    for (name, value) in req.headers() {
        if !is_hop_by_hop(name.as_str()) {
            upstream = upstream.header(name.as_str(), value.as_bytes());
        }
    }

    let body = axum::body::to_bytes(req.into_body(), usize::MAX).await;
    let body_bytes = match body {
        Ok(b) => b,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    if !body_bytes.is_empty() {
        upstream = upstream.body(body_bytes);
    }

    let upstream_resp = match upstream.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("upstream request failed: {e}");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let status = StatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut builder = axum::response::Response::builder().status(status);

    for (name, value) in upstream_resp.headers() {
        if !is_hop_by_hop(name.as_str()) {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
    }

    let body_stream = upstream_resp.bytes_stream();
    let body = axum::body::Body::from_stream(body_stream);

    builder.body(body).unwrap()
}

fn path_starts_with_prefix(path: &str, prefix: &str) -> bool {
    if path == prefix {
        return true;
    }
    path.starts_with(prefix) && path[prefix.len()..].starts_with('/')
}

const HOP_BY_HOP_HEADER_NAMES: [&str; 7] = [
    "connection",
    "keep-alive",
    "transfer-encoding",
    "te",
    "trailer",
    "proxy-authorization",
    "upgrade",
];
fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP_HEADER_NAMES.contains(&name)
}
