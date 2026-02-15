use std::io;

use futures_util::Stream;
use tokio_util::bytes::Bytes;
use tokio_util::sync::CancellationToken;

pub trait AudioStream: Stream<Item = io::Result<Bytes>> + Send + Unpin {
    type CaptureOption;

    fn new(
        cancellation_token: CancellationToken,
        capture_option: Self::CaptureOption,
    ) -> io::Result<Self>
    where
        Self: Sized;
}
