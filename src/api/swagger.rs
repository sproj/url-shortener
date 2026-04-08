use serde::{Deserialize, Serialize};
use utoipa::{Modify, OpenApi, ToSchema};

use crate::{
    api::{
        error::ApiError,
        handlers::{
            auth::{auth_handlers, login_request::LoginRequest},
            redirect,
            short_url::{
                CreateShortUrlRequest, CreateShortUrlResponse, CreateVanityUrlRequest,
                UpdateShortUrlRequest,
            },
            users::{
                create_user_request::CreateUserRequest, handlers as user_handlers,
                update_password_request::UpdatePasswordRequest, user_response::UserResponse,
            },
        },
        server,
    },
    application::security::jwt::JwtTokens,
    domain::{models::short_url::ShortUrl, validation_issue::ValidationIssue},
};

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
        auth_handlers::login,
        auth_handlers::logout,
        auth_handlers::refresh,
        crate::api::handlers::short_url::handlers::get_all,
        crate::api::handlers::short_url::handlers::get_one_by_uuid,
        crate::api::handlers::short_url::handlers::create_short_url,
        crate::api::handlers::short_url::handlers::create_vanity_url,
        crate::api::handlers::short_url::handlers::update_one_by_uuid,
        crate::api::handlers::short_url::handlers::delete_one_by_uuid,
        crate::api::handlers::short_url::handlers::get_one_by_code,
        user_handlers::get_all,
        user_handlers::get_one_by_uuid,
        user_handlers::delete_one_by_uuid,
        user_handlers::update_password,
        user_handlers::create_user,
        redirect::redirect,
        server::health_handler,
        server::ready_handler
    ),
    components(
        schemas(
            ApiError,
            ValidationIssue,
            LoginRequest,
            LoginResponse,
            JwtTokens,
            StatusResponse,
            CreateShortUrlRequest,
            CreateVanityUrlRequest,
            UpdateShortUrlRequest,
            CreateShortUrlResponse,
            ShortUrl,
            CreateUserRequest,
            UpdatePasswordRequest,
            UserResponse
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
