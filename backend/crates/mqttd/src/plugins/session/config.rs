use kv_storage::Config;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginConfig {
    pub storage: Config,
}

impl PluginConfig {
    #[inline]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self)
    }
}
