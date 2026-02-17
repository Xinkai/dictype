use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use base_client::asr_client::AsrClient;
use config_tool::config_store::ConfigFile;
use config_tool::profile_config::ProfileConfig;
use paraformer_v2_client::client::ParaformerV2Client;
use qwen_v3_client::client::QwenV3Client;

use crate::client::AsrClientInstance;

pub struct ClientStore {
    clients: Arc<Mutex<BTreeMap<String, Arc<AsrClientInstance>>>>,
}

impl ClientStore {
    pub fn load(config_file: &ConfigFile) -> Self {
        let mut clients = BTreeMap::new();
        for (profile_name, config) in config_file.profiles() {
            match &config {
                ProfileConfig::ParaformerV2(paraformer_v2) => {
                    clients.insert(
                        profile_name.clone(),
                        Arc::new(AsrClientInstance::ParaformerV2(ParaformerV2Client::new(
                            paraformer_v2.clone(),
                        ))),
                    );
                }
                ProfileConfig::QwenV3(qwen_v3) => {
                    clients.insert(
                        profile_name.clone(),
                        Arc::new(AsrClientInstance::QwenV3(QwenV3Client::new(
                            qwen_v3.clone(),
                        ))),
                    );
                }
            }
        }

        Self {
            clients: Arc::new(Mutex::new(clients)),
        }
    }

    pub fn get_asr_client_for_profile(&self, profile_name: &str) -> Option<Arc<AsrClientInstance>> {
        let locked = self.clients.lock().expect("locking asr clients");

        locked.get(profile_name).cloned()
    }
}
