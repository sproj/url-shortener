use axum::{Json, response::IntoResponse};
use jsonwebtoken::{DecodingKey, EncodingKey, errors::ErrorKind};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;

use crate::application::{constants::USER_ROLE_ADMIN, security::auth_error::AuthError};

#[derive(Clone)]
pub struct JwtKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl std::fmt::Debug for JwtKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtKeys").finish()
    }
}

impl JwtKeys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

pub struct JwtTokens {
    pub access_token: String,
}
pub fn tokens_to_response(jwt_tokens: JwtTokens) -> impl IntoResponse {
    let json = json!({
        "access_token": jwt_tokens.access_token,
        "token_type": "Bearer"
    });
    Json(json)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub aud: String,
    pub iss: String,
    pub iat: usize,
    pub exp: usize,
    pub jti: String,
    pub roles: String,
}

impl Display for AccessClaims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "sub: {}, iss: {}, aud: {}, iat: {}, exp: {}",
            self.sub, self.iss, self.aud, self.iat, self.exp
        )
    }
}

pub trait ClaimsMethods {
    fn validate_role_admin(&self) -> Result<(), AuthError>;
    fn get_sub(&self) -> &str;
    fn get_exp(&self) -> usize;
    fn get_iat(&self) -> usize;
    fn get_jti(&self) -> &str;
}

impl ClaimsMethods for AccessClaims {
    fn validate_role_admin(&self) -> Result<(), AuthError> {
        if self
            .roles
            .split(',')
            .any(|role| role.trim().eq(USER_ROLE_ADMIN))
        {
            Ok(())
        } else {
            tracing::warn!("admin action attempted without admin privilege");
            Err(AuthError::Forbidden)
        }
    }
    fn get_sub(&self) -> &str {
        &self.sub
    }

    fn get_iat(&self) -> usize {
        self.iat
    }

    fn get_exp(&self) -> usize {
        self.exp
    }

    fn get_jti(&self) -> &str {
        &self.jti
    }
}

pub fn decode_token<T: for<'de> serde::Deserialize<'de>>(
    token: &str,
    decoding_key: &DecodingKey,
) -> Result<T, AuthError> {
    let mut validation = jsonwebtoken::Validation::default();
    // validation.leeway = config.jwt.jwt_validation_leeway_seconds as u64;
    // todo: reckon hardcoding better than putting jwt config on State - think it through.
    validation.leeway = 60u64;
    validation.set_audience(&["url-shortener"]);
    validation.set_issuer(&["url-shortener"]);

    let token_data = jsonwebtoken::decode::<T>(token, decoding_key, &validation).map_err(|e| {
        tracing::warn!(%e, "token validation rejected");
        match e.kind() {
            ErrorKind::ExpiredSignature => AuthError::ExpiredSignature(e.to_string()),
            _ => AuthError::InvalidToken,
        }
    })?;

    Ok(token_data.claims)
}
