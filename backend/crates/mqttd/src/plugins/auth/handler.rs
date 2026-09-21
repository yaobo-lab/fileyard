use super::super::PublishParams;
use super::config::PluginConfig;
use super::log_prefix;
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
        _ipaddr_str: &str,
        _aes_cbc_secret_key: &str,
        _aes_cbc_iv_val: &str,
    ) -> anyhow::Result<(String, bool)> {
        let pwd = ByteString::try_from(pwd.clone());
        if let Err(e) = pwd {
            return Err(anyhow!("{log_prefix}::passwd decode err: {}", e));
        }
        let _pwd = pwd.unwrap_or_default();
        Ok(("".into(), true))
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
                let Some(_pwd) = v.password() else {
                    return auth_fail_result("pwd is empty".into());
                };
                let Some(ipaddr) = v.ipaddress() else {
                    return auth_fail_result("ipaddr is empty".into());
                };
                if v.client_id().is_empty() {
                    return auth_fail_result("client_id is empty".into());
                };
                let ipaddr_str = ipaddr.ip().to_string();

                //判断是否超级用户
                let superuser = true;

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
                        for (key, _v) in c.user_properties.iter() {
                            if key == "EndpointId" {
                                println!("");
                            }
                            if key == "Version" {
                                println!("");
                            }
                            if key == "ClassId" {
                                println!("");
                            }
                        }
                    }
                }

                log::debug!(
                    "{log_prefix} auth usr_name:{},clientid:{}  is superuser: {}",
                    usr_name,
                    v.client_id(),
                    superuser,
                );

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
                    "time": now_time,
                })
                .to_string();

                if let Err(e) = super::super::re_publish_message(msg, &self.scx, None).await {
                    log::error!("{log_prefix}: message publish err: {e:?}");
                }

                // 登记客户端连接事件
                crate::events::notify_client_connected(crate::events::MqttClientConnectedEvent {
                    username: usr_name.to_string(),
                    client_id: v.client_id().to_string(),
                    ip_address: Some(ipaddr_str.clone()),
                    port: Some(ipaddr.port() as i32),
                    proto_ver: match v {
                        ConnectInfo::V3(_, _) => Some(4),
                        ConnectInfo::V5(_, _) => Some(5),
                    },
                    keepalive: Some(v.keep_alive() as i32),
                    clean_start: Some(v.clean_start()),
                });

                return (
                    false,
                    Some(HookResult::AuthResult(AuthResult::Allow(
                        superuser,
                        Some(usr),
                    ))),
                );
            }

            // 客户端断开连接
            Parameter::ClientDisconnected(session, reason) => {
                let u = session.id.username_ref();
                let username = if u.is_empty() { None } else { Some(u.to_string()) };
                let client_id = session.id.client_id.to_string();
                let reason_str = reason.to_string();

                crate::events::notify_client_disconnected(crate::events::MqttClientDisconnectedEvent {
                    username,
                    client_id: Some(client_id),
                    reason: Some(reason_str),
                });

                return (false, None);
            }

            _ => {
                log::error!("{log_prefix} unimplemented, {param:?}")
            }
        }
        (true, acc)
    }
}
