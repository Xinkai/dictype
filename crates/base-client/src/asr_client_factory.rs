use futures_util::Stream;
use prost::bytes::Bytes;
use std::io;

use crate::asr_client::AsrClient;

pub trait AsrClientFactory<Client>
where
    Client: AsrClient,
{
    type Options;

    fn new(options: impl Into<Self::Options>) -> Self;

    fn create(
        &self,
        audio_stream: impl Stream<Item = io::Result<Bytes>> + Send + 'static + Unpin,
    ) -> impl Future<Output = Result<Client, anyhow::Error>>;
}
