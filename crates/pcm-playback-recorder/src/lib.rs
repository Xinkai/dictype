use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::Stream;
use tokio::time::{self, Interval};
use tokio_util::bytes::Bytes;
use tokio_util::sync::CancellationToken;

use base_client::audio_stream::{AudioCapture, AudioStream};

const WAV_HEADER_SIZE: usize = 44;
const BYTES_PER_SECOND_16K_MONO_PCM16: usize = 16_000 * 2;
const CHUNK_MILLIS: usize = 100;

pub struct PcmPlaybackRecorder {
    pcm: Arc<[u8]>,
    chunk_size: usize,
}

struct PcmPlaybackStream {
    pcm: Arc<[u8]>,
    offset: usize,
    chunk_size: usize,
    interval: Interval,
    cancellation_token: CancellationToken,
}

pub struct PcmPlaybackCaptureOption {
    pub file: PathBuf,
}

impl PcmPlaybackCaptureOption {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self { file: file.into() }
    }
}

impl AudioCapture for PcmPlaybackRecorder {
    type CaptureOption = PcmPlaybackCaptureOption;

    fn new(capture_option: Self::CaptureOption) -> io::Result<Self> {
        let wav = std::fs::read(&capture_option.file)?;
        if wav.len() <= WAV_HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "wav payload is empty",
            ));
        }
        let chunk_size = ((BYTES_PER_SECOND_16K_MONO_PCM16 * CHUNK_MILLIS) / 1000).max(1);
        let pcm = Arc::<[u8]>::from(wav[WAV_HEADER_SIZE..].to_vec());

        Ok(Self { pcm, chunk_size })
    }

    fn create(&self, cancellation_token: CancellationToken) -> io::Result<AudioStream> {
        Ok(AudioStream(Box::pin(PcmPlaybackStream {
            pcm: self.pcm.clone(),
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

    fn test_wav_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/harvard.16k.mono.wav")
    }

    #[tokio::test]
    async fn emits_pcm_chunks() {
        let recorder =
            PcmPlaybackRecorder::new(PcmPlaybackCaptureOption::new(test_wav_path())).unwrap();
        let mut audio_stream = recorder.create(CancellationToken::new()).unwrap();
        let first = audio_stream.next().await.unwrap().unwrap();
        assert!(!first.is_empty());
    }
}
