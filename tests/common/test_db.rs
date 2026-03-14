use std::sync::{Arc, Mutex, Weak};

use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use url_shortener::application::config::DbConfig;

use crate::common::shared_container::{SharedContainer, get_or_create_shared_container};

pub type SharedTestDb = SharedContainer<Postgres, DbConfig>;
static CONTAINER: Mutex<Weak<SharedTestDb>> = Mutex::new(Weak::new());

pub async fn get_or_create() -> Arc<SharedContainer<Postgres, DbConfig>> {
    get_or_create_shared_container(&CONTAINER, async move || {
        let container = Postgres::default()
            .with_db_name("test_postgres")
            .with_user("admin")
            .with_password("password")
            .start()
            .await
            .unwrap();
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let id = container.id().to_string();
        let dbconfig = DbConfig {
            postgres_port: port,
            postgres_connection_pool: 5,
            postgres_db: "test_postgres".to_string(),
            postgres_host: container.get_host().await.unwrap().to_string(),
            postgres_password: "password".to_string(),
            postgres_user: "admin".to_string(),
        };
        (id, container, dbconfig)
    })
    .await
}

// pub async fn get_or_create() -> Arc<SharedTestDb> {
//     {
//         let weak = CONTAINER.lock().unwrap();
//         if let Some(arc) = weak.upgrade() {
//             return arc;
//         }
//     }

//     let (tx, rx) = tokio::sync::oneshot::channel();

//     std::thread::spawn(move || {
//         let rt = tokio::runtime::Builder::new_multi_thread()
//             .enable_all()
//             .build()
//             .unwrap();

//         let (port, container_id, container) = rt.block_on(async {
//             let container = Postgres::default()
//                 .with_db_name("url_shortener_test")
//                 .with_user("admin")
//                 .with_password("password")
//                 .start()
//                 .await
//                 .unwrap();

//             let port = container.get_host_port_ipv4(5432).await.unwrap();
//             let id = container.id().to_string();
//             (port, id, container)
//         });

//         let arc = Arc::new(SharedTestDb {
//             host: "127.0.0.1".into(),
//             port,
//             user: "admin".into(),
//             password: "password".into(),
//             db_name: "url_shortener_test".into(),
//             container_id,
//             _container: ManuallyDrop::new(container),
//             _rt: ManuallyDrop::new(rt),
//         });

//         tx.send(arc).ok();
//     });

//     let new_arc = rx.await.expect("container init thread panicked");

//     let mut weak = CONTAINER.lock().unwrap();
//     if let Some(arc) = weak.upgrade() {
//         // Another task won the race; ours will drop, cleaning up the extra container.
//         return arc;
//     }
//     *weak = Arc::downgrade(&new_arc);
//     new_arc
// }
