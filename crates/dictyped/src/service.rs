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

#[async_trait::async_trait]
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
        info!("found asr client for profile: {}", &req.profile_name);

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
                    let _ = state.lock().expect("state poisoned").reset();
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
                    let _ = state.lock().expect("state poisoned").reset();
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
    use super::*;
    use tonic::Code;

    use crate::client::BackendClient;
    use crate::service::tests::mock_services::*;

    mod mock_recorders {
        use std::io;

        use async_stream::stream;
        use tokio_util::bytes::Bytes;
        use tokio_util::sync::CancellationToken;

        use base_client::audio_stream::{AudioCapture, AudioStream};

        pub(super) struct NoiseRecorder {
            remaining: usize,
        }

        impl AudioCapture for NoiseRecorder {
            type CaptureOption = usize;

            fn new(emit_count: Self::CaptureOption) -> io::Result<Self> {
                Ok(Self {
                    remaining: emit_count,
                })
            }

            fn create(&self, _cancellation_token: CancellationToken) -> io::Result<AudioStream> {
                let mut remaining = self.remaining;
                Ok(AudioStream(Box::pin(stream! {
                    let mut value = 0x1234_5678_u32;

                    loop {
                        if remaining == 0 {
                            return;
                        }
                        value = value.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        yield Ok(Bytes::from(value.to_le_bytes().to_vec()));
                        remaining -= 1;
                    }
                })))
            }
        }

        pub(super) struct ImmediateBadCaptureRecorder;

        impl AudioCapture for ImmediateBadCaptureRecorder {
            type CaptureOption = ();

            fn new(_capture_option: Self::CaptureOption) -> io::Result<Self> {
                Ok(Self)
            }

            fn create(&self, _cancellation_token: CancellationToken) -> io::Result<AudioStream> {
                Err(io::Error::other("immediate bad capture boom!"))
            }
        }

        pub(super) struct BadCaptureRecorder {
            success_count: usize,
        }

        impl AudioCapture for BadCaptureRecorder {
            type CaptureOption = usize;

            fn new(success_count: Self::CaptureOption) -> io::Result<Self> {
                Ok(Self { success_count })
            }

            fn create(&self, _cancellation_token: CancellationToken) -> io::Result<AudioStream> {
                let mut remaining = self.success_count;
                Ok(AudioStream(Box::pin(stream! {
                    while remaining > 0 {
                        yield Ok(Bytes::from(vec![0x12, 0x34, 0x56, 0x78]));
                        remaining -= 1;
                    }
                    yield Err(io::Error::other("bad capture boom!"));
                })))
            }
        }
    }

    mod mock_clients {
        use anyhow::anyhow;
        use async_stream::stream;
        use tokio_stream::StreamExt;

        use base_client::audio_stream::AudioStream;
        use base_client::grpc_server::TranscribeResponse;
        use base_client::transcribe_stream::TranscribeStream;

        use crate::client::BackendClient;

        pub(super) struct ImmediateBadAsrClient;

        #[async_trait::async_trait]
        impl BackendClient for ImmediateBadAsrClient {
            async fn create_transcription_stream(
                &self,
                _audio_stream: AudioStream,
            ) -> Result<TranscribeStream<anyhow::Error>, anyhow::Error> {
                Err(anyhow!("immediate bad asr client boom!"))
            }
        }

        pub(super) struct BadAsrClient {
            pub(crate) remaining_success: usize,
        }

        #[async_trait::async_trait]
        impl BackendClient for BadAsrClient {
            async fn create_transcription_stream(
                &self,
                mut audio_stream: AudioStream,
            ) -> Result<TranscribeStream<anyhow::Error>, anyhow::Error> {
                let remaining_success = self.remaining_success;
                Ok(TranscribeStream::new(Box::pin(stream! {
                    let mut success = 0;
                    while let Some(chunk) = audio_stream.next().await {
                        let _chunk = match chunk {
                            Ok(chunk) => chunk,
                            Err(err) => {
                                yield Err(err.into());
                                return;
                            }
                        };
                        if (success < remaining_success) {
                            yield Ok(TranscribeResponse {
                                begin_time: 0,
                                sentence_end: false,
                                text: "ok".to_string(),
                            });
                            success += 1;
                        } else {
                            yield Err(anyhow!("bad asr client boom!"));
                        }
                    };
                })))
            }
        }

        pub(super) struct YesAsrClient;

        #[async_trait::async_trait]
        impl BackendClient for YesAsrClient {
            async fn create_transcription_stream(
                &self,
                mut audio_stream: AudioStream,
            ) -> Result<TranscribeStream<anyhow::Error>, anyhow::Error> {
                Ok(TranscribeStream::new(Box::pin(stream! {
                    while let Some(chunk) = audio_stream.next().await {
                        let _chunk = match chunk {
                            Ok(chunk) => chunk,
                            Err(err) => {
                                yield Err(err.into());
                                return;
                            }
                        };
                         yield Ok(TranscribeResponse {
                            begin_time: 0,
                            sentence_end: false,
                            text: "yes".to_string(),
                        })
                    };
                })))
            }
        }
    }

    mod mock_services {
        use std::collections::BTreeMap;
        use std::sync::Arc;

        use base_client::audio_stream::AudioCapture;

        use crate::client::BackendClient;
        use crate::client_store::ClientStore;
        use crate::service::DictypeService;
        use crate::service::tests::mock_clients::*;
        use crate::service::tests::mock_recorders::*;

        pub(super) fn bad_capture_service(
            success_count: usize,
        ) -> DictypeService<BadCaptureRecorder> {
            let mut clients = BTreeMap::new();
            clients.insert(
                "yes-asr".to_string(),
                Arc::new(YesAsrClient {}) as Arc<dyn BackendClient + Send + Sync>,
            );
            DictypeService::new(
                ClientStore::from_clients(clients),
                BadCaptureRecorder::new(success_count).expect("BadCaptureRecorder must initialize"),
            )
        }

        pub(super) fn immediate_bad_capture_service() -> DictypeService<ImmediateBadCaptureRecorder>
        {
            let mut clients = BTreeMap::new();
            clients.insert(
                "yes-asr".to_string(),
                Arc::new(YesAsrClient {}) as Arc<dyn BackendClient + Send + Sync>,
            );
            DictypeService::new(
                ClientStore::from_clients(clients),
                ImmediateBadCaptureRecorder::new(())
                    .expect("ImmediateBadCaptureRecorder must initialize"),
            )
        }

        pub(super) fn asr_service(
            capture_count: usize,
            asr_count_before_bad: usize,
        ) -> DictypeService<NoiseRecorder> {
            let mut clients = BTreeMap::new();
            clients.insert(
                "immediate-bad-asr".to_string(),
                Arc::new(ImmediateBadAsrClient {}) as Arc<dyn BackendClient + Send + Sync>,
            );
            clients.insert(
                "bad-asr".to_string(),
                Arc::new(BadAsrClient {
                    remaining_success: asr_count_before_bad,
                }) as Arc<dyn BackendClient + Send + Sync>,
            );
            clients.insert(
                "yes-asr".to_string(),
                Arc::new(YesAsrClient {}) as Arc<dyn BackendClient + Send + Sync>,
            );

            DictypeService::new(
                ClientStore::from_clients(clients),
                NoiseRecorder::new(capture_count).expect("NoiseRecorder must initialize"),
            )
        }
    }

    #[tokio::test]
    async fn capture_ends() {
        let capture_count = 1024;
        let asr_count_before_bad = 0; // does not matter
        let service = asr_service(capture_count, asr_count_before_bad);

        let make_request_to_immediate_bad_asr = async || {
            let response = service
                .transcribe(Request::new(TranscribeRequest {
                    profile_name: "yes-asr".to_string(),
                }))
                .await
                .expect("transcribe should return a stream");
            let mut stream = response.into_inner();

            let mut success_count = 0;
            while let Some(result) = stream.next().await {
                let success = result.expect("stream should not fail");
                assert_eq!(success.text, "yes");
                success_count += 1;
            }
            assert_eq!(success_count, capture_count);
            assert!(stream.next().await.is_none());
        };
        make_request_to_immediate_bad_asr().await;
        make_request_to_immediate_bad_asr().await;
    }

    #[tokio::test]
    async fn transcribe_returns_invalid_argument_for_unknown_profile() {
        let service = asr_service(1024, 0);
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
        let service = asr_service(1024, 0);

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

    #[tokio::test]
    async fn bad_asr() {
        let capture_count = 1024;
        let asr_count_before_bad = capture_count - 1;
        let service = asr_service(capture_count, asr_count_before_bad);

        let make_request_to_bad_asr = async || {
            let response = service
                .transcribe(Request::new(TranscribeRequest {
                    profile_name: "bad-asr".to_string(),
                }))
                .await
                .expect("transcribe should return a stream");
            let mut stream = response.into_inner();

            let mut success_count = 0;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(result) => {
                        success_count += 1;
                    }
                    Err(err) => {
                        assert_eq!(err.code(), Code::Internal);
                        assert!(err.message().contains("bad asr client boom!"));
                        assert!(!service.state.lock().expect("state poisoned").is_some());
                    }
                }
            }
            assert_eq!(success_count, asr_count_before_bad);
            assert!(stream.next().await.is_none());
        };
        make_request_to_bad_asr().await;
        make_request_to_bad_asr().await;
    }

    #[tokio::test]
    async fn immediate_bad_asr() {
        let service = asr_service(10240, 1024);

        let make_request_to_immediate_bad_asr = async || {
            let response = service
                .transcribe(Request::new(TranscribeRequest {
                    profile_name: "immediate-bad-asr".to_string(),
                }))
                .await
                .expect("transcribe should return a stream");
            let mut stream = response.into_inner();
            let first = stream.next().await.expect("stream should not be empty");
            let err = first.expect_err("stream should fail");
            assert_eq!(err.code(), Code::Internal);
            assert!(err.message().contains("immediate bad asr client boom!"));
            assert!(stream.next().await.is_none());
        };
        make_request_to_immediate_bad_asr().await;
        make_request_to_immediate_bad_asr().await;
    }

    #[tokio::test]
    async fn bad_capture() {
        let success_capture_count = 10;
        let service = bad_capture_service(success_capture_count);

        let make_request_to_bad_asr = async || {
            let response = service
                .transcribe(Request::new(TranscribeRequest {
                    profile_name: "yes-asr".to_string(),
                }))
                .await
                .expect("transcribe should fail");

            let mut stream = response.into_inner();
            let mut success_count = 0;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(result) => {
                        success_count += 1;
                    }
                    Err(err) => {
                        assert_eq!(err.code(), Code::Internal);
                        assert!(err.message().contains("bad capture boom!"));
                    }
                }
            }
            assert_eq!(success_count, success_capture_count);
            assert!(stream.next().await.is_none());
        };
        make_request_to_bad_asr().await;
        make_request_to_bad_asr().await;
    }

    #[tokio::test]
    async fn immediate_bad_capture() {
        let service = immediate_bad_capture_service();

        let make_request_to_immediate_bad_asr = async || {
            let response = service
                .transcribe(Request::new(TranscribeRequest {
                    profile_name: "yes-asr".to_string(),
                }))
                .await
                .expect("transcribe should fail");

            let mut stream = response.into_inner();
            let first = stream
                .next()
                .await
                .expect("stream should not be empty")
                .expect_err("stream should fail");
            assert!(stream.next().await.is_none());
        };
        make_request_to_immediate_bad_asr().await;
        make_request_to_immediate_bad_asr().await;
    }

    #[tokio::test]
    async fn busy() {
        let service = asr_service(10240, 0 /* does not matter */);

        let make_request_to_busy_asr = async || {
            service
                .transcribe(Request::new(TranscribeRequest {
                    profile_name: "yes-asr".to_string(),
                }))
                .await
        };

        let mut first_stream = make_request_to_busy_asr()
            .await
            .expect("transcribe should succeed")
            .into_inner();

        let second_err = make_request_to_busy_asr()
            .await
            .expect_err("second call must fail");
        assert_eq!(second_err.code(), Code::AlreadyExists);
        assert_eq!(second_err.message(), "request exists");

        let stop_response = service
            .stop(Request::new(StopRequest {}))
            .await
            .expect("stop should succeed")
            .into_inner();
        assert!(stop_response.stopped);

        while let Some(response) = first_stream.next().await {
            assert!(response.is_ok());
        }

        let restarted = make_request_to_busy_asr()
            .await
            .expect("transcribe should succeed")
            .into_inner()
            .next()
            .await
            .expect("transcribe should succeed again after stop");
    }
}
