use serde::{Deserialize, Serialize};
use utoipa::{Modify, OpenApi, ToSchema};

use crate::{
    api::{
        error::ApiError,
        handlers::{
            auth::{auth_handlers, login_request::LoginRequest},
            users::{
                create_user_request::CreateUserRequest, handlers as user_handlers,
                update_password_request::UpdatePasswordRequest, user_response::UserResponse,
            },
        },
        server,
    },
    domain::validation_issue::ValidationIssue,
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
        auth_handlers::login,
        auth_handlers::logout,
        auth_handlers::refresh,
        user_handlers::get_all,
        user_handlers::get_one_by_uuid,
        user_handlers::delete_one_by_uuid,
        user_handlers::update_password,
        user_handlers::create_user,
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
            CreateUserRequest,
            UpdatePasswordRequest,
            UserResponse
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication and token lifecycle"),
        (name = "users", description = "User management"),
        (name = "system", description = "Service health endpoints")
    ),
    info(
        title = "Api-Gateway API",
        version = "0.1.0",
        description = "HTTP API for managing users, issuing JWTs, creating short URLs, and resolving redirects."
    )
)]
pub struct ApiDoc;
