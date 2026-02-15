use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::stream;
use futures_util::{SinkExt, Stream, StreamExt};
use tokio::select;
use tokio_tungstenite::connect_async;
use tokio_util::bytes::Bytes;
use tracing::{error, info, trace};
use tungstenite::Message;
use tungstenite::client::IntoClientRequest;
use tungstenite::http::HeaderValue;
use tungstenite::http::header::AUTHORIZATION;

use base_client::asr_client::AsrClient;
use base_client::audio_stream::AudioStream;
use base_client::grpc_server::TranscribeResponse;
use base_client::transcribe_stream::TranscribeStream;

use crate::client_state::ClientState;
use crate::config::QwenV3Config;
use crate::error::QwenV3Error;
use crate::types;

#[allow(dead_code)]
pub struct QwenV3Client {
    inner: TranscribeStream<QwenV3Error>,
}

impl Stream for QwenV3Client {
    type Item = Result<TranscribeResponse, QwenV3Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

#[derive(Debug, PartialEq)]
enum Stage {
    SessionCreating,
    SessionCreated,
    AwaitTaskFinished,
}

fn transcribe<W>(
    web_socket_stream: W,
    mut audio_stream: impl Stream<Item = io::Result<Bytes>> + Unpin,
    config: QwenV3Config,
) -> impl Stream<Item = Result<TranscribeResponse, QwenV3Error>>
where
    W: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + StreamExt
        + futures_util::Sink<Message>,
    <W as futures_util::Sink<Message>>::Error: std::fmt::Debug,
    QwenV3Error: From<<W as futures_util::Sink<Message>>::Error>,
{
    let (mut send, mut recv) = web_socket_stream.split();

    let mut stage = Stage::SessionCreating;
    let mut event_count = 0;

    let mut client_state: Option<ClientState> = None;

    stream! {
        loop {
            select! {
                server_msg = recv.next() => {
                    match server_msg {
                        Some(Ok(Message::Text(text))) => {
                            let server_event = serde_json::from_str::<types::ServerEvent>(&text);

                            let server_event = match server_event {
                                Ok(e) => e,
                                Err(_e) => {
                                    error!("failed to parse ServerEvent: {text}");
                                    break;
                                },
                            };

                            match server_event {
                                types::ServerEvent::Error(err) => {
                                    error!("err: {err:?}");
                                    let _ = send.close().await;
                                }
                                types::ServerEvent::SessionCreated(response) => {
                                    trace!("session created: {:?}", &response);
                                    let session_update = types::session::update::request::Request::new(event_count, &config);
                                    event_count+=1;
                                    let body = Message::Text(serde_json::to_string(&session_update).expect("Json Stringify").into());
                                    if let Err(_e) = send.send(body).await {
                                        yield Err(QwenV3Error::Connection);
                                        break;
                                    }
                                    stage = Stage::SessionCreated;
                                },
                                types::ServerEvent::SessionUpdated(event) => {
                                    info!("SessionUpdatedsave: {:?}", event);
                                },
                                types::ServerEvent::SessionFinished(response) => {
                                    info!("SessionFinished: {response:?}");
                                    let _ = send.close().await;
                                },
                                types::ServerEvent::ConversationItemCreated(response) => {
                                    trace!("ConversationItemCreated: {:?}", &response.item);
                                },
                                types::ServerEvent::ConversationItemInputAudioTranscriptionTranscriptionText(response) => {
                                    trace!("ConversationItemInputAudioTranscriptionTranscriptionText: {:?}", &response);
                                    if response.text.is_empty() {
                                        continue;
                                    }
                                    let existing = client_state.as_mut().expect("ClientState must exist");
                                    existing.text = response.text;
                                    yield Ok(TranscribeResponse {
                                        begin_time: existing.start_time,
                                        sentence_end: false,
                                        text: existing.text.clone(),
                                    });
                                },
                                types::ServerEvent::ConversationItemInputAudioTranscriptionCompleted(response) => {
                                    trace!("ConversationItemInputAudioTranscriptionCompleted: {:?}", &response);
                                    let existing = client_state.take();
                                    match existing {
                                        Some(client_state) => {
                                            yield Ok(TranscribeResponse {
                                                begin_time: client_state.start_time,
                                                text: response.transcript,
                                                sentence_end: true,
                                            })
                                        },
                                        None => {
                                            panic!("existing ClientState");
                                        }
                                    }
                                },
                                types::ServerEvent::InputAudioBufferSpeechStarted(response) => {
                                    trace!("InputAudioBufferSpeechStarted: {:?}", &response);
                                    let existing = client_state.replace(ClientState { start_time: response.audio_start_ms, text: String::new() });
                                    assert!(existing.is_none(), "existing ClientState");
                                },
                                types::ServerEvent::InputAudioBufferSpeechStopped(response) => {
                                    trace!("InputAudioBufferSpeechStopped: {:?}", &response);
                                },
                                types::ServerEvent::InputAudioBufferCommitted(response) => {
                                    trace!("InputAudioBufferCommitted: {:?}", &response);
                                },
                            }
                        }
                        Some(Ok(Message::Close(frame))) => {
                            match frame {
                                Some(close_frame) => {
                                    info!("close by server: {:?}", &close_frame);
                                    yield Err(QwenV3Error::Closed(close_frame.reason.as_str().to_string()));
                                },
                                None => {
                                    info!("close by server: {:?}", frame);
                                }
                            }
                        },
                        None => {
                            info!("server disconnected.");
                            return;
                        }
                        Some(Err(error)) => {
                            error!("connection error: {:?}", error);
                            yield Err(QwenV3Error::Connection);
                            break;
                        },
                        Some(Ok(Message::Ping(data))) => {
                            let _ = send.send(Message::Pong(data)).await;
                        },
                        Some(Ok(Message::Pong(_))) => {
                            // ignore
                        }
                        Some(Ok(Message::Binary(_))) => {
                            unreachable!("Unexpected binary");
                        }
                        Some(Ok(Message::Frame(_))) => {
                            unreachable!("Unexpected frame");
                        }
                    }
                },
                chunk = audio_stream.next(), if matches!(stage, Stage::SessionCreated) => {
                    if let Some(chunk) = chunk {
                        match chunk {
                            Ok(chunk) => {
                                let req = types::input_audio_buffer::append::request::Request::new(format!("event_{event_count}"), chunk);
                                event_count += 1;
                                let body = Message::Text(serde_json::to_string(&req).expect("Json Stringify").into());
                                if let Err(_e) = send.send(body).await {
                                    yield Err(QwenV3Error::Connection);
                                    break;
                                }
                            },
                            Err(err) => {
                                error!("error: {:?}", err);
                                yield Err(QwenV3Error::Audio(err));
                            }
                        }
                    } else {
                        let finish_req = types::session::finish::request::Request::new(event_count);
                        event_count += 1;
                        let txt = match serde_json::to_string(&finish_req) {
                            Ok(t) => t,
                            Err(e) => {
                                yield Err(e.into());
                                break;
                            }
                        };
                        if let Err(_e) = send.send(Message::Text(txt.into())).await {
                            yield Err(QwenV3Error::Connection);
                            break;
                        }
                        stage = Stage::AwaitTaskFinished;
                    }
                }
            }
        }
    }
}

impl AsrClient for QwenV3Client {
    type Options = QwenV3Config;
    type Client = Self;

