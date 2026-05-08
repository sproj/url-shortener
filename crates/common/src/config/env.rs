use super::ConfigError;

pub fn env_get(key: &str) -> Result<String, ConfigError> {
    match std::env::var(key) {
        Ok(v) => Ok(v),
        Err(e) => {
            let msg = format!("{} {}", key, e);
            Err(ConfigError::Missing(msg))
        }
    }
}

pub fn env_get_or(key: &str, default: &str) -> String {
    if let Ok(v) = std::env::var(key) {
        return v;
    }
    default.to_owned()
}

pub fn env_parse<T>(key: &str) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    env_get(key)?
        .parse::<T>()
        .map_err(|e| ConfigError::Parse(format!("Failed to parse {} : {}", key, e)))
}
