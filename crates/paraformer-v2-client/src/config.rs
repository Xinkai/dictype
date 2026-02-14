use serde::{Deserialize, Serialize};

use crate::types;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParaformerV2Config {
    pub dashscope_api_key: String,
    pub language_hints: Option<Vec<types::run_task::request::Language>>,
}

impl ParaformerV2Config {}
