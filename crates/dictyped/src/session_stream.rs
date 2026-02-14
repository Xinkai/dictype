use std::pin::Pin;
use std::task::{Context, Poll};

use tokio_stream::Stream;
use tokio_util::sync::CancellationToken;
use tonic::Status;
use tracing::warn;

use base_client::grpc_server::TranscribeResponse;

pub struct SessionStream {
    inner: Pin<Box<dyn Stream<Item = Result<TranscribeResponse, Status>> + Send>>,
    cancellation: CancellationToken,
}

impl SessionStream {
    pub(crate) fn new<S>(stream: S, cancellation: CancellationToken) -> Self
    where
        S: Stream<Item = Result<TranscribeResponse, Status>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
            cancellation,
        }
    }
}

impl Drop for SessionStream {
    fn drop(&mut self) {
        // If the gRPC client disconnects, dropping the response stream should
        // propagate cancellation to stop audio capture and upstream processing.
        self.cancellation.cancel();
    }
}

impl Stream for SessionStream {
    type Item = Result<TranscribeResponse, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let item = self.inner.as_mut().poll_next(cx);
        if let Poll::Ready(Some(Err(status))) = &item {
            warn!(
                code = ?status.code(),
                message = %status.message(),
                "transcribe stream yielded error status"
            );
        }
        item
    }
}
