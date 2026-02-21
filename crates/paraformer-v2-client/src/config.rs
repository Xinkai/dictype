use serde::{Deserialize, Serialize};

use crate::types;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParaformerV2Config {
    pub dashscope_api_key: String,
    pub dashscope_websocket_url: Option<String>,
    pub disfluency_removal_enabled: Option<bool>,
    pub language_hints: Option<Vec<types::run_task::request::Language>>,
    pub semantic_punctuation_enabled: Option<bool>,
    pub max_sentence_silence: Option<u32>,
    pub multi_threshold_mode_enabled: Option<bool>,
    pub punctuation_prediction_enabled: Option<bool>,
    pub inverse_text_normalization_enabled: Option<bool>,
}

impl ParaformerV2Config {
    pub const DEFAULT_WEBSOCKET_URL: &str = "wss://dashscope.aliyuncs.com/api-ws/v1/inference";

    #[must_use]
    pub fn websocket_url(&self) -> &str {
        self.dashscope_websocket_url
            .as_deref()
            .unwrap_or(Self::DEFAULT_WEBSOCKET_URL)
    }
}
