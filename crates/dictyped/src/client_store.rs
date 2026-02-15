use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use base_client::asr_client_factory::AsrClientFactory;
use config_tool::config_store::ConfigFile;
use config_tool::profile_config::ProfileConfig;
use paraformer_v2_client::client_factory::ParaformerV2ClientFactory;
use qwen_v3_client::client_factory::QwenV3ClientFactory;

use crate::client::ClientFactory;

pub struct ClientStore {
    clients: Arc<Mutex<BTreeMap<String, Arc<ClientFactory>>>>,
}

impl ClientStore {
    pub fn load(config_file: &ConfigFile) -> Self {
        let mut clients = BTreeMap::new();
        for (profile_name, config) in config_file.profiles() {
            match &config {
                ProfileConfig::ParaformerV2(paraformer_v2) => {
                    clients.insert(
                        profile_name.clone(),
                        Arc::new(ClientFactory::ParaformerV2(ParaformerV2ClientFactory::new(
                            paraformer_v2.clone(),
                        ))),
                    );
                }
                ProfileConfig::QwenV3(qwen_v3) => {
                    clients.insert(
                        profile_name.clone(),
                        Arc::new(ClientFactory::QwenV3(QwenV3ClientFactory::new(
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

    pub fn get_client_for_profile(&self, profile_name: &str) -> Option<Arc<ClientFactory>> {
        let locked = self.clients.lock().expect("locking clients");

        locked.get(profile_name).cloned()
    }
}
