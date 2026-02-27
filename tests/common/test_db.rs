use std::sync::LazyLock;

use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::postgres::{self, Postgres};

pub struct SharedTestDb {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub db_name: String,
    _container: ContainerAsync<Postgres>,
    _rt: tokio::runtime::Runtime,
}

pub static SHARED_POSTGRES: LazyLock<SharedTestDb> = LazyLock::new(|| {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let (port, container) = rt.block_on(async {
            let container = postgres::Postgres::default()
                .with_db_name("url_shortener_test")
                .with_user("admin")
                .with_password("password")
                .start()
                .await
                .unwrap();

            let port = container.get_host_port_ipv4(5432).await.unwrap();

            (port, container)
        });

        SharedTestDb {
            host: "127.0.0.1".into(),
            port,
            user: "admin".into(),
            password: "password".into(),
            db_name: "url_shortener_test".into(),
            _container: container,
            _rt: rt,
        }
    })
    .join()
    .unwrap()
});
