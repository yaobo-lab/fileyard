use rmqtt::Result;
use rmqtt::hook::Priority;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct PluginConfig {
    #[serde(default = "PluginConfig::queue_capacity_default")]
    pub(super) queue_capacity: usize,
    pub(super) start_with_topics: Vec<String>,
    #[serde(default = "PluginConfig::priority_default")]
    pub(super) priority: Priority,
}

#[allow(dead_code)]
impl PluginConfig {
    fn queue_capacity_default() -> usize {
        300
    }

    //越小越优先执行
    fn priority_default() -> Priority {
        50
    }

    #[inline]
    pub fn to_json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}
