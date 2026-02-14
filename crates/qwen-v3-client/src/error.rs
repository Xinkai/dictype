use std::io;

use tokio_tungstenite::tungstenite::Error as WsError;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum QwenV3Error {
    /// Errors raised by the WebSocket layer.
    #[error("websocket error: {0}")]
    WebSocket(#[from] WsError),

    /// Errors raised while parsing or serializing JSON payloads.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    // Audio error
    #[error("audio error: {0}")]
    Audio(#[from] io::Error),

    /// A header value could not be encoded using HTTP header rules.
    #[error("invalid header value for `{0}`")]
    InvalidHeaderValue(&'static str),

    #[error("connection error")]
    Connection,

    #[error("connection closed: {0}")]
    Closed(String),
}
