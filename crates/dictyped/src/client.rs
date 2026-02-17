use base_client::asr_client::AsrClient;
use base_client::audio_stream::AudioStream;
use base_client::transcribe_stream::TranscribeStream;
use paraformer_v2_client::client::ParaformerV2Client;
use qwen_v3_client::client::QwenV3Client;

#[derive(Debug)]
pub enum AsrClientInstance {
    ParaformerV2(ParaformerV2Client),
    QwenV3(QwenV3Client),
}

impl AsrClientInstance {
    pub async fn create_transcription_stream(
        &self,
        audio_stream: AudioStream,
    ) -> Result<TranscribeStream<anyhow::Error>, anyhow::Error> {
        match self {
            Self::ParaformerV2(paraformer_v2) => paraformer_v2.create(audio_stream).await,
            Self::QwenV3(qwen_v3) => qwen_v3.create(audio_stream).await,
        }
    }
}
