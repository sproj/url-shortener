use jsonwebtoken::{DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth_error::AuthError;

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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct JwtTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum JwtTokenType {
    AccessToken,
    RefreshToken,
    UnknownToken,
}

impl From<u8> for JwtTokenType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::AccessToken,
            1 => Self::RefreshToken,
            _ => Self::UnknownToken,
        }
    }
}

pub trait ClaimsMethods {
    const EXPECTED_TYPE: JwtTokenType;
    fn validate_role_admin(&self) -> Result<(), AuthError>;
    fn get_sub(&self) -> &str;
    fn get_exp(&self) -> usize;
    fn get_iat(&self) -> usize;
    fn get_jti(&self) -> &str;
    fn get_typ(&self) -> u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_token_type_from_maps_expected_variants() {
        assert_eq!(JwtTokenType::from(0), JwtTokenType::AccessToken);
        assert_eq!(JwtTokenType::from(1), JwtTokenType::RefreshToken);
        assert_eq!(JwtTokenType::from(9), JwtTokenType::UnknownToken);
    }
}
