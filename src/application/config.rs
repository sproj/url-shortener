use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct Config {
    pub service_host: String,
    pub service_port: u16,
    // pub postgres_user: String,
    // pub postgres_password: String,
    // pub postgres_host: String,
    // pub postgres_port: u16,
    // pub postgres_db: String,
    // pub postgres_connection_pool: u32,
}

impl Config {
    pub fn service_host(&self) -> String {
        format!("{}://{}", "http", self.service_host)
    }

    pub fn service_http_address(&self) -> String {
        format!("{}://{}:{}", "http", self.service_host, self.service_port)
    }

    pub fn service_socket_address(&self) -> SocketAddr {
        use std::str::FromStr;
        SocketAddr::from_str(&format!("{}:{}", self.service_host, self.service_port)).unwrap()
    }
}

pub fn load() -> Config {
    let env_file = if env_get_or("ENV_TEST", "0") == "1" {
        ".env_test"
    } else {
        ".env"
    };

    // Try to load environment variables from file.
    if dotenvy::from_filename(env_file).is_ok() {
        println!("{} file loaded", env_file);
    } else {
        println!("{} file not found, using existing environment", env_file);
    }

    let config = Config {
        service_host: env_get("SERVICE_HOST"),
        service_port: env_parse("SERVICE_PORT"),
    };

    config
}

fn env_get(key: &str) -> String {
    match std::env::var(key) {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("{} {}", key, e);
            panic!("{msg}")
        }
    }
}

fn env_get_or(key: &str, default: &str) -> String {
    if let Ok(v) = std::env::var(key) {
        return v;
    }
    default.to_owned()
}

fn env_parse<T: std::str::FromStr>(key: &str) -> T {
    env_get(key).parse().map_or_else(
        |_| {
            let msg = format!("Failed to parse: {}", key);
            panic!("{msg}")
        },
        |v| v,
    )
}
