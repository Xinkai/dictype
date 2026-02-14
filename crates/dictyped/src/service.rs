use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use tracing::{Span, error, info, trace};

use base_client::grpc_server::{
    Dictype, StopRequest, StopResponse, TranscribeRequest, TranscribeResponse,
};
use config_tool::config_store::ConfigFile;

use crate::audio_stream::AudioStream;
use crate::client_store::ClientStore;
use crate::service_state::ServiceState;
use crate::session_stream::SessionStream;

pub struct DictypeService {
    state: Arc<Mutex<ServiceState>>,
    client_store: ClientStore,
}

impl DictypeService {}

#[tonic::async_trait]
impl Dictype for DictypeService {
    type TranscribeStream = SessionStream;

    async fn transcribe(
        &self,
        request: Request<TranscribeRequest>,
    ) -> Result<Response<Self::TranscribeStream>, Status> {
        let req = request.get_ref();
        Span::current().record("profile_name", tracing::field::display(&req.profile_name));
        info!("starting session by profile name: {}", &req.profile_name);

        // Check if an existing request is ongoing
        let state = self.state.clone();
        {
            if state
                .lock()
                .map_err(|_| Status::internal("state poisoned"))?
                .is_some()
            {
                Err(Status::already_exists("request exists"))?;
            }
        }

        let client_factory = self
            .client_store
            .get_client_for_profile(&req.profile_name)
            .ok_or_else(|| {
                Status::invalid_argument(format!("profile not found: {:?}", &req.profile_name))
            })?;
        info!("found client_factory: {:?}", &client_factory);

        // Expose cancellation so Stop can signal this session.
        let cancellation = CancellationToken::new();
        {
            let mut state = state
                .lock()
                .map_err(|_| Status::internal("state poisoned"))?;
            state.replace(cancellation.clone())?;
        }

        // Channel for streaming gRPC responses.
        let (tx, rx) = mpsc::channel::<Result<TranscribeResponse, Status>>(32);

        let spawn_cancellation_token = cancellation.clone();
        tokio::spawn(async move {
            trace!("starting recording");
            let audio_stream = match AudioStream::new(spawn_cancellation_token.clone()) {
                Ok(audio_stream) => audio_stream,
                Err(e) => {
                    let _ = tx
                        .send(Err(Status::internal(format!("failed to record: {e:?}"))))
                        .await;
                    return;
                }
            };
            trace!("started recording");

            let mut client = match client_factory
                .connect(audio_stream)
                .await
                .map_err(|e| Status::internal(format!("backend client connect failed: {e}")))
            {
                Ok(client) => client,
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            };
            loop {
                tokio::select! {
                    () = spawn_cancellation_token.cancelled() => {
                        info!("session cancellation requested");
                        break;
                    }
                    transcribe_response = client.next() => {
                        match transcribe_response {
                            Some(Ok(evt)) => {
                                if tx.send(Ok(evt)).await.is_err() {
                                    error!("Cannot send response to gRPC client, session stopped.");
                                    break;
                                }
                            }
                            Some(Err(e)) => {
                                let _ = tx.send(Err(Status::internal(format!("receive error: {e}")))).await;
                                break;
                            }
                            None => break,
                        }
                    }
                }
            }

            let _ = state.lock().expect("state poisoned").reset();
        });

        let response_stream = ReceiverStream::new(rx);
        let stream = SessionStream::new(response_stream, cancellation);
        Ok(Response::new(stream))
    }

    async fn stop(&self, _request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let stopped = self.state.lock().expect("state poisoned").reset();
        let response = StopResponse { stopped };
        Ok(Response::new(response))
    }
}

impl DictypeService {
    pub fn new() -> Self {
        let config = ConfigFile::load().unwrap_or_else(|err| {
            error!("Failed to load config: {err}");
            ConfigFile::default()
        });
        let client_store = ClientStore::load(&config);
        Self {
            state: Arc::new(Mutex::new(ServiceState::new())),
            client_store,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Code;

    fn empty_service() -> DictypeService {
        DictypeService {
            state: Arc::new(Mutex::new(ServiceState::new())),
            client_store: ClientStore::load(&ConfigFile::default()),
        }
    }

    #[tokio::test]
    async fn transcribe_returns_invalid_argument_for_unknown_profile() {
        let service = empty_service();
        let request = Request::new(TranscribeRequest {
            profile_name: "missing-profile".to_string(),
        });

        let Err(err) = service.transcribe(request).await else {
            panic!("must fail")
        };

        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(err.message().contains("profile not found"));
        assert!(!service.state.lock().expect("state poisoned").is_some());
    }

    #[tokio::test]
    async fn transcribe_repeated_unknown_profile_calls_are_handled() {
        let service = empty_service();

        let first = service
            .transcribe(Request::new(TranscribeRequest {
                profile_name: "missing-profile".to_string(),
            }))
            .await;
        let Err(first) = first else {
            panic!("first call must fail")
        };
        assert_eq!(first.code(), Code::InvalidArgument);

        let second = service
            .transcribe(Request::new(TranscribeRequest {
                profile_name: "missing-profile".to_string(),
            }))
            .await;
        let Err(second) = second else {
            panic!("second call must fail")
        };
        assert_eq!(second.code(), Code::InvalidArgument);
        assert!(!service.state.lock().expect("state poisoned").is_some());
    }
}
