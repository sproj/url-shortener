#![allow(dead_code)]
use std::sync::{Arc, Mutex, Weak};

use crate::common::shared_container::{SharedContainer, get_or_create_shared_container};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::redis::Redis;
use url_shortener::application::config::RedisConfig;

pub type SharedTestRedis = SharedContainer<Redis, RedisConfig>;
static CONTAINER: Mutex<Weak<SharedTestRedis>> = Mutex::new(Weak::new());

pub async fn get_or_create() -> Arc<SharedContainer<Redis, RedisConfig>> {
    get_or_create_shared_container(&CONTAINER, async move || {
        let container = Redis::default().start().await.unwrap();

        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(6379).await.unwrap();
        let id = container.id().to_string();
        let dbconfig = RedisConfig {
            redis_port: port,
            redis_host: host.to_string(),
        };
        (id, container, dbconfig)
    })
    .await
}
