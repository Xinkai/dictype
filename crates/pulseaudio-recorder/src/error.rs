use std::io;

use tokio::sync::mpsc;
use tokio_util::bytes::Bytes;

#[derive(Debug, thiserror::Error)]
pub enum PulseAudioRecorderError {
    #[error("pulseaudio error: {0}")]
    PulseAudio(#[from] pulseaudio::ClientError),

    #[error("audio io error: {0}")]
    Audio(#[from] io::Error),

    #[error("audio stream receiver dropped")]
    StreamReceiverDropped(#[from] mpsc::error::SendError<io::Result<Bytes>>),
}
