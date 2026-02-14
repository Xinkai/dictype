use serde::{Deserialize, Serialize};

use paraformer_v2_client::config::ParaformerV2Config;
use qwen_v3_client::config::QwenV3Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "Backend", content = "Config")]
pub enum ProfileConfig {
    ParaformerV2(ParaformerV2Config),
    QwenV3(QwenV3Config),
}

impl ProfileConfig {
    #[must_use]
    pub const fn backend_name(&self) -> &'static str {
        match self {
            Self::ParaformerV2(_) => "ParaformerV2",
            Self::QwenV3(_) => "QwenV3",
        }
    }
}
