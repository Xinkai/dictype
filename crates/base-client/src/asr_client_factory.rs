use crate::asr_client::AsrClient;
use crate::audio_stream::AudioStream;

pub trait AsrClientFactory<Client>
where
    Client: AsrClient,
{
    type Options;

    fn new(options: impl Into<Self::Options>) -> Self;

    fn create(
        &self,
        audio_stream: impl AudioStream + 'static,
    ) -> impl Future<Output = Result<Client, anyhow::Error>>;
}
