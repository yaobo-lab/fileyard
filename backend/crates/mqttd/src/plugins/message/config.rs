use serde::{
    Deserialize, Serialize,
    de::{self, Deserializer},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct PluginConfig {
    #[serde(deserialize_with = "PluginConfig::deserialize_storage_cfg")]
    pub storage: rmqtt_storage::Config,
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
    fn deserialize_storage_cfg<'de, D>(
        deserializer: D,
    ) -> std::result::Result<rmqtt_storage::Config, D::Error>
    where
        D: Deserializer<'de>,
    {
        let storage = serde_json::Value::deserialize(deserializer)?;
        match serde_json::from_value::<rmqtt_storage::Config>(storage) {
            Err(e) => Err(de::Error::custom(e.to_string())),
            Ok(s_cfg) => Ok(s_cfg),
        }
    }

    #[inline]
    pub(super) fn to_json(&self) -> serde_json::Value {
        serde_json::json!(self)
    }
}
