use crate::IType;
use async_trait::async_trait;
use base64::prelude::{BASE64_STANDARD, Engine};
use rmqtt::{
    hook::{self, Handler, HookResult, Parameter, ReturnType, Type},
    types::TopicFilter,
    utils::{format_timestamp_millis, timestamp_millis},
};
use serde_json::{self, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};

use tokio::{self, sync::RwLock, sync::mpsc::Sender};
// impl Handler
pub(super) type Message = (hook::Type, Option<TopicFilter>, serde_json::Value);

pub(super) struct WebHookHandler {
    pub(super) tx: Arc<RwLock<Sender<Message>>>,
    pub(super) chan_queue_count: Arc<AtomicIsize>,
}

#[async_trait]
impl Handler for WebHookHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        let typ = param.get_type();
        let now = timestamp_millis();
        let now_time = format_timestamp_millis(now);
        let bodys = match param {
            Parameter::ClientConnected(session) => {
                let mut body = session
                    .connect_info()
                    .await
                    .map(|c| c.to_hook_body())
                    .unwrap_or_default();
                if let Some(obj) = body.as_object_mut() {
                    obj.insert(
                        "connected_at".into(),
                        serde_json::Value::Number(serde_json::Number::from(
                            session.connected_at().await.unwrap_or_default(),
                        )),
                    );
                    obj.insert(
                        "session_present".into(),
                        serde_json::Value::Bool(
                            session.session_present().await.unwrap_or_default(),
                        ),
                    );
                    obj.insert("time".into(), serde_json::Value::String(now_time));
                }
                Some((None, body))
            }

            Parameter::ClientDisconnected(session, reason) => {
                let body = json!({
                    "node": session.id.node(),
                    "ipaddress": session.id.remote_addr,
                    "clientid": session.id.client_id,
                    "username": session.id.username_ref(),
                    "disconnected_at": session.disconnected_at().await.unwrap_or_default(),
                    "reason": reason.to_string(),
                    "time": now_time
                });
                Some((None, body))
            }

            Parameter::MessagePublish(_session, from, publish) => {
                let topic = &publish.topic;
                let body = json!({
                    "dup": publish.dup,
                    "retain": publish.retain,
                    "qos": publish.qos.value(),
                    "topic": topic,
                    "packet_id": publish.packet_id,
                    "payload": BASE64_STANDARD.encode(publish.payload.as_ref()),
                    "ts": publish.create_time,
                    "time": now_time
                });
                let body = from.to_from_json(body);
                Some((Some(topic.clone()), body))
            }

            Parameter::MessageDropped(to, from, publish, reason) => {
                if from.is_system() {
                    None
                } else {
                    let body = json!({
                        "dup": publish.dup,
                        "retain": publish.retain,
                        "qos": publish.qos.value(),
                        "topic": publish.topic,
                        "packet_id": publish.packet_id,
                        "payload": BASE64_STANDARD.encode(publish.payload.as_ref()),
                        "reason": reason.to_string(),
                        "pts": publish.create_time,
                        "ts": now,
                        "time": now_time
                    });
                    let mut body = from.to_from_json(body);
                    if let Some(to) = to {
                        body = to.to_to_json(body);
                    }
                    Some((None, body))
                }
            }
            _ => {
                log::error!("parameter is: {param:?}");
                None
            }
        };

        let body_msg = match &bodys {
            Some((topic, b)) => {
                let topic = topic.as_ref().map(|t| t.to_string()).unwrap_or_default();
                let msg = b.to_string();
                (topic, msg)
            }
            None => ("".to_string(), "".to_string()),
        };

        match typ {
            //设备上下线
            Type::ClientConnected | Type::ClientDisconnected => {
                log::debug!(
                    "type: {} topic:{}, body: {}",
                    typ.to_string(),
                    body_msg.0,
                    body_msg.1
                );
            }
            //设备消息被丢弃
            Type::MessageDropped => {
                log::debug!(
                    "type: {} topic:{}, body: {}",
                    typ.to_string(),
                    body_msg.0,
                    body_msg.1
                );
            }
            //设备消息发送
            Type::MessagePublish => {
                log::debug!(
                    "type: {} topic:{}, body: {}",
                    typ.to_string(),
                    body_msg.0,
                    body_msg.1
                );
            }
            _ => {
                log::debug!(
                    "type: {} topic:{}, body: {}",
                    typ.to_string(),
                    body_msg.0,
                    body_msg.1
                );
            }
        }

        if let Some((topic, body)) = bodys {
            let tx = self.tx.read().await.clone();
            if let Err(e) = tx.send((typ, topic, body)).await {
                log::warn!("web-hook send error, typ: {typ:?}, {e:?}");
            } else {
                self.chan_queue_count.fetch_add(1, Ordering::SeqCst);
            }
        }

        (true, acc)
    }
}
