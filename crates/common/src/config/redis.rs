#[derive(Clone, Debug)]
pub struct RedisConfig {
    pub redis_host: String,
    pub redis_port: u16,
}
