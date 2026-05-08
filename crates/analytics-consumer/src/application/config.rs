use crate::application::startup_error::StartupError;
pub use common::config::{DbConfig, RabbitMqConfig, ServiceConfig};
use common::config::{env_get, env_get_or, env_parse};
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct Config {
    pub app: ServiceConfig,
    pub db: DbConfig,
    pub rabbitmq: RabbitMqConfig,
    pub analytics_queue_name: String,
}

impl Config {
    pub fn service_socket_address(&self) -> Result<SocketAddr, StartupError> {
        use std::str::FromStr;
        SocketAddr::from_str(&format!("{}:{}", self.app.host, self.app.port))
            .map_err(|e| StartupError::Server(e.to_string()))
    }
}

pub fn load() -> Result<Config, StartupError> {
    let env_file = if env_get_or("ENV_TEST", "0") == "1" {
        ".env.test"
    } else {
        ".env"
    };

    match dotenvy::from_filename(env_file) {
        Ok(path) => tracing::info!(%env_file, path = %path.display(), "config loaded from file"),
        Err(err) if err.not_found() => {
            tracing::info!(%env_file, "config file not found, reading from environment");
        }
        Err(err) => {
            tracing::warn!(%env_file, error = %err, "failed to load config file, reading from environment");
        }
    }

    let cfg = Config {
        app: ServiceConfig {
            host: env_get("SERVICE_HOST")?,
            port: env_parse("SERVICE_PORT")?,
        },
        db: DbConfig {
            postgres_user: env_get("POSTGRES_USER")?,
            postgres_password: env_get("POSTGRES_PASSWORD")?,
            postgres_host: env_get("POSTGRES_HOST")?,
            postgres_port: env_parse("POSTGRES_PORT")?,
            postgres_db: env_get("POSTGRES_DB")?,
            postgres_connection_pool: env_parse("POSTGRES_CONNECTION_POOL")?,
        },
        rabbitmq: RabbitMqConfig {
            rabbitmq_host: env_get("RABBITMQ_HOST")?,
            rabbitmq_port: env_parse("RABBITMQ_PORT")?,
            rabbitmq_user: env_get("RABBITMQ_USER")?,
            rabbitmq_password: env_get("RABBITMQ_PASSWORD")?,
            rabbitmq_exchange: env_get("RABBITMQ_EXCHANGE")?,
            redirect_event_routing_key: env_get("REDIRECT_EVENT_ROUTING_KEY")?,
        },
        analytics_queue_name: env_get_or("ANALYTICS_QUEU_NAME", "analytics_redirect_events"),
    };

    Ok(cfg)
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

        dbg!(&result);
        assert!(matches!(result, Err(StartupError::Config(..))));
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
            app: ServiceConfig {
                host: "bad host name with spaces".to_string(),
                port: config.app.port,
            },
            ..config
        };

        let result = invalid.service_socket_address();

        assert!(matches!(result, Err(StartupError::Server(_))));
    }

    fn config_fixture() -> Config {
        Config {
            app: ServiceConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
            },
            db: DbConfig {
                postgres_user: "admin".to_string(),
                postgres_password: "password".to_string(),
                postgres_host: "127.0.0.1".to_string(),
                postgres_port: 5432,
                postgres_db: "url_shortener".to_string(),
                postgres_connection_pool: 5,
            },
            rabbitmq: RabbitMqConfig {
                rabbitmq_exchange: "test_exchange".to_string(),
                rabbitmq_host: "127.0.0.1".to_string(),
                rabbitmq_port: 5612,
                rabbitmq_user: "test_user".to_string(),
                rabbitmq_password: "test_pass".to_string(),
                redirect_event_routing_key: "test_routing_key".to_string(),
            },
            analytics_queue_name: "test_queue".to_string(),
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
