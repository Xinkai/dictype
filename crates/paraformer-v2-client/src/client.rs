use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::stream;
use futures_util::Stream;
use futures_util::{SinkExt, StreamExt};
use tokio::select;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_util::bytes::Bytes;
use tracing::{error, info, trace};

use base_client::asr_client::AsrClient;
use base_client::grpc_server::TranscribeResponse;
use base_client::transcribe_stream::TranscribeStream;

use crate::config::ParaformerV2Config;
use crate::error::ParaformerV2Error;
use crate::types;

pub struct ParaformerV2Client {
    inner: TranscribeStream<ParaformerV2Error>,
}

impl Stream for ParaformerV2Client {
    type Item = Result<TranscribeResponse, ParaformerV2Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

#[allow(clippy::enum_variant_names)]
enum Stage {
    AwaitTaskStarted,
    AwaitResultGenerated,
    AwaitTaskFinished,
}

fn transcribe<W>(
    web_socket_stream: W,
    mut audio_stream: impl Stream<Item = io::Result<Bytes>> + Unpin,
    config: ParaformerV2Config,
) -> impl Stream<Item = Result<TranscribeResponse, ParaformerV2Error>>
where
    W: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + StreamExt
        + futures_util::Sink<Message>,
    <W as futures_util::Sink<Message>>::Error: std::fmt::Debug,
    ParaformerV2Error: From<<W as futures_util::Sink<Message>>::Error>,
{
    let (mut send, mut recv) = web_socket_stream.split();

    stream! {
        let mut stage: Stage = Stage::AwaitTaskStarted;
        let task_id = {
            let run_task_req = types::run_task::request::Request::new(config);
            let text = match serde_json::to_string(&run_task_req) { Ok(t) => t, Err(e) => { yield Err(e.into()); return; } };
            if let Err(_e) = send.send(Message::Text(text.into())).await { yield Err(ParaformerV2Error::Connection); return; }
            run_task_req.header.task_id.clone()
        };

        loop {
            select! {
                chunk = audio_stream.next(), if matches!(stage, Stage::AwaitResultGenerated) => {
                    if let Some(chunk) = chunk {
                        match chunk {
                            Ok(chunk) => {
                                if let Err(_e) = send.send(Message::Binary(chunk)).await {
                                    yield Err(ParaformerV2Error::Connection);
                                    break;
                                }
                            },
                            Err(err) => {
                                yield Err(ParaformerV2Error::Audio(err));
                            }
                        }
                    } else {
                        let finish_req = types::finish_task::request::Request::new(&task_id);
                        let txt = match serde_json::to_string(&finish_req) {
                            Ok(t) => t,
                            Err(e) => {
                                yield Err(e.into());
                                break;
                            }
                        };
                        if let Err(_e) = send.send(Message::Text(txt.into())).await {
                            yield Err(ParaformerV2Error::Connection);
                            break;
                        }
                        stage = Stage::AwaitTaskFinished;
                    }
                }
                server_msg = recv.next() => {
                    match server_msg {
                        Some(Ok(Message::Text(text))) => {
                            let server_event = serde_json::from_str::<types::ServerEvent>(&text);

                            let server_event = match server_event {
                                Ok(e) => e,
                                Err(_e) => {
                                    error!("failed to parse server event: {text}");
                                    break;
                                },
                            };

                            match server_event {
                                types::ServerEvent::TaskFailed(response) => {
                                    error!("TaskFailed {response:?}");
                                    yield Err(ParaformerV2Error::Connection);
                                    let _ = send.close().await;
                                    break;
                                } ,
                                types::ServerEvent::TaskStarted(response) => {
                                    trace!("TaskStarted {response:?}");
                                    stage = Stage::AwaitResultGenerated;
                                },
                                types::ServerEvent::ResultGenerated(response) => {
                                    yield Ok(response.into());
                                },
                                types::ServerEvent::TaskFinished(response) => {
                                    info!("TaskFinished {response:?}");
                                    let _ = send.close().await;
                                },
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = send.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // ignore
                        }
                        Some(Ok(Message::Close(frame))) => {
                            match frame {
                                Some(close_frame) => {
                                    info!("close by server: {:?}", &close_frame);
                                    yield Err(ParaformerV2Error::Closed(close_frame.reason.as_str().to_string()));
                                },
                                None => {
                                    info!("close by server: {:?}", frame);
                                }
                            }
                        }
                        None => {
                            info!("server disconnected.");
                            return;
                        }
                        Some(Ok(Message::Binary(_))) => {
                            unreachable!("Unexpected binary");
                        }
                        Some(Ok(Message::Frame(_))) => {
                            unreachable!("Unexpected frame");
                        }
                        Some(Err(error)) => {
                            error!("connection error: {:?}", error);
                            yield Err(ParaformerV2Error::Connection);
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl AsrClient for ParaformerV2Client {
    type Options = ParaformerV2Config;
    type Client = Self;

    async fn connect(
        config: &Self::Options,
        audio_stream: impl Stream<Item = io::Result<Bytes>> + Send + 'static + Unpin,
    ) -> anyhow::Result<Self> {
        let mut request =
            "wss://dashscope.aliyuncs.com/api-ws/v1/inference".into_client_request()?;
        let headers = request.headers_mut();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.dashscope_api_key))
                .map_err(|_| ParaformerV2Error::InvalidHeaderValue("Authorization"))?,
        );

        let (ws_stream, _resp) = connect_async(request)
            .await
            .map_err(ParaformerV2Error::from)?;

        let transcribe_stream = transcribe(ws_stream, audio_stream, config.clone());

        Ok(Self {
            inner: TranscribeStream::new(Box::pin(transcribe_stream)),
        })
    }
}
