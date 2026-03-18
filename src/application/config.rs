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
        tracing::info!(%env_file, "config loaded from file");
    } else {
        tracing::info!(%env_file, "config file not found, reading from environment");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn load_fails_when_required_env_var_is_empty_for_numeric_field() {
        let _guard = lock_env();
        let _env_test = EnvVarGuard::set("ENV_TEST", Some("1"));
        let _invalid = EnvVarGuard::set("POSTGRES_PORT", Some(""));

        let result = load();

        assert!(
            matches!(result, Err(StartupError::Config(msg)) if msg.contains("Failed to parse POSTGRES_PORT"))
        );
    }

    #[test]
    fn load_fails_when_env_var_cannot_be_parsed() {
        let _guard = lock_env();
        let _env_test = EnvVarGuard::set("ENV_TEST", Some("1"));
        let _invalid = EnvVarGuard::set("SERVICE_PORT", Some("not-a-number"));

        let result = load();

        assert!(
            matches!(result, Err(StartupError::Config(msg)) if msg.contains("Failed to parse SERVICE_PORT"))
        );
    }

    #[test]
    fn service_socket_address_returns_error_for_invalid_host() {
        let config = config_fixture();
        let invalid = Config {
            app: AppConfig {
                service_host: "bad host name with spaces".to_string(),
                service_port: config.app.service_port,
            },
            ..config
        };

        let result = invalid.service_socket_address();

        assert!(matches!(result, Err(StartupError::Server(_))));
    }

    fn config_fixture() -> Config {
        Config {
            app: AppConfig {
                service_host: "127.0.0.1".to_string(),
                service_port: 0,
            },
            db: DbConfig {
                postgres_user: "admin".to_string(),
                postgres_password: "password".to_string(),
                postgres_host: "127.0.0.1".to_string(),
                postgres_port: 5432,
                postgres_db: "url_shortener".to_string(),
                postgres_connection_pool: 5,
            },
            redis: RedisConfig {
                redis_host: "127.0.0.1".to_string(),
                redis_port: 6379,
            },
        }
    }

    fn lock_env() -> MutexGuard<'static, ()> {
        match ENV_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    struct EnvVarGuard {
        key: String,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &str, value: Option<&str>) -> Self {
            let original = std::env::var(key).ok();

            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }

            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(value) => std::env::set_var(&self.key, value),
                    None => std::env::remove_var(&self.key),
                }
            }
        }
    }
}
