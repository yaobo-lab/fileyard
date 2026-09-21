use super::{super::PublishParams, log_prefix};
use async_trait::async_trait;
use base64::prelude::{BASE64_STANDARD, Engine};

use bytestring::ByteString;
use rmqtt::{
    hook::{Handler, HookResult, Parameter, ReturnType},
    utils::timestamp_millis,
};
use serde_json::{self, json};

use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use tokio::{self, sync::RwLock, sync::mpsc::Sender};
pub(super) struct TransferHandler {
    pub(super) tx: Arc<RwLock<Sender<PublishParams>>>,
    pub(super) chan_queue_count: Arc<AtomicIsize>,
}

#[async_trait]
impl Handler for TransferHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        let typ = param.get_type();
        let now = timestamp_millis();
        let now_time = super::super::to_timestamp_millis(now);
        let mut msg = PublishParams::default();

        match param {
            Parameter::ClientDisconnected(s, reason) => {
                log::debug!(
                    "{log_prefix} client_disconnected: {}, reason_code: {reason:?}",
                    s.id.client_id
                );
                msg.topic = Some("/aam/broker/sub/event".into());
                msg.payload = json!({
                    "action":"disconnected",
                    "ipaddress": s.id.remote_addr,
                    "clientid": s.id.client_id,
                    "username": s.id.username_ref(),
                    "time": now_time,
                   // "subgateway":true,
                })
                .to_string();
            }

            Parameter::MessageDropped(to, _, publish, reason) => {
                let body = json!({
                    "dup": publish.dup,
                    "to":to,
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
                log::debug!("{log_prefix} message drop {}", body);
                return (true, acc);
            }

            Parameter::MessagePublish(session, _from, publish) => {
                let body = json!({
                    "retain": publish.retain,
                    "qos": publish.qos.value(),
                    "topic": &publish.topic,
                    "payload":std::str::from_utf8(& publish.payload).unwrap_or(&""),
                });
                if let Some(s) = session {
                    log::debug!(
                        "{log_prefix} [MessagePublish]: client_id: {} username:{}, ipaddr:{:?} body: {}",
                        s.id.client_id,
                        s.username().unwrap_or(&ByteString::default()),
                        s.id.remote_addr,
                        body.to_string()
                    );
                } else {
                    log::debug!("{log_prefix} [MessagePublish]: {}", body.to_string());
                }
            }

            _ => {
                log::error!("{log_prefix} parameter is: {param:?}");
            }
        }

        if msg.topic.is_none() {
            return (true, acc);
        }

        let tx = self.tx.read().await.clone();
        if let Err(e) = tx.send(msg).await {
            log::warn!("{log_prefix} send error, typ: {typ:?}, {e:?}");
        } else {
            self.chan_queue_count.fetch_add(1, Ordering::SeqCst);
        }
        (true, acc)
    }
}
