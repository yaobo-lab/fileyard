use rmqtt::{Result, hook::Priority};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub(super) struct PluginConfig {
    //如果 订阅与发布 acl 拒绝，是否断开连接
    #[serde(default = "PluginConfig::disconnect_if_pub_rejected_default")]
    pub(super) disconnect_if_sub_pub_rejected: bool,

    //# 子设备 username 前缀标记
    #[serde(default = "PluginConfig::sub_devices_start_with_str_default")]
    pub(super) sub_devices_start_with_str: Vec<String>,

    //# 超级管理员 username 前缀标记
    #[serde(default = "PluginConfig::superuser_devices_start_with_str_default")]
    pub(super) superuser_devices_start_with_str: Vec<String>,

    #[serde(default = "PluginConfig::priority_default")]
    pub(super) priority: Priority,

    //子设备 允许的订阅 清单
    #[serde(default = "PluginConfig::sub_devices_sub_topics")]
    pub(super) sub_devices_sub_topics: Vec<String>,

    //子设备 允许的发布 清单
    #[serde(default = "PluginConfig::sub_devices_pub_topics_default")]
    pub(super) sub_devices_pub_topics: Vec<String>,

    // mqtt 加密通信
    #[serde(default)]
    pub(super) aes_cbc_secret_key: String,
    #[serde(default)]
    pub(super) aes_cbc_iv_val: String,

    // 是否允许未认证用户登录,true:允许--并且这些都是管理员,false:不允许
    #[serde(default = "PluginConfig::unverify_login_default")]
    pub(super) unverify_login: bool,
}

impl PluginConfig {
    #[inline]
    fn unverify_login_default() -> bool {
        false
    }

    //如果没有权限 ,即让 客户端断开连接
    fn disconnect_if_pub_rejected_default() -> bool {
        true
    }

    fn sub_devices_start_with_str_default() -> Vec<String> {
        vec!["aam_sub_".to_string()]
    }
    fn superuser_devices_start_with_str_default() -> Vec<String> {
        vec!["aam_superuser_".to_string(), "aam_admin_".to_string()]
    }

    fn sub_devices_pub_topics_default() -> Vec<String> {
        vec![
            "/aam/sub/response/{skuid}/{uuid}".to_string(),
            "/aam/sub/response/aam_sub_{skuid}/{uuid}".to_string(),
        ]
    }

    fn sub_devices_sub_topics() -> Vec<String> {
        vec![
            "/aam/sub/request/{skuid}/{uuid}".to_string(),
            "/aam/sub/request/aam_sub_{skuid}/{uuid}".to_string(),
            "/aam/sub/upgrade/{skuid}/{uuid}".to_string(),
            "/aam/sub/upgrade/aam_sub_{skuid}/{uuid}".to_string(),
            "/aam/sub/upgrade/app/{skuid}/{uuid}".to_string(),
            "/aam/sub/upgrade/app/aam_sub_{skuid}/{uuid}".to_string(),
        ]
    }
    //越小越优先执行
    fn priority_default() -> Priority {
        10
    }

    #[inline]
    pub(super) fn to_json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}
