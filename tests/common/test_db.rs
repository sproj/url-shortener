use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex, Weak};

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

pub struct SharedTestDb {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub db_name: String,
    container_id: String,
    _container: ManuallyDrop<testcontainers::ContainerAsync<Postgres>>,
    _rt: ManuallyDrop<tokio::runtime::Runtime>,
}

impl Drop for SharedTestDb {
    fn drop(&mut self) {
        std::process::Command::new("docker")
            .args(["rm", "-fv", &self.container_id])
            .output()
            .ok();
    }
}

static CONTAINER: Mutex<Weak<SharedTestDb>> = Mutex::new(Weak::new());

pub async fn get_or_create() -> Arc<SharedTestDb> {
    {
        let weak = CONTAINER.lock().unwrap();
        if let Some(arc) = weak.upgrade() {
            return arc;
        }
    }

    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let (port, container_id, container) = rt.block_on(async {
            let container = Postgres::default()
                .with_db_name("url_shortener_test")
                .with_user("admin")
                .with_password("password")
                .start()
                .await
                .unwrap();

            let port = container.get_host_port_ipv4(5432).await.unwrap();
            let id = container.id().to_string();
            (port, id, container)
        });

        let arc = Arc::new(SharedTestDb {
            host: "127.0.0.1".into(),
            port,
            user: "admin".into(),
            password: "password".into(),
            db_name: "url_shortener_test".into(),
            container_id,
            _container: ManuallyDrop::new(container),
            _rt: ManuallyDrop::new(rt),
        });

        tx.send(arc).ok();
    });

    let new_arc = rx.await.expect("container init thread panicked");

    let mut weak = CONTAINER.lock().unwrap();
    if let Some(arc) = weak.upgrade() {
        // Another task won the race; ours will drop, cleaning up the extra container.
        return arc;
    }
    *weak = Arc::downgrade(&new_arc);
    new_arc
}
