use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::process::ChildStdout;
use tokio::select;
use tokio_stream::Stream;
use tokio_util::bytes::Bytes;
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub struct AudioStream {
    inner: ReaderStream<ChildStdout>,
    cancellation_token: CancellationToken,
}

impl AudioStream {
    pub fn new(cancellation_token: CancellationToken) -> io::Result<Self> {
        let mut child = tokio::process::Command::new("/bin/pw-record")
            .arg("--rate")
            .arg("16000")
            .args(["--properties", r#"{ "media.class": "Stream/Input/Audio" }"#])
            .args([
                "--media-role",
                "Communication",
                "--media-category",
                "Capture",
                "--channels",
                "1",
                "--raw",
                "-",
            ])
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("no stdout");

        let cancellation_token_clone = cancellation_token.clone();

        // Ensure the child process is spawned in the runtime so it can
        // make progress on its own while we await for any output.
        tokio::spawn(async move {
            select! {
                () = cancellation_token_clone.cancelled() => {
                    debug!("AudioStream: cancellation requested");
                    child.kill().await.expect("failed to kill child process");
                    debug!("AudioStream: recorder process killed");
                }
                status = child.wait() => {
                    panic!("child process exited with status: {}", status.unwrap());
                }
            }
        });

        Ok(Self {
            inner: ReaderStream::new(stdout),
            cancellation_token,
        })
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}

impl Stream for AudioStream {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;
    use tokio_stream::StreamExt;

    #[tokio::test]
    #[ignore = "audio server is likely not installed"]
    async fn it_works() {
        let cancellation_token = CancellationToken::new();
        {
            let mut stream = AudioStream::new(cancellation_token).unwrap();
            let mut a = 0;
            while let Some(_value) = stream.next().await {
                a += 1;
                if a > 20 {
                    break;
                }
            }
        }
        sleep(Duration::from_millis(5_000)).await;
    }
}
