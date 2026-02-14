use serde::{Deserialize, Serialize};

use crate::types::Language;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TurnDetection {
    pub threshold: f32,
    pub silence_duration_ms: u32,
}

impl Default for TurnDetection {
    fn default() -> Self {
        Self {
            threshold: 0.2,
            silence_duration_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QwenV3Config {
    pub dashscope_api_key: String,
    pub language: Option<Language>,
    pub turn_detection: Option<TurnDetection>,
}

impl QwenV3Config {}
