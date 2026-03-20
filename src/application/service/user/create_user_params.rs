#[derive(Debug)]
pub struct CreateUserParams {
    pub username: String,
    pub email: String,
    pub password: String,
    pub roles: String,
    pub active: bool,
}
