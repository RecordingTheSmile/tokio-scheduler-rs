use std::future::Future;
use std::pin::Pin;

pub(crate) type PinFutureResult<T,E> = Pin<Box<dyn Future<Output = Result<T,E>>>>;
pub(crate) type PinFuture<T> = Pin<Box<dyn Future<Output = T>>>;
pub(crate) type FutureResult<T,E> = dyn Future<Output=Result<T, E>>;
