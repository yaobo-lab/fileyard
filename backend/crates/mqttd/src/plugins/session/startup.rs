use super::RebuildChanType;
use super::config::PluginConfig;
use super::log_prefix;
use super::storage_info::StoredSessionInfos;
use super::storage_keys::*;
use async_trait::async_trait;
use futures::SinkExt;
use futures::channel::{mpsc, oneshot};
use kv_storage::StorageDB;
use rmqtt::context::ServerContext;
use rmqtt::{
    fitter::Fitter,
    hook::{Handler, HookResult, Parameter, ReturnType},
    session::Session,
    types::DisconnectInfo,
    types::{SessionSubs, TimestampMillis},
    utils::timestamp_millis,
};
use std::sync::Arc;
use std::time::Duration;

//session 存储
#[allow(dead_code)]
pub(super) struct StorageHandler {
    scx: ServerContext,
    storage_db: StorageDB,
    cfg: Arc<PluginConfig>,
    stored_session_infos: StoredSessionInfos,
    rebuild_tx: mpsc::Sender<RebuildChanType>,
}

impl StorageHandler {
    pub(super) fn new(
        scx: ServerContext,
        storage_db: StorageDB,
        cfg: Arc<PluginConfig>,
        stored_session_infos: StoredSessionInfos,
        rebuild_tx: mpsc::Sender<RebuildChanType>,
    ) -> Self {
        Self {
            scx,
            storage_db,
            cfg,
            stored_session_infos,
            rebuild_tx,
        }
    }

    //重建离线会话
    async fn rebuild_offline_sessions(&self, done_tx: oneshot::Sender<()>) {
        let mut offline_sessions_count = 0;

        for mut entry in self.stored_session_infos.iter_mut() {
            let (_, store_sessions) = entry.pair_mut();

            if let Some(store_session) = store_sessions.iter_mut().next() {
                let id = store_session.basic.id.clone();

                let listen_cfg = if let Some(listen_cfg) =
                    self.scx.listen_cfgs.get(&id.lid).map(|c| c.value().clone())
                {
                    listen_cfg
                } else {
                    log::warn!(
                        "{log_prefix} tcp listener config is not found, local addr is {:?}",
                        id.local_addr
                    );
                    continue;
                };

                //create fitter
                let fitter = self.scx.extends.fitter_mgr().await.create(
                    store_session.basic.conn_info.clone(),
                    id.clone(),
                    listen_cfg.clone(),
                );

                //check session expiry interval
                let session_expiry_interval = session_expiry_interval(
                    fitter.as_ref(),
                    store_session.disconnect_info.as_ref(),
                    store_session.last_time,
                )
                .await;

                if session_expiry_interval <= 0 {
                    log::debug!(
                        "{log_prefix} session is expiry id:{:?},  id_key: {:?} \n map_key: {:?},\n list_key: {:?}",
                        id,
                        store_session.id_key,
                        make_map_stored_key(store_session.id_key.as_ref()),
                        make_list_stored_key(store_session.id_key.as_ref())
                    );
                    let storage_db = self.storage_db.clone();
                    if let Err(e) = storage_db
                        .map_remove(make_map_stored_key(store_session.id_key.as_ref()))
                        .await
                    {
                        log::warn!("{log_prefix} {id:?} remove map error, {e:?}");
                    }
                    if let Err(e) = storage_db
                        .list_remove(make_list_stored_key(store_session.id_key.as_ref()))
                        .await
                    {
                        log::warn!("{log_prefix} {id:?} remove list error, {e:?}");
                    }
                    //session is expiry
                    continue;
                }
                offline_sessions_count += 1;

                if store_session.disconnect_info.is_none() {
                    store_session.disconnect_info =
                        Some(DisconnectInfo::new(store_session.last_time));
                }

                //最大飞行数
                let max_inflight = fitter.max_inflight();
                //最大队列长度
                let max_mqueue_len = fitter.max_mqueue_len();

                let subs = store_session
                    .subs
                    .take()
                    .map(SessionSubs::from)
                    .unwrap_or_else(SessionSubs::new);

                //创建会话
                let session = match Session::new(
                    id.clone(),
                    self.scx.clone(),
                    max_mqueue_len,
                    listen_cfg,
                    fitter,
                    None,
                    max_inflight,
                    store_session.basic.created_at,
                    store_session.basic.conn_info.clone(),
                    false,
                    false,
                    false,
                    store_session.basic.connected_at,
                    subs,
                    store_session.disconnect_info.take(),
                    None,
                )
                .await
                {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!(
                            "{log_prefix} rebuild session offline message error, create session error, {e:?}"
                        );
                        continue;
                    }
                };

                let deliver_queue = session.deliver_queue();
                for item in store_session.offline_messages.drain(..) {
                    if let Err((f, p)) = deliver_queue.push(item) {
                        log::warn!(
                            "{log_prefix} session offline message error, deliver queue is full, from: {f:?}, publish: {p:?}"
                        );
                    }
                }

                let out_inflight = session.out_inflight();
                for item in store_session.inflight_messages.drain(..) {
                    out_inflight.write().await.push_back(item);
                }

                if let Err(e) = self
                    .rebuild_tx
                    .clone()
                    .send(RebuildChanType::Session(
                        session,
                        Duration::from_millis(session_expiry_interval as u64),
                    ))
                    .await
                {
                    log::error!("{log_prefix} rebuild offline sessions error, {e:?}");
                }
            }
        }

        log::trace!("{log_prefix} offline_sessions_count: {offline_sessions_count}");
        let _ = self
            .rebuild_tx
            .clone()
            .send(RebuildChanType::Done(done_tx))
            .await;
    }
}

//重建离线会话
#[async_trait]
impl Handler for StorageHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        match param {
            Parameter::BeforeStartup => {
                let (done_tx, done_rx) = oneshot::channel::<()>();
                self.rebuild_offline_sessions(done_tx).await;
                let _ = done_rx.await;
            }
            _ => {
                log::error!("{log_prefix} unimplemented, {param:?}")
            }
        }
        (true, acc)
    }
}

#[inline]
async fn session_expiry_interval(
    fitter: &dyn Fitter,
    disconnect_info: Option<&DisconnectInfo>,
    last_time: TimestampMillis,
) -> TimestampMillis {
    let disconnected_at = disconnect_info
        .map(|d| d.disconnected_at)
        .unwrap_or_default();
    let disconnected_at = if disconnected_at <= 0 {
        last_time
    } else {
        disconnected_at
    };
    fitter
        .session_expiry_interval(disconnect_info.and_then(|d| d.mqtt_disconnect.as_ref()))
        .as_millis() as i64
        - (timestamp_millis() - disconnected_at)
}
