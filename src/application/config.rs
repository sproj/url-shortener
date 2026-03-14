use std::net::SocketAddr;

use crate::application::startup_error::StartupError;

#[derive(Clone, Debug)]
pub struct Config {
    // Rest API configuration
    pub app: AppConfig,
    // PostgreSQL configuration
    pub db: DbConfig,
    // Redis configuration.
    pub redis: RedisConfig,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub service_host: String,
    pub service_port: u16,
}

#[derive(Clone, Debug)]
pub struct DbConfig {
    pub postgres_user: String,
    pub postgres_password: String,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_db: String,
    pub postgres_connection_pool: u32,
}

#[derive(Clone, Debug)]
pub struct RedisConfig {
    pub redis_host: String,
    pub redis_port: u16,
}

impl Config {
    pub fn service_socket_address(&self) -> Result<SocketAddr, StartupError> {
        use std::str::FromStr;
        SocketAddr::from_str(&format!(
            "{}:{}",
            self.app.service_host, self.app.service_port
        ))
        .map_err(|e| StartupError::Server(e.to_string()))
    }

    pub fn redis_url(&self) -> String {
        format!(
            "redis://{}:{}",
            self.redis.redis_host, self.redis.redis_port
        )
    }
}

pub fn load() -> Result<Config, StartupError> {
    let env_file = if env_get_or("ENV_TEST", "0") == "1" {
        ".env.test"
    } else {
        ".env"
    };

    // Try to load environment variables from file.
    if dotenvy::from_filename(env_file).is_ok() {
        tracing::info!(%env_file, "config found");
    } else {
        let config_not_found = format!("{} file not found", env_file);
        tracing::error!(%env_file, "config file not found");
        return Err(StartupError::Config(config_not_found));
    }

    let cfg = Config {
        app: AppConfig {
            service_host: env_get("SERVICE_HOST")?,
            service_port: env_parse("SERVICE_PORT")?,
        },
        db: DbConfig {
            postgres_user: env_get("POSTGRES_USER")?,
            postgres_password: env_get("POSTGRES_PASSWORD")?,
            postgres_host: env_get("POSTGRES_HOST")?,
            postgres_port: env_parse("POSTGRES_PORT")?,
            postgres_db: env_get("POSTGRES_DB")?,
            postgres_connection_pool: env_parse("POSTGRES_CONNECTION_POOL")?,
        },
        redis: RedisConfig {
            redis_host: env_get("REDIS_HOST")?,
            redis_port: env_parse("REDIS_PORT")?,
        },
    };

    Ok(cfg)
}

fn env_get(key: &str) -> Result<String, StartupError> {
    match std::env::var(key) {
        Ok(v) => Ok(v),
        Err(e) => {
            let msg = format!("{} {}", key, e);
            Err(StartupError::Config(msg))
        }
    }
}

fn env_get_or(key: &str, default: &str) -> String {
    if let Ok(v) = std::env::var(key) {
        return v;
    }
    default.to_owned()
}

fn env_parse<T>(key: &str) -> Result<T, StartupError>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    env_get(key)?
        .parse::<T>()
        .map_err(|e| StartupError::Config(format!("Failed to parse {} : {}", key, e)))
}
