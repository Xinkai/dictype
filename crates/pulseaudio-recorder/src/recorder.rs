use std::ffi::CString;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::Stream;
use pulseaudio::{Client, protocol};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::bytes::Bytes;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, debug, info_span, trace, warn};

use base_client::audio_stream::{AudioCapture, AudioStream};

use crate::PulseAudioConfig;
use crate::error::PulseAudioRecorderError;

const SAMPLE_RATE: u32 = 16_000;
const CHANNELS: u8 = 1;

pub struct PulseAudioRecorder {
    client: Client,
    capture_option: PulseAudioConfig,
}

struct PulseAudioRecorderStream {
    inner: UnboundedReceiverStream<io::Result<Bytes>>,
}

impl AudioCapture for PulseAudioRecorder {
    type CaptureOption = PulseAudioConfig;

    fn new(capture_option: Self::CaptureOption) -> io::Result<Self> {
        let client = Client::from_env(c"dictype").map_err(|err| io::Error::other(err.to_string()))?;

        Ok(Self {
            client,
            capture_option,
        })
    }

    fn create(&self, cancellation_token: CancellationToken) -> io::Result<AudioStream> {
        let (tx, rx) = mpsc::unbounded_channel::<io::Result<Bytes>>();
        let client = self.client.clone();
        let capture_option = self.capture_option.clone();

        tokio::spawn(
            async move {
                if let Err(err) = capture_loop(tx, cancellation_token, client, capture_option).await
                {
                    debug!("capture loop ended with error: {err}");
                }
            }
            .instrument(info_span!("PulseAudioRecorder")),
        );

        Ok(AudioStream(Box::pin(PulseAudioRecorderStream {
            inner: UnboundedReceiverStream::new(rx),
        })))
    }
}

impl Stream for PulseAudioRecorderStream {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

async fn capture_loop(
    tx: mpsc::UnboundedSender<io::Result<Bytes>>,
    cancellation_token: CancellationToken,
    client: Client,
    capture_option: PulseAudioConfig,
) -> Result<(), PulseAudioRecorderError> {
    let result = run_capture_loop(&tx, cancellation_token, &client, capture_option).await;
    if let Err(error) = &result {
        let _ = tx.send(Err(io::Error::other(error.to_string())));
    }

    result
}

async fn get_source_info(
    client: &Client,
    preferred_device: Option<&str>,
) -> pulseaudio::Result<protocol::SourceInfo> {
    if let Some(device_name) = preferred_device {
        match CString::new(device_name) {
            Ok(device_name_c) => {
                if let Ok(source_info) = client.source_info_by_name(device_name_c).await {
                    return Ok(source_info);
                }
                warn!(
                    preferred_device = device_name,
                    "preferred source unavailable, falling back to default source"
                );
            }
            Err(err) => {
                warn!(
                    preferred_device = device_name,
                    error = %err,
                    "invalid preferred source name, falling back to default source"
                );
            }
        }
    }
    client
        .source_info_by_name(protocol::DEFAULT_SOURCE.to_owned())
        .await
}

async fn run_capture_loop(
    tx: &mpsc::UnboundedSender<io::Result<Bytes>>,
    cancellation_token: CancellationToken,
    client: &Client,
    capture_option: PulseAudioConfig,
) -> Result<(), PulseAudioRecorderError> {
    let source_info = get_source_info(client, capture_option.preferred_device.as_deref()).await?;
    trace!("selected source: {source_info:?}");

    let params = protocol::RecordStreamParams {
        source_index: Some(source_info.index),
        sample_spec: protocol::SampleSpec {
            format: protocol::SampleFormat::S16Le,
            channels: CHANNELS,
            sample_rate: SAMPLE_RATE,
        },
        ..Default::default()
    };

    let tx = tx.clone();

    if cancellation_token.is_cancelled() {
        return Ok(());
    }
    let stream = client
        .create_record_stream(params, move |data: &[u8]| {
            if data.is_empty() {
                return;
            }
            let _ = tx.send(Ok(Bytes::copy_from_slice(data)));
        })
        .await?;

    cancellation_token.cancelled().await;
    debug!("cancellation requested");

    stream.delete().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio_stream::StreamExt;
    use tokio_util::sync::CancellationToken;

    use base_client::audio_stream::AudioCapture;

    use crate::{PulseAudioConfig, PulseAudioRecorder};

    #[tokio::test]
    #[cfg_attr(not(has_pulseaudio), ignore = "PulseAudio is likely not available.")]
    async fn emits_pcm_chunks() {
        let Ok(recorder) = PulseAudioRecorder::new(PulseAudioConfig::default()) else {
            return;
        };
        let mut audio_stream = recorder.create(CancellationToken::new()).unwrap();
        match audio_stream.next().await {
            Some(Ok(_)) => {}
            _ => panic!("expected audio chunk"),
        }
    }
}
