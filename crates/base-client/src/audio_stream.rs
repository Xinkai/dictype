use std::io;

use futures_util::Stream;
use tokio_util::bytes::Bytes;
use tokio_util::sync::CancellationToken;

pub trait AudioStream: Stream<Item = io::Result<Bytes>> + Send + Unpin {
    fn new(cancellation_token: CancellationToken) -> io::Result<Self>
    where
        Self: Sized;
}
