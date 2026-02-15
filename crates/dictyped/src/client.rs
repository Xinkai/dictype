use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use pin_project_lite::pin_project;

use base_client::asr_client_factory::AsrClientFactory;
use base_client::audio_stream::AudioStream;
use base_client::grpc_server::TranscribeResponse;
use paraformer_v2_client::client::ParaformerV2Client;
use paraformer_v2_client::client_factory::ParaformerV2ClientFactory;
use qwen_v3_client::client::QwenV3Client;
use qwen_v3_client::client_factory::QwenV3ClientFactory;

pin_project! {
    #[project = AnyClientProj]
    pub enum AnyClient {
        ParaformerV2 { #[pin] inner: ParaformerV2Client },
        QwenV3 { #[pin] inner: QwenV3Client },
    }
}

impl Stream for AnyClient {
    type Item = Result<TranscribeResponse, anyhow::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let projected = self.project();
        match projected {
            AnyClientProj::ParaformerV2 { inner } => inner
                .poll_next(cx)
                .map(|opt| opt.map(|res| res.map_err(anyhow::Error::from))),
            AnyClientProj::QwenV3 { inner } => inner
                .poll_next(cx)
                .map(|opt| opt.map(|res| res.map_err(anyhow::Error::from))),
        }
    }
}

#[derive(Debug)]
pub enum ClientFactory {
    ParaformerV2(ParaformerV2ClientFactory),
    QwenV3(QwenV3ClientFactory),
}

impl ClientFactory {
    pub async fn connect(
        &self,
        audio_stream: impl AudioStream + 'static,
    ) -> Result<AnyClient, anyhow::Error> {
        let client = match self {
            Self::ParaformerV2(paraformer_v2) => AnyClient::ParaformerV2 {
                inner: paraformer_v2.create(audio_stream).await?,
            },
            Self::QwenV3(qwen_v3) => AnyClient::QwenV3 {
                inner: qwen_v3.create(audio_stream).await?,
            },
        };

        Ok(client)
    }
}
