use crate::audio_stream::AudioStream;

pub trait AsrClient {
    type Config;
    type TranscriptionStream;

    fn new(config: impl Into<Self::Config>) -> Self;

    fn create(
        &self,
        audio_stream: AudioStream,
    ) -> impl Future<Output = Result<Self::TranscriptionStream, anyhow::Error>>;
}
