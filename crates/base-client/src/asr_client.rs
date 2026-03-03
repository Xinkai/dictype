use crate::audio_stream::AudioStream;

#[async_trait::async_trait]
pub trait AsrClient {
    type Config;
    type TranscriptionStream;

    fn new(config: impl Into<Self::Config>) -> Self;

    async fn create(
        &self,
        audio_stream: AudioStream,
    ) -> Result<Self::TranscriptionStream, anyhow::Error>;
}
