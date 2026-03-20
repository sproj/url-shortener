use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub password: String,
}
