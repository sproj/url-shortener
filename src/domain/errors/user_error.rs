use argon2::password_hash::Error as HashError;
use thiserror::Error;

use crate::infrastructure::database::database_error::DatabaseError;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("failed to hash password: {0}")]
    HashingError(HashError),
    #[error("user repository error: {0}")]
    Storage(DatabaseError),
}

// impl Display for UserError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "user error")
//     }
// }
