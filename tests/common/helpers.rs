use std::sync::OnceLock;
use url_shortener::application::config::Config;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn config() -> &'static Config {
    CONFIG.get().unwrap()
}

pub fn build_path(path: &str) -> reqwest::Url {
    let url = format!("{}/{}", config().service_http_address(), path);
    dbg!("building url: {}", &url);
    reqwest::Url::parse(&url).unwrap()
}
