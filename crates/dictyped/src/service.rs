use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use tracing::{Span, error, info, trace};

use base_client::audio_stream::AudioCapture;
use base_client::grpc_server::{
    Dictype, StopRequest, StopResponse, TranscribeRequest, TranscribeResponse,
};

use crate::client_store::ClientStore;
use crate::service_state::ServiceState;
use crate::session_stream::SessionStream;

pub struct DictypeService<R>
where
    R: AudioCapture,
{
    state: Arc<Mutex<ServiceState>>,
    client_store: ClientStore,
    recorder: Arc<R>,
}

#[tonic::async_trait]
impl<R> Dictype for DictypeService<R>
where
    R: AudioCapture + Send + Sync + 'static,
{
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

        let asr_client = self
            .client_store
            .get_asr_client_for_profile(&req.profile_name)
            .ok_or_else(|| {
                Status::invalid_argument(format!("profile not found: {:?}", &req.profile_name))
            })?;
        info!("found asr client: {:?}", &asr_client);

        // Expose cancellation so Stop can signal this session.
        let recording_cancellation = CancellationToken::new();
        {
            let mut state = state
                .lock()
                .map_err(|_| Status::internal("state poisoned"))?;
            state.replace(recording_cancellation.clone())?;
        }

        // Channel for streaming gRPC responses.
        let (tx, rx) = mpsc::channel::<Result<TranscribeResponse, Status>>(32);
        let recorder = Arc::clone(&self.recorder);

        let recording_cancellation2 = recording_cancellation.clone();
        tokio::spawn(async move {
            trace!("starting recording");
            let audio_stream = match recorder.create(recording_cancellation.clone()) {
                Ok(audio_stream) => audio_stream,
                Err(e) => {
                    let _ = tx
                        .send(Err(Status::internal(format!("failed to record: {e:?}"))))
                        .await;
                    return;
                }
            };
            trace!("started recording");

            let mut client = match asr_client
                .create_transcription_stream(audio_stream)
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
                match client.next().await {
                    Some(Ok(evt)) => {
                        if tx.send(Ok(evt)).await.is_err() {
                            error!("Cannot send response to gRPC client, session stopped.");
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        let _ = tx
                            .send(Err(Status::internal(format!("receive error: {e}"))))
                            .await;
                        break;
                    }
                    None => {
                        info!("TranscribeStream closed.");
                        break;
                    }
                }
            }

            let _ = state.lock().expect("state poisoned").reset();
        });

        let response_stream = ReceiverStream::new(rx);
        let stream = SessionStream::new(response_stream, recording_cancellation2);
        Ok(Response::new(stream))
    }

    async fn stop(&self, _request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let stopped = self.state.lock().expect("state poisoned").reset();
        let response = StopResponse { stopped };
        Ok(Response::new(response))
    }
}

impl<R> DictypeService<R>
where
    R: AudioCapture + Send + Sync + 'static,
{
    pub fn new(client_store: ClientStore, recorder: R) -> Self {
        Self {
            state: Arc::new(Mutex::new(ServiceState::new())),
            client_store,
            recorder: Arc::new(recorder),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use tonic::Code;

    use base_client::audio_stream::AudioStream;
    use config_tool::config_store::ConfigFile;

    struct NeverRecorder;

    impl AudioCapture for NeverRecorder {
        type CaptureOption = ();

        fn new(_capture_option: Self::CaptureOption) -> io::Result<Self> {
            Ok(Self)
        }

        fn create(&self, _cancellation_token: CancellationToken) -> io::Result<AudioStream> {
            Ok(AudioStream(Box::pin(futures_util::stream::empty())))
        }
    }

    fn empty_service() -> DictypeService<NeverRecorder> {
        DictypeService::new(
            ClientStore::load(&ConfigFile::default()),
            NeverRecorder::new(()).expect("NeverRecorder must initialize"),
        )
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
