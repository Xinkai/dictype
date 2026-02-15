use anyhow::Error;

use base_client::asr_client::AsrClient;
use base_client::asr_client_factory::AsrClientFactory;
use base_client::audio_stream::AudioStream;

use crate::client::ParaformerV2Client;
use crate::config::ParaformerV2Config;

/// Read more: <https://help.aliyun.com/zh/model-studio/websocket-for-paraformer-real-time-service>
#[derive(Debug)]
pub struct ParaformerV2ClientFactory {
    options: ParaformerV2Config,
}

impl AsrClientFactory<ParaformerV2Client> for ParaformerV2ClientFactory {
    type Options = ParaformerV2Config;

    fn new(options: impl Into<Self::Options>) -> Self {
        Self {
            options: options.into(),
        }
    }

    fn create(
        &self,
        audio_stream: impl AudioStream + 'static,
    ) -> impl Future<Output = Result<ParaformerV2Client, Error>> {
        ParaformerV2Client::connect(&self.options, audio_stream)
    }
}
