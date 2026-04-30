use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex, Weak};
use testcontainers::{ContainerAsync, Image};

pub struct SharedContainer<I: Image, C: Send + Clone + Sync> {
    pub config: C,
    container_id: String,
    _container: ManuallyDrop<testcontainers::ContainerAsync<I>>,
    _rt: ManuallyDrop<tokio::runtime::Runtime>,
}

impl<I: Image, C: Send + Clone + Sync> Drop for SharedContainer<I, C> {
    fn drop(&mut self) {
        let drop_result = std::process::Command::new("docker")
            .args(["rm", "-fv", &self.container_id])
            .output();

        if let Err(e) = drop_result {
            eprintln!("Failed to remove container {}: {}", self.container_id, e)
        }
    }
}

pub type ContainerBootstrapOutput<I, C> = (String, ContainerAsync<I>, C);
// pub type ContainerBootstrap<I, C> = fn(image: I) -> ContainerBootstrapOutput<I, C>;

pub async fn get_or_create_shared_container<I: Image, C: Send + Clone + Sync, F, Fut>(
    storage: &'static Mutex<Weak<SharedContainer<I, C>>>,
    start_container: F,
) -> Arc<SharedContainer<I, C>>
where
    I: Image,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ContainerBootstrapOutput<I, C>> + Send,
{
    {
        let weak = storage.lock().unwrap();
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

        let start_container_future = start_container();
        let (container_id, container, config) = rt.block_on(start_container_future);

        let arc = Arc::new(SharedContainer {
            config: config.clone(),
            _container: ManuallyDrop::new(container),
            _rt: ManuallyDrop::new(rt),
            container_id,
        });

        tx.send(arc).ok();
    });
    let new_arc = rx.await.expect("container init thread panicked");

    let mut weak = storage.lock().unwrap();
    if let Some(arc) = weak.upgrade() {
        // Another task won the race; ours will drop, cleaning up the extra container.
        return arc;
    }
    *weak = Arc::downgrade(&new_arc);
    new_arc
}
