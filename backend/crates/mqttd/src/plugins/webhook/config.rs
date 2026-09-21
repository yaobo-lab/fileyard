use super::rule::{Rule, Url};
use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};
use rmqtt::{
    Result,
    hook::{Priority, Type},
    utils::deserialize_duration,
};
use serde::{
    Deserialize, Serialize,
    de::{self},
};
use std::time::Duration;
type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginConfig {
    #[serde(default = "PluginConfig::queue_capacity_default")]
    pub queue_capacity: usize,
    #[serde(default = "PluginConfig::concurrency_limit_default")]
    pub concurrency_limit: usize,
    #[serde(default)]
    pub urls: Vec<Url>,
    #[serde(default)]
    #[deprecated]
    http_urls: Vec<Url>,
    #[serde(
        default = "PluginConfig::http_timeout_default",
        deserialize_with = "deserialize_duration"
    )]
    pub http_timeout: Duration,
    #[serde(rename = "rule")]
    #[serde(default, deserialize_with = "PluginConfig::deserialize_rules")]
    pub rules: HashMap<Type, Vec<Rule>>,
    #[serde(
        default = "PluginConfig::retry_max_elapsed_time_default",
        deserialize_with = "deserialize_duration"
    )]
    pub retry_max_elapsed_time: Duration,
    #[serde(default = "PluginConfig::retry_multiplier_default")]
    pub retry_multiplier: f64,
    #[serde(default = "PluginConfig::priority_default")]
    pub(super) priority: Priority,
}

impl PluginConfig {
    fn queue_capacity_default() -> usize {
        300
    }
    fn concurrency_limit_default() -> usize {
        10
    }
    //越小越优先执行
    fn priority_default() -> Priority {
        60
    }
    fn http_timeout_default() -> Duration {
        Duration::from_secs(5)
    }
    fn retry_max_elapsed_time_default() -> Duration {
        Duration::from_secs(60)
    }
    fn retry_multiplier_default() -> f64 {
        2.5
    }

    fn deserialize_rules<'de, D>(
        deserializer: D,
    ) -> std::result::Result<HashMap<Type, Vec<Rule>>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let mut rules_cfg: HashMap<String, Vec<Rule>> = HashMap::deserialize(deserializer)?;
        let mut rules = HashMap::default();
        for (typ, r) in rules_cfg.drain() {
            rules.insert(Type::from(typ.as_str()), r);
        }
        Ok(rules)
    }

    #[inline]
    pub fn to_json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }

    #[inline]
    pub fn get_backoff_strategy(&self) -> ExponentialBackoff {
        ExponentialBackoffBuilder::new()
            .with_max_elapsed_time(Some(self.retry_max_elapsed_time))
            .with_multiplier(self.retry_multiplier)
            .build()
    }

    #[allow(deprecated)]
    #[inline]
    pub fn urls(&self) -> &Vec<Url> {
        &self.urls
    }

    #[allow(deprecated)]
    #[inline]
    pub fn merge_urls(&mut self) {
        self.urls.append(&mut self.http_urls)
    }
}
