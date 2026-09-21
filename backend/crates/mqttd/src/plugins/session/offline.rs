use super::config::PluginConfig;
use super::log_prefix;
use super::storage::INFLIGHT_MESSAGES;
use super::storage_keys::*;
use async_trait::async_trait;
use kv_storage::{List, Map, StorageDB};
use rmqtt::hook::{Handler, HookResult, Parameter, ReturnType};
use std::sync::Arc;

#[allow(dead_code)]
pub(super) struct OfflineMessageHandler {
    pub(super) cfg: Arc<PluginConfig>,
    pub(super) storage_db: StorageDB,
}

impl OfflineMessageHandler {
    pub(super) fn new(cfg: Arc<PluginConfig>, storage_db: StorageDB) -> Self {
        Self { cfg, storage_db }
    }
}

#[async_trait]
impl Handler for OfflineMessageHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            //离线消息，有订阅客户端才会回调
            //客户端离线  发送端发出的消息
            Parameter::OfflineMessage(s, f, p) => {
                log::debug!(
                    "{log_prefix} offline message usrname:{:?},skuid:{} topic:{}",
                    s.username(),
                    s.id.client_id,
                    p.topic
                );
                let list_stored_key = make_list_stored_key(s.id.to_string());
                let storage_db = self.storage_db.clone();
                let id = s.id.clone();
                let max_mqueue_len = s.listen_cfg().max_mqueue_len;
                let p = (*p).clone();
                let f = f.clone();

                tokio::spawn(async move {
                    match storage_db.list(list_stored_key.as_ref(), None).await {
                        Ok(offlines_list) => {
                            let res = offlines_list
                                .push_limit::<super::OfflineMessageOptionType>(
                                    &Some((id.client_id.clone(), f, p)),
                                    max_mqueue_len,
                                    true,
                                )
                                .await;
                            if let Err(e) = res {
                                log::warn!("{log_prefix} {id:?} save offline messages error, {e}")
                            }
                        }
                        Err(e) => {
                            log::warn!("{log_prefix} {id:?} save offline messages error, {e}")
                        }
                    }
                });
            }

            //客户端正在处理消息,突然离线，但没有回复服务器的消息
            Parameter::OfflineInflightMessages(s, m) => {
                log::debug!("{log_prefix} OfflineInflightMessages len: {:?}", m.len(),);
                let map_stored_key = make_map_stored_key(s.id.to_string());
                let storage_db = self.storage_db.clone();
                let inflight_messages = m.clone();
                let id = s.id.clone();
                tokio::spawn(async move {
                    match storage_db.map(map_stored_key.as_ref(), None).await {
                        Ok(m) => {
                            if let Err(e) = m.insert(INFLIGHT_MESSAGES, &inflight_messages).await {
                                log::warn!(
                                    "{log_prefix} {id:?} save offline inflight messages error, {e}"
                                )
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "{log_prefix} {id:?} save offline inflight messages error, {e}"
                            )
                        }
                    }
                });
            }

            _ => {
                log::error!("{log_prefix} unimplemented, {param:?}")
            }
        }
        (true, acc)
    }
}
