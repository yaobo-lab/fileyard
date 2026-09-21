use kv_storage::Config;
use rmqtt::Result;
use rmqtt::utils::{Bytesize, deserialize_duration_option};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginConfig {
    #[serde(deserialize_with = "PluginConfig::deserialize_storage")]
    pub storage: Config,
    #[serde(default = "PluginConfig::max_retained_messages_default")]
    pub max_retained_messages: isize, // = 0
    #[serde(default = "PluginConfig::max_payload_size_default")]
    pub max_payload_size: Bytesize, // = "1MB"
    #[serde(default, deserialize_with = "deserialize_duration_option")]
    pub retained_message_ttl: Option<Duration>,
}

impl PluginConfig {
    fn max_retained_messages_default() -> isize {
        0
    }
    fn max_payload_size_default() -> Bytesize {
        Bytesize::from(1024 * 1024)
    }

    #[inline]
    fn deserialize_storage<'de, D>(deserializer: D) -> std::result::Result<Config, D::Error>
    where
        D: Deserializer<'de>,
    {
        let storage = serde_json::Value::deserialize(deserializer)?;

        match serde_json::from_value::<Config>(storage) {
            Err(e) => Err(de::Error::custom(e.to_string())),
            Ok(s_cfg) => Ok(s_cfg),
        }
    }

    #[inline]
    pub fn to_json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}
