use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct PluginConfig {
    #[serde(flatten)]
    pub storage: kv_storage::Config,
    #[serde(default = "PluginConfig::cleanup_count_default")]
    pub cleanup_count: usize,
    #[serde(default = "PluginConfig::queue_max_count_default")]
    pub queue_max_count: usize,
}

impl PluginConfig {
    fn cleanup_count_default() -> usize {
        200
    }
    fn queue_max_count_default() -> usize {
        1000
    }

    #[inline]
    pub(super) fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self)
    }
}
