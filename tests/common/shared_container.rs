use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex, Weak};
use testcontainers::{ContainerAsync, Image};

pub struct SharedContainer<I: Image, C: Send> {
    pub config: C,
    container_id: String,
    _container: ManuallyDrop<testcontainers::ContainerAsync<I>>,
    _rt: ManuallyDrop<tokio::runtime::Runtime>,
}

impl<I: Image, C: Send> Drop for SharedContainer<I, C> {
    fn drop(&mut self) {
        std::process::Command::new("docker")
            .args(["rm", "fv", &self.container_id])
            .output()
            .ok();
    }
}

pub type ContainerBootstrapOutput<I: Image> = (u16, String, ContainerAsync<I>);
pub type ContainerBootstrap<I: Image> = fn(image: I) -> ContainerBootstrapOutput<I>;

pub async fn get_or_create<I: Image, C, F, Fut>(
    storage: &'static Mutex<Weak<SharedContainer<I, C>>>,
    container_config: C,
    start_container: F,
) -> Arc<SharedContainer<I, C>>
where
    I: Image,
    C: Send + Sync,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ContainerBootstrapOutput<I>> + Send,
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
        let (port, container_id, container) = rt.block_on(start_container_future);

        let arc = Arc::new(SharedContainer {
            config: container_config,
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
