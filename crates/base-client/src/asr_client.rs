use crate::audio_stream::AudioStream;

pub trait AsrClient {
    type Options;
    type Client;

    fn connect(
        options: &Self::Options,
        audio_stream: impl AudioStream + 'static,
    ) -> impl Future<Output = Result<Self::Client, anyhow::Error>>;
}