    async fn connect(
        config: &Self::Options,
        audio_stream: impl AudioStream + 'static,
    ) -> anyhow::Result<Self> {
        let mut request =
            "wss://dashscope.aliyuncs.com/api-ws/v1/realtime?model=qwen3-asr-flash-realtime"
                .into_client_request()?;
        let headers = request.headers_mut();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.dashscope_api_key))
                .map_err(|_| anyhow::anyhow!("invalid header value for `Authorization`"))?,
        );

        let (ws_stream, _resp) = connect_async(request).await?;

        let transcribe_stream = transcribe(ws_stream, audio_stream, config.clone());
        Ok(Self {
            inner: TranscribeStream::new(Box::pin(transcribe_stream)),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::types::Language;
    use pcm_playback_recorder::PcmPlaybackRecorder;
    use tokio::time::sleep;
    use tokio_stream::StreamExt;
    use tokio_util::sync::CancellationToken;

    #[cfg_attr(not(has_dashscope), ignore = "requires DASHSCOPE_API_KEY env var")]
    #[tokio::test]
    async fn connect() {
        let cancellation = CancellationToken::new();
        let audio_stream = PcmPlaybackRecorder::new(cancellation.clone(), ()).unwrap();
        tokio::spawn(async move {
            sleep(Duration::from_secs(5)).await;
            cancellation.cancel();
        });

        let mut client = QwenV3Client::connect(
            &QwenV3Config {
                dashscope_api_key: std::env::var("DASHSCOPE_API_KEY").unwrap(),
                language: Some(Language::English),
                turn_detection: None,
            },
            audio_stream,
        )
        .await
        .unwrap();

        while let (Some(event)) = client.next().await {
            dbg!(event);
        }
    }
}
