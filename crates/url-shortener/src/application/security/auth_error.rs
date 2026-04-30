use argon2::password_hash::Error as HashError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("failed to hash password input: {0}")]
    HashingError(HashError),
    #[error("provided credentials incorrect")]
    IncorrectCredentials,
    #[error("no credentials provided")]
    MissingCredentials,
    #[error("failed to create token")]
    TokenCreation,
    #[error("provided token incorrect")]
    InvalidToken,
    #[error("forbidden")]
    Forbidden,
    #[error("token signature has expired")]
    ExpiredSignature(String),
    #[error("internal auth error")]
    Internal,
}
