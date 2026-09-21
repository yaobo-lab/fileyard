use super::super::PublishParams;
use super::config::PluginConfig;
use super::{log_prefix, utils::aes128_cbc_decrypt};
use anyhow::anyhow;
use async_trait::async_trait;
use bytestring::ByteString;
use rmqtt::context::ServerContext;
use rmqtt::utils::timestamp_millis;
use rmqtt::{
    acl::AuthInfo,
    codec::v5::SubscribeAckReason,
    hook::{Handler, HookResult, Parameter, ReturnType},
    types::{AuthResult, ConnectInfo, PublishAclResult, SubscribeAclResult},
};
use serde_json::json;
use std::sync::Arc;
pub(super) struct AuthHandler {
    cfg: Arc<PluginConfig>,
    scx: ServerContext,
}

fn auth_fail_result(err_msg: String) -> ReturnType {
    log::warn!("{log_prefix} [auth_fail_result]: {}", err_msg);
    (
        false,
        Some(HookResult::AuthResult(AuthResult::BadUsernameOrPassword)),
    )
}

fn sub_acl_fail_result(err_msg: String) -> ReturnType {
    log::warn!("{log_prefix} [sub_acl_fail_result]: {}", err_msg);
    (
        false,
        Some(HookResult::SubscribeAclResult(
            SubscribeAclResult::new_failure(SubscribeAckReason::NotAuthorized),
        )),
    )
}

fn pub_acl_fail_result(topic: &str, user: &str, reason_string: Option<ByteString>) -> ReturnType {
    log::warn!(
        "{log_prefix} [pub_acl_fail_result]: user:{} publish topic: {} acl fail:{:?}",
        user,
        topic,
        reason_string
    );

    (
        false,
        Some(HookResult::PublishAclResult(PublishAclResult::rejected(
            false,
            reason_string,
        ))),
    )
}

impl AuthHandler {
    pub(super) fn new(cfg: Arc<PluginConfig>, scx: ServerContext) -> Self {
        Self { cfg, scx }
    }

    //授权验证
    // 成功返回OK((sku_id, superuser))
    // 失败返回Err
    fn _verify_login(
        pwd: &bytes::Bytes,
        ipaddr_str: &str,
        aes_cbc_secret_key: &str,
        aes_cbc_iv_val: &str,
    ) -> anyhow::Result<(String, bool)> {
        let pwd = ByteString::try_from(pwd.clone());
        if let Err(e) = pwd {
            return Err(anyhow!("{log_prefix}::passwd decode err: {}", e));
        }
        let pwd = pwd.unwrap_or_default();

        let passwd_decode = aes128_cbc_decrypt(&pwd, aes_cbc_secret_key, aes_cbc_iv_val);
        if let Err(e) = passwd_decode {
            return Err(anyhow!("{log_prefix} aes128_cbc_decrypt err: {}", e));
        }
        let passwd_decode = passwd_decode.unwrap_or_default();

        let passwd: Vec<&str> = passwd_decode.split("|").collect();
        if passwd.len() != 4 {
            return Err(anyhow!(
                "{log_prefix} passwd_decode: {} passwd.len() != 4",
                passwd_decode,
            ));
        }

        if passwd[0] != ipaddr_str {
            return Err(anyhow!(
                "{log_prefix} passwd[0] != ipaddr_str  passwd[0]:{} ipaddr_str:{ipaddr_str}",
                passwd[0]
            ));
        }

        let sku_id = passwd[2].to_owned();
        //判断是否超级用户
        let superuser = passwd[0] == "127.0.0.1";
        Ok((sku_id, superuser))
    }
}

