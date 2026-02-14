use std::io;

use futures_util::Stream;
use prost::bytes::Bytes;

pub trait AsrClient {
    type Options;
    type Client;

    fn connect(
        options: &Self::Options,
        audio_stream: impl Stream<Item = io::Result<Bytes>> + Send + 'static + Unpin,
    ) -> impl Future<Output = Result<Self::Client, anyhow::Error>>;
}
