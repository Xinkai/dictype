use base_client::asr_client::AsrClient;
use base_client::audio_stream::AudioStream;
use base_client::transcribe_stream::TranscribeStream;
use paraformer_v2_client::client::ParaformerV2Client;
use qwen_v3_client::client::QwenV3Client;

#[async_trait::async_trait]
pub trait BackendClient {
    async fn create_transcription_stream(
        &self,
        audio_stream: AudioStream,
    ) -> Result<TranscribeStream<anyhow::Error>, anyhow::Error>;
}

#[async_trait::async_trait]
impl BackendClient for ParaformerV2Client {
    async fn create_transcription_stream(
        &self,
        audio_stream: AudioStream,
    ) -> Result<TranscribeStream<anyhow::Error>, anyhow::Error> {
        self.create(audio_stream).await
    }
}

#[async_trait::async_trait]
impl BackendClient for QwenV3Client {
    async fn create_transcription_stream(
        &self,
        audio_stream: AudioStream,
    ) -> Result<TranscribeStream<anyhow::Error>, anyhow::Error> {
        self.create(audio_stream).await
    }
}