#[async_trait]
impl Handler for AuthHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            //订阅验证
            Parameter::ClientSubscribeCheckAcl(s, sub) => {
                //判断是否是超级用户
                let res = s.superuser().await;
                if let Err(e) = res {
                    return sub_acl_fail_result(format!("sub check error: {}", e));
                }
                let superuser = res.unwrap_or(false);

                if superuser {
                    return (
                        false,
                        Some(HookResult::SubscribeAclResult(
                            SubscribeAclResult::new_success(sub.opts.qos(), None),
                        )),
                    );
                }

                //子设备 只能订阅自身的topic
                let Some(skuid) = s.username() else {
                    return sub_acl_fail_result(format!("username is empty"));
                };
                let uuid = s.id.client_id.clone();
                let skuid_str = skuid.to_string();
                let mut skuid = skuid_str.clone();
                let cfg = self.cfg.clone();
                //替换skuid 前缀
                for prix in cfg.sub_devices_start_with_str.iter() {
                    skuid = skuid.replace(prix, "")
                }

                let allow = cfg.sub_devices_sub_topics.iter().any(|t| {
                    let allow_topic = t.replace("{skuid}", &skuid).replace("{uuid}", &uuid);
                    log::trace!(
                        "allow_topic: {allow_topic}, sub.topic_filter: {}",
                        sub.topic_filter
                    );
                    if allow_topic.to_lowercase() == sub.topic_filter.to_lowercase() {
                        return true;
                    }
                    if super::utils::topic_match_one(&sub.topic_filter, t) {
                        return true;
                    }
                    if super::utils::topic_match_all(&sub.topic_filter, t) {
                        return true;
                    }
                    false
                });

                if !allow {
                    let s = format!(
                        "{log_prefix} client_id={uuid} usr_name={skuid_str} sub topic: {} rejected",
                        sub.topic_filter
                    );
                    return sub_acl_fail_result(s);
                }

                return (
                    false,
                    Some(HookResult::SubscribeAclResult(
                        SubscribeAclResult::new_success(sub.opts.qos(), None),
                    )),
                );
            }

            //发布验证
            Parameter::MessagePublishCheckAcl(s, p) => {
                //判断是否是超级用户
                let res = s.superuser().await;
                if let Err(e) = res {
                    log::error!("{log_prefix}  check superuser error: {}", e);
                    return pub_acl_fail_result("", "", Some("check superuser error".into()));
                }

                let superuser = res.unwrap_or(false);

                if superuser {
                    return (
                        false,
                        Some(HookResult::PublishAclResult(PublishAclResult::allow())),
                    );
                }

                //子设备 只能订阅自身的topic
                let Some(skuid) = s.username() else {
                    return pub_acl_fail_result("", "", Some("username is emtpy".into()));
                };
                let skuid_str = skuid.clone().to_string();
                let uuid = s.id.client_id.clone();
                let mut skuid = skuid_str.clone();
                let cfg = self.cfg.clone();
                //替换skuid 前缀
                for prix in cfg.sub_devices_start_with_str.iter() {
                    skuid = skuid.replace(prix, "")
                }

                let allow = cfg.sub_devices_pub_topics.iter().any(|t| {
                    let allow_topic = t.replace("{skuid}", &skuid).replace("{uuid}", &uuid);
                    if allow_topic.to_lowercase() == p.topic.to_lowercase() {
                        return true;
                    }
                    if super::utils::topic_match_all(&p.topic, t) {
                        return true;
                    }
                    if super::utils::topic_match_one(&p.topic, t) {
                        return true;
                    }
                    false
                });

                if !allow {
                    return pub_acl_fail_result(
                        &p.topic,
                        &skuid_str,
                        Some(
                            format!(
                                "client_id={uuid} usr_name={skuid_str} publish topic: {} no permission",
                                p.topic
                            )
                            .into(),
                        ),
                    );
                }

                return (
                    false,
                    Some(HookResult::PublishAclResult(PublishAclResult::allow())),
                );
            }

            //连接验证
            Parameter::ClientAuthenticate(v) => {
                log::debug!(
                    "{log_prefix}::[ClientAuthenticate]: username={:?}, password={:?}, client_id={:?} ipaddr={:?}",
                    v.username(),
                    v.password(),
                    v.client_id(),
                    v.ipaddress(),
                );

                let Some(usr_name) = v.username() else {
                    return auth_fail_result("username is empty".into());
                };
                let Some(pwd) = v.password() else {
                    return auth_fail_result("pwd is empty".into());
                };
                let Some(ipaddr) = v.ipaddress() else {
                    return auth_fail_result("ipaddr is empty".into());
                };
                if v.client_id().is_empty() {
                    return auth_fail_result("client_id is empty".into());
                };
                let ipaddr_str = ipaddr.ip().to_string();

                let cfg = self.cfg.clone();

                let info = if !cfg.unverify_login {
                    let v = match Self::_verify_login(
                        &pwd,
                        &ipaddr_str,
                        &cfg.aes_cbc_secret_key,
                        &cfg.aes_cbc_iv_val,
                    ) {
                        Ok(v) => v,
                        Err(e) => {
                            return auth_fail_result(format!(
                                "verify login error: {}",
                                e.to_string()
                            ));
                        }
                    };
                    (v.0, v.1)
                } else {
                    ("".to_string(), true)
                };

                //判断是否超级用户
                let superuser = info.1
                    || cfg
                        .superuser_devices_start_with_str
                        .iter()
                        .any(|t| usr_name.starts_with(t));

                //获取 endpoint_id 与 版本号
                let mut endpoint_id: Option<ByteString> = None;
                let mut version: Option<ByteString> = None;
                let mut class_id: Option<ByteString> = None;

                match v {
                    ConnectInfo::V3(_, _) => {
                        let s = format!(
                            "{log_prefix} usr_name:{},clientid:{} mqtt v3.1.1 not support",
                            usr_name,
                            v.client_id()
                        );
                        return auth_fail_result(s);
                    }
                    ConnectInfo::V5(_, c) => {
                        for (key, v) in c.user_properties.iter() {
                            if key == "EndpointId" {
                                endpoint_id = Some(v.clone());
                            }
                            if key == "Version" {
                                version = Some(v.clone());
                            }
                            if key == "ClassId" {
                                class_id = Some(v.clone());
                            }
                        }
                    }
                }

                log::debug!(
                    "{log_prefix} auth usr_name:{},clientid:{}  is superuser: {} EndpointId:{:?},Version:{:?} ClassId:{:?}",
                    usr_name,
                    v.client_id(),
                    superuser,
                    endpoint_id,
                    version,
                    class_id,
                );

                //子设备必须要有版本号，与 端点id 与分类
                if !superuser {
                    if endpoint_id.is_none() || version.is_none() || class_id.is_none() {
                        return auth_fail_result(format!(
                            "You're not a superuser some params is empty: EndpointId: {:?}, Version: {:?}, ClassId: {:?} ",
                            endpoint_id, version, class_id
                        ));
                    }
                }

                let usr = AuthInfo {
                    superuser,
                    expire_at: None,
                    rules: vec![],
                };

                //推送连接成功
                let now = timestamp_millis();
                let now_time = super::super::to_timestamp_millis(now);
                let mut msg = PublishParams::default();
                msg.topic = Some("/aam/broker/sub/event".into());
                msg.payload = json!({
                    "action":"connected",
                    "ipaddress":ipaddr_str,
                    "clientid": v.client_id().to_string(),
                    "username": v.username().map(|u| u.to_string()),
                    "keep_alive_period": v.keep_alive(),
                    "endpoint_id":endpoint_id.unwrap_or("".into()),
                    "version":version.unwrap_or("".into()),
                    "class_id":class_id.unwrap_or("".into()),
                    "time": now_time,
                    "sku_id": info.0,
                })
                .to_string();

                if let Err(e) = super::super::re_publish_message(msg, &self.scx, None).await {
                    log::error!("{log_prefix}: message publish err: {e:?}");
                }

                return (
                    false,
                    Some(HookResult::AuthResult(AuthResult::Allow(
                        superuser,
                        Some(usr),
                    ))),
                );
            }
            _ => {
                log::error!("{log_prefix} unimplemented, {param:?}")
            }
        }
        (true, acc)
    }
}
