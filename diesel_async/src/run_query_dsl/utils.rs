use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use diesel::QueryResult;
use futures_core::{ready, TryFuture, TryStream};
use futures_util::{TryFutureExt, TryStreamExt};
use pin_project_lite::pin_project;

// We use a custom future implementation here to erase some lifetimes
// that otherwise need to be specified explicitly
//
// Specifying these lifetimes results in the compiler not being
// able to look through the generic code and emit
// lifetime errors for pipelined queries. See
// https://github.com/weiznich/diesel_async/issues/249 for more context
pin_project! {
    #[repr(transparent)]
    pub struct MapOk<F: TryFutureExt, T> {
        #[pin]
        future: futures_util::future::MapOk<F, fn(F::Ok) -> T>,
    }
}

impl<F, T> Future for MapOk<F, T>
where
    F: TryFuture,
    futures_util::future::MapOk<F, fn(F::Ok) -> T>: Future<Output = Result<T, F::Error>>,
{
    type Output = Result<T, F::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.project().future.poll(cx)
    }
}

impl<Fut: TryFutureExt, T> MapOk<Fut, T> {
    pub(crate) fn new(future: Fut, f: fn(Fut::Ok) -> T) -> Self {
        Self {
            future: future.map_ok(f),
        }
    }
}

// similar to `MapOk` above this mainly exists to hide the lifetime
pin_project! {
    #[repr(transparent)]
    pub struct AndThen<F1: TryFuture, F2> {
        #[pin]
        future: futures_util::future::AndThen<F1, F2, fn(F1::Ok) -> F2>,
    }
}

impl<Fut1, Fut2> AndThen<Fut1, Fut2>
where
    Fut1: TryFuture,
    Fut2: TryFuture<Error = Fut1::Error>,
{
    pub(crate) fn new(fut1: Fut1, f: fn(Fut1::Ok) -> Fut2) -> AndThen<Fut1, Fut2> {
        Self {
            future: fut1.and_then(f),
        }
    }
}

impl<F1, F2> Future for AndThen<F1, F2>
where
    F1: TryFuture,
    F2: TryFuture<Error = F1::Error>,
{
    type Output = Result<F2::Ok, F2::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().future.poll(cx)
    }
}

/// Converts a stream into a future, only yielding the first element.
/// Based on [`futures_util::stream::StreamFuture`].
pub struct LoadNext<St> {
    stream: Option<St>,
}

impl<St> LoadNext<St> {
    pub(crate) fn new(stream: St) -> Self {
        Self {
            stream: Some(stream),
        }
    }
}

impl<St> Future for LoadNext<St>
where
    St: TryStream<Error = diesel::result::Error> + Unpin,
{
    type Output = QueryResult<St::Ok>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let first = {
            let s = self.stream.as_mut().expect("polling LoadNext twice");
            ready!(s.try_poll_next_unpin(cx))
        };
        self.stream = None;
        match first {
            Some(first) => Poll::Ready(first),
            None => Poll::Ready(Err(diesel::result::Error::NotFound)),
        }
    }
}
