use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use tokio_util::bytes::Bytes;
use tokio_util::sync::CancellationToken;

pub struct AudioStream(pub Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send + 'static>>);

impl Stream for AudioStream {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.as_mut().poll_next(cx)
    }
}

pub trait AudioCapture {
    type CaptureOption;

    fn create(
        cancellation_token: CancellationToken,
        capture_option: Self::CaptureOption,
    ) -> io::Result<AudioStream>;
}
