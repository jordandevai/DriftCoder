use std::future::Future;
use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

static SSH_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    SSH_RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_all()
            .thread_name("driftcode-ssh")
            .build()
            .expect("failed to build driftcode SSH runtime")
    })
}

pub fn spawn<F, T>(future: F) -> tokio::task::JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    get_runtime().spawn(future)
}

