use tokio_util::sync::CancellationToken;
use tonic::Status;
use tracing::{info, warn};

pub struct ServiceState {
    cancellation_token: Option<CancellationToken>,
}

impl ServiceState {
    pub(crate) const fn new() -> Self {
        Self {
            cancellation_token: None,
        }
    }

    pub(crate) const fn is_some(&self) -> bool {
        self.cancellation_token.is_some()
    }

    pub(crate) fn reset(&mut self) -> bool {
        if let Some(cancellation_token) = self.cancellation_token.take() {
            cancellation_token.cancel();
            info!("stop: stopped session");
            true
        } else {
            warn!("stop: no session running");
            false
        }
    }

    pub(crate) fn replace(&mut self, cancellation_token: CancellationToken) -> Result<(), Status> {
        let existing_cancellation = self.cancellation_token.replace(cancellation_token);

        match existing_cancellation {
            Some(existing) if !existing.is_cancelled() => {
                Err(Status::internal("cancellation_token already set"))?
            }
            _ => Ok(()),
        }
    }
}
