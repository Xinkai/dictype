use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::Stream;
use tokio::time::{self, Interval};
use tokio_util::bytes::Bytes;
use tokio_util::sync::CancellationToken;

use base_client::audio_stream::{AudioCapture, AudioStream};

const PCM_TEST_WAV: &[u8] = include_bytes!("../../../assets/harvard.16k.mono.wav");
const WAV_HEADER_SIZE: usize = 44;
const BYTES_PER_SECOND_16K_MONO_PCM16: usize = 16_000 * 2;
const CHUNK_MILLIS: usize = 100;

pub struct PcmPlaybackRecorder {
    pcm: &'static [u8],
    chunk_size: usize,
}

struct PcmPlaybackStream {
    pcm: &'static [u8],
    offset: usize,
    chunk_size: usize,
    interval: Interval,
    cancellation_token: CancellationToken,
}

impl AudioCapture for PcmPlaybackRecorder {
    type CaptureOption = ();

    fn new(_capture_option: Self::CaptureOption) -> io::Result<Self> {
        if PCM_TEST_WAV.len() <= WAV_HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "wav payload is empty",
            ));
        }
        let chunk_size = ((BYTES_PER_SECOND_16K_MONO_PCM16 * CHUNK_MILLIS) / 1000).max(1);

        Ok(Self {
            pcm: &PCM_TEST_WAV[WAV_HEADER_SIZE..],
            chunk_size,
        })
    }

    fn create(&self, cancellation_token: CancellationToken) -> io::Result<AudioStream> {
        Ok(AudioStream(Box::pin(PcmPlaybackStream {
            pcm: self.pcm,
            offset: 0,
            chunk_size: self.chunk_size,
            interval: time::interval(Duration::from_millis(CHUNK_MILLIS as u64)),
            cancellation_token,
        })))
    }
}

impl Stream for PcmPlaybackStream {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.cancellation_token.is_cancelled() {
            return Poll::Ready(None);
        }

        if self.offset >= self.pcm.len() {
            return Poll::Ready(None);
        }

        if self.interval.poll_tick(cx).is_pending() {
            return Poll::Pending;
        }

        let end = self
            .offset
            .saturating_add(self.chunk_size)
            .min(self.pcm.len());
        let chunk = Bytes::copy_from_slice(&self.pcm[self.offset..end]);
        self.offset = end;

        Poll::Ready(Some(Ok(chunk)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_client::audio_stream::AudioCapture;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn emits_pcm_chunks() {
        let recorder = PcmPlaybackRecorder::new(()).unwrap();
        let mut audio_stream = recorder.create(CancellationToken::new()).unwrap();
        let first = audio_stream.next().await.unwrap().unwrap();
        assert!(!first.is_empty());
    }
}
