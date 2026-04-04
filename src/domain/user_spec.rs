use std::fmt::Display;

use uuid::Uuid;

/// Data required to persist a new user. All fields are resolved values (password already hashed).
/// The `TryFrom<CreateUserParams>` conversion (which hashes the password) lives in
/// `application::service::user::user_spec` to keep security dependencies out of the domain.
#[derive(Debug)]
pub struct UserSpec {
    pub uuid: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub password_salt: String,
    pub active: bool,
    pub roles: String,
}

impl Display for UserSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "uuid: {}, username: {}, email: {}, active: {}, roles: {}",
            self.uuid, self.username, self.email, self.active, self.roles
        )
    }
}
