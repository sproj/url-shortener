use std::sync::OnceLock;
use url_shortener::application::config::Config;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

pub static ADDR: OnceLock<String> = OnceLock::new();

pub fn config() -> &'static Config {
    CONFIG.get().unwrap()
}

pub fn addr() -> &'static String {
    ADDR.get().unwrap()
}

pub fn build_path(path: &str) -> reqwest::Url {
    let url = format!("{}/{}", addr(), path);
    dbg!("building url: {}", &url);
    reqwest::Url::parse(&url).unwrap()
}
