use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;

use crate::grpc_server::TranscribeResponse;

pub struct TranscribeStream<E> {
    inner: Pin<Box<dyn Stream<Item = Result<TranscribeResponse, E>> + Send>>,
}

impl<E> TranscribeStream<E> {
    #[must_use]
    pub fn new(inner: Pin<Box<dyn Stream<Item = Result<TranscribeResponse, E>> + Send>>) -> Self {
        Self { inner }
    }
}

impl<E> Stream for TranscribeStream<E> {
    type Item = Result<TranscribeResponse, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}
