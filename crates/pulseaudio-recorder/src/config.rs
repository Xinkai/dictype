use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PulseAudioConfig {
    pub preferred_source_name: Option<String>,
}
