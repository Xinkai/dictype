use std::collections::BTreeMap;
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

    #[serde(rename = "Profiles", default)]
    profiles: BTreeMap<String, ProfileConfig>,
}

impl ConfigFile {
    pub fn parse(content: &str) -> Result<Self, ConfigStoreError> {
        let config = toml::from_str(content)?;
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

pub fn get_config_path() -> Result<PathBuf, ConfigStoreError> {
    let home = std::env::var_os("HOME").ok_or(ConfigStoreError::MissingHome)?;
    let mut path = PathBuf::from(home);
    path.push(".config");
    path.push("dictype.toml");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_path() {
        assert!(get_config_path().is_ok());
    }

    #[test]
    fn test_load_profiles() {
        let config = r#"
        [Profiles.Profile1]
        Backend = "ParaformerV2"
        Config = { dashscope_api_key = "fake" }
        "#;

        let config = ConfigFile::parse(config).unwrap();
        assert_eq!(config.profiles.len(), 1);
    }

    #[test]
    fn test_load_profiles_with_pulseaudio() {
        let config = r#"
        [PulseAudio]

        [Profiles.Profile1]
        Backend = "ParaformerV2"
        Config = { dashscope_api_key = "fake" }
        "#;

        let config = ConfigFile::parse(config).unwrap();
        assert_eq!(config.profiles.len(), 1);
    }

    #[test]
    fn test_reject_known_sections() {
        let config = r"
        [Unknown]
        a = 1
        ";

        assert!(ConfigFile::parse(config).is_err());
    }
}
