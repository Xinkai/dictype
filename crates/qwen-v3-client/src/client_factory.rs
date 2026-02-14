use std::io;

use anyhow::Error;
use futures_util::Stream;
use tokio_util::bytes::Bytes;

use base_client::asr_client::AsrClient;
use base_client::asr_client_factory::AsrClientFactory;

use crate::client::QwenV3Client;
use crate::config::QwenV3Config;

/// Read more: <https://help.aliyun.com/zh/model-studio/qwen-real-time-speech-recognition>
#[derive(Debug)]
pub struct QwenV3ClientFactory {
    options: QwenV3Config,
}

impl AsrClientFactory<QwenV3Client> for QwenV3ClientFactory {
    type Options = QwenV3Config;

    fn new(options: impl Into<Self::Options>) -> Self {
        Self {
            options: options.into(),
        }
    }

    fn create(
        &self,
        audio_stream: impl Stream<Item = io::Result<Bytes>> + Send + 'static + Unpin,
    ) -> impl Future<Output = Result<QwenV3Client, Error>> {
        QwenV3Client::connect(&self.options, audio_stream)
    }
}
