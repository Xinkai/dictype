#[allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
pub mod proto {
    tonic::include_proto!("dictype");
}

pub use proto::dictype_server::{Dictype, DictypeServer};
pub use proto::{StopRequest, StopResponse, TranscribeRequest, TranscribeResponse};
