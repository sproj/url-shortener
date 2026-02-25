#[derive(Debug)]
pub enum StartupError {
    Config(String),
    Db(String),
    Server(String)
}