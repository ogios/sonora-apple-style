use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::task::{AbortHandle, JoinError, JoinHandle};

/// A Tokio job whose owner cancels it on drop, including when its awaiting GUI task is released.
pub struct IoTask<T>(JoinHandle<T>);

impl<T> IoTask<T> {
    pub(crate) fn new(task: JoinHandle<T>) -> Self {
        Self(task)
    }

    pub fn abort(&self) {
        self.0.abort();
    }

    pub fn abort_handle(&self) -> AbortHandle {
        self.0.abort_handle()
    }
}

impl<T> Future for IoTask<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

impl<T> Drop for IoTask<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}
