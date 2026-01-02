use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub enum JoinError {
    Cancelled,
    Panic(String),
    Other(String),
}

impl JoinError {
    pub fn is_cancelled(&self) -> bool {
        matches!(self, JoinError::Cancelled)
    }

    pub fn is_panic(&self) -> bool {
        matches!(self, JoinError::Panic(_))
    }
}

impl std::fmt::Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinError::Cancelled => write!(f, "JoinError::Cancelled"),
            JoinError::Panic(msg) => write!(f, "JoinError::Panic({})", msg),
            JoinError::Other(msg) => write!(f, "JoinError::Other({})", msg),
        }
    }
}

impl std::error::Error for JoinError {}

pub struct JoinHandle<T>
where
    T: Send,
{
    #[cfg(target_arch = "wasm32")]
    handle: tokio::sync::oneshot::Receiver<T>,
    #[cfg(not(target_arch = "wasm32"))]
    handle: tokio::task::JoinHandle<T>,
}

#[cfg(target_arch = "wasm32")]
macro_rules! spawn_impl {
    ($fn:expr) => {
        wasm_bindgen_futures::spawn_local($fn)
    };
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! spawn_impl {
    ($fn:expr) => {
        tokio::spawn($fn)
    };
}

fn panic_payload_to_string(payload: Box<dyn Any + Send + 'static>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_string(),
            Err(_) => "panic payload (non-string)".to_string(),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn map_tokio_join_error(err: tokio::task::JoinError) -> JoinError {
    if err.is_cancelled() {
        return JoinError::Cancelled;
    }
    if err.is_panic() {
        return JoinError::Panic(panic_payload_to_string(err.into_panic()));
    }
    JoinError::Other(format!("{:?}", err))
}

pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + 'static + Send,
    T: Send + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        spawn_impl!(async move {
            let result = future.await;
            let _ = sender.send(result);
        });
        return JoinHandle { handle: receiver };
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = spawn_impl!(future);
        return JoinHandle { handle };
    }
}

impl<T> Future for JoinHandle<T>
where
    T: Send,
{
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        #[cfg(target_arch = "wasm32")]
        {
            match Pin::new(&mut self.handle).poll(cx) {
                Poll::Ready(Ok(val)) => Poll::Ready(Ok(val)),
                Poll::Ready(Err(_)) => Poll::Ready(Err(JoinError::Cancelled)),
                Poll::Pending => Poll::Pending,
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match Pin::new(&mut self.handle).poll(cx) {
                Poll::Ready(Ok(val)) => Poll::Ready(Ok(val)),
                Poll::Ready(Err(err)) => Poll::Ready(Err(map_tokio_join_error(err))),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}
