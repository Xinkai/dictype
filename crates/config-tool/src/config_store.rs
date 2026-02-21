use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use pulseaudio_recorder::PulseAudioConfig;

use crate::config_store_error::ConfigStoreError;
use crate::profile_config::ProfileConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    #[serde(rename = "PulseAudio", default)]
    pulseaudio: PulseAudioConfig,

    #[serde(flatten)]
    profiles: BTreeMap<String, ProfileConfig>,
}

impl ConfigFile {
    pub fn load() -> Result<Self, ConfigStoreError> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(&path)?;
        let config = toml::from_str(&data)?;
        Ok(config)
    }

    #[must_use]
    pub const fn profiles(&self) -> &BTreeMap<String, ProfileConfig> {
        &self.profiles
    }

    #[must_use]
    pub const fn pulseaudio(&self) -> &PulseAudioConfig {
        &self.pulseaudio
    }
}

fn config_path() -> Result<PathBuf, ConfigStoreError> {
    let home = std::env::var_os("HOME").ok_or(ConfigStoreError::MissingHome)?;
    let mut path = PathBuf::from(home);
    path.push(".config");
    path.push("dictype.toml");
    Ok(path)
}
