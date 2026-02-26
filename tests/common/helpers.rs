use std::sync::OnceLock;
use url_shortener::application::config::Config;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn config() -> &'static Config {
    CONFIG.get().unwrap()
}
