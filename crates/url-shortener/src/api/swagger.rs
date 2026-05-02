use serde::{Deserialize, Serialize};
use utoipa::{Modify, OpenApi, ToSchema};

use crate::{
    api::{
        error::ApiError,
        handlers::{
            redirect,
            short_url::short_url_handlers,
            short_url::{
                CreateShortUrlRequest, CreateShortUrlResponse, CreateVanityUrlRequest,
                UpdateShortUrlRequest,
            },
        },
        server,
    },
    domain::{models::short_url::ShortUrl, validation_issue::ValidationIssue},
};
use auth::jwt::JwtTokens;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
}

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};

        let components = openapi.components.get_or_insert_default();
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        short_url_handlers::get_all,
        short_url_handlers::get_one_by_uuid,
        short_url_handlers::create_short_url,
        short_url_handlers::create_vanity_url,
        short_url_handlers::update_one_by_uuid,
        short_url_handlers::delete_one_by_uuid,
        short_url_handlers::get_one_by_code,
        redirect::redirect,
        server::health_handler,
        server::ready_handler
    ),
    components(
        schemas(
            ApiError,
            ValidationIssue,
            JwtTokens,
            StatusResponse,
            CreateShortUrlRequest,
            CreateVanityUrlRequest,
            UpdateShortUrlRequest,
            CreateShortUrlResponse,
            ShortUrl,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication and token lifecycle"),
        (name = "short-url", description = "Short URL management"),
        (name = "users", description = "User management"),
        (name = "redirect", description = "Short code redirect resolution"),
        (name = "system", description = "Service health endpoints")
    ),
    info(
        title = "URL Shortener API",
        version = "0.1.0",
        description = "HTTP API for managing users, issuing JWTs, creating short URLs, and resolving redirects."
    )
)]
pub struct ApiDoc;
