use async_trait::async_trait;
use config::PluginConfig;
use futures::{
    StreamExt,
    channel::{mpsc, oneshot},
};
use kv_storage::{List, Map, StorageDB, init_db};
use rmqtt::{
    Result,
    context::ServerContext,
    hook::{Register, Type},
    inflight::OutInflightMessage,
    macros::Plugin,
    plugin::Plugin,
    register,
    session::{Session, SessionState},
    types::DisconnectInfo,
    types::{ClientId, From, Publish, SessionSubMap, TimestampMillis},
};
use serde_json::{self, json};
use std::sync::Arc;
use std::time::Duration;
use storage::StorageSessionManager;
use storage::{BASIC, DISCONNECT_INFO, INFLIGHT_MESSAGES, LAST_TIME, SESSION_SUB_MAP};
use storage_info::{Basic, StoredKey, StoredSessionInfo, StoredSessionInfos};
const log_prefix: &str = "[plugin-session]";

mod config;
mod offline;
mod startup;
mod storage;
mod storage_info;
mod storage_keys;
use offline::*;
use startup::*;
use storage_keys::*;

enum RebuildChanType {
    Session(Session, Duration),
    Done(oneshot::Sender<()>),
}

type OfflineMessageOptionType = Option<(ClientId, From, Publish)>;

register!(StoragePlugin::new);

#[derive(Plugin)]
struct StoragePlugin {
    scx: ServerContext,
    cfg: Arc<PluginConfig>,
    storage_db: StorageDB,
    stored_session_infos: StoredSessionInfos,
    register: Box<dyn Register>,
    session_mgr: StorageSessionManager,
    rebuild_tx: mpsc::Sender<RebuildChanType>,
}

impl StoragePlugin {
    #[inline]
    async fn new<S: Into<String>>(scx: ServerContext, name: S) -> Result<Self> {
        let name = name.into();
        let cfg = scx.plugins.read_config_default::<PluginConfig>(&name)?;
        let storage_db = match init_db(&cfg.storage).await {
            Err(e) => {
                log::error!("{log_prefix} init storage db error, {e}");
                return Err(e);
            }
            Ok(db) => db,
        };

        let stored_session_infos = StoredSessionInfos::new();
        let register = scx.extends.hook_mgr().register();
        let session_mgr =
            StorageSessionManager::new(storage_db.clone(), stored_session_infos.clone());

        let cfg = Arc::new(cfg);
        let rebuild_tx = Self::start_local_runtime(scx.clone());
        Ok(Self {
            scx,
            cfg,
            storage_db,
            stored_session_infos,
            register,
            session_mgr,
            rebuild_tx,
        })
    }

    //2. 从数据库 加载离线会话信息
    async fn load_offline_session_infos(&mut self) -> Result<()> {
        log::trace!("{log_prefix}  load_offline_session_infos ...");

        let storage_db = self.storage_db.clone();
        let mut iter_storage_db = storage_db.clone();

        //订阅关系 ,将db 里的session 数据 放入stored_session_infos
        let mut map_iter = iter_storage_db.map_iter().await?;
        while let Some(m) = map_iter.next().await {
            match m {
                Ok(m) => {
                    let id_key = StoredKey::from(map_stored_key_to_id_bytes(m.name()).to_vec());

                    //获取session
                    let basic = match m.get::<_, Basic>(BASIC).await {
                        Err(e) => {
                            log::warn!(
                                "{log_prefix} {id_key:?} load offline session basic info error, {e:?}"
                            );
                            if let Err(e) = storage_db.map_remove(m.name()).await {
                                log::warn!(
                                    "{log_prefix} {id_key:?} remove offline session info error, {e:?}"
                                );
                            }
                            continue;
                        }
                        Ok(None) => {
                            log::warn!(
                                "{log_prefix}  {id_key:?} offline session basic info is None"
                            );
                            if let Err(e) = storage_db.map_remove(m.name()).await {
                                log::warn!(
                                    " {log_prefix}   {id_key:?} remove offline session info error, {e:?}"
                                );
                            }
                            continue;
                        }
                        Ok(Some(basic)) => basic,
                    };

                    let mut s_info = StoredSessionInfo::from(id_key.clone(), basic);

                    //最后时间
                    match m.get::<_, TimestampMillis>(LAST_TIME).await {
                        Ok(Some(last_time)) => {
                            s_info.set_last_time(last_time);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!(
                                "{log_prefix} {id_key:?} load offline session last time error, {e:?}"
                            );
                        }
                    }

                    //订阅关系
                    match m.get::<_, SessionSubMap>(SESSION_SUB_MAP).await {
                        Ok(Some(subs)) => {
                            s_info.set_subs(subs);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!(
                                "{log_prefix}   {id_key:?} load offline session subscription info error, {e:?}"
                            );
                        }
                    }

                    //连接信息
                    match m.get::<_, DisconnectInfo>(DISCONNECT_INFO).await {
                        Ok(Some(disc_info)) => {
                            log::trace!("{log_prefix}   disc_info: {disc_info:?}");
                            s_info.set_disconnect_info(disc_info);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!(
                                "{log_prefix}  {id_key:?} load offline session disconnect info error, {e:?}"
                            );
                        }
                    }

                    //飞行消息
                    match m.get::<_, Vec<OutInflightMessage>>(INFLIGHT_MESSAGES).await {
                        Ok(Some(inflights)) => {
                            log::debug!("{log_prefix}   inflights len: {:?}", inflights.len());
                            s_info.inflight_messages = inflights;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!(
                                "{log_prefix} {id_key:?} load offline session inflight messages error, {e:?}"
                            );
                        }
                    }

                    self.stored_session_infos.add(s_info);
                }
                Err(e) => {
                    log::warn!("{log_prefix} load offline session info error, {e:?}");
                }
            }
        }

        drop(map_iter);

        //离线消息 放入stored_session_infos
        let mut list_iter = iter_storage_db.list_iter().await?;
        while let Some(list) = list_iter.next().await {
            match list {
                Ok(list) => {
                    let id_key = StoredKey::from(list_stored_key_to_id_bytes(list.name()).to_vec());
                    log::trace!("{log_prefix}   list_stored_key, id_key: {id_key:?}");
                    match list.all::<OfflineMessageOptionType>().await {
                        Ok(offline_msgs) => {
                            log::debug!(
                                "{log_prefix}   {:?} offline_msgs len: {}",
                                id_key,
                                offline_msgs.len(),
                            );
                            let ok = self
                                .stored_session_infos
                                .set_offline_messages(id_key.clone(), offline_msgs);

                            if !ok {
                                if let Err(e) = storage_db.list_remove(list.name()).await {
                                    log::warn!("{id_key:?} remove offline messages error, {e:?}");
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "{log_prefix} {id_key:?} load offline messages error, {e:?}"
                            );
                            if let Err(e) = storage_db.list_remove(list.name()).await {
                                log::warn!(
                                    "{log_prefix} {id_key:?} remove offline messages error, {e:?}"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("{log_prefix} load offline messages error, {e:?}");
                }
            }
        }
        drop(list_iter);

        //移除过期的离线消息
        for removed_key in self.stored_session_infos.retain_latests() {
            storage_db
                .map_remove(make_map_stored_key(removed_key.as_ref()))
                .await?;
            storage_db
                .list_remove(make_list_stored_key(removed_key.as_ref()))
                .await?;
        }

        Ok(())
    }

    //1.接收 channel 发来的session信息 保存到rmqt 内核中
    fn start_local_runtime(scx: ServerContext) -> mpsc::Sender<RebuildChanType> {
        let (tx, mut rx) = futures::channel::mpsc::channel::<RebuildChanType>(1000);

        std::thread::spawn(move || {
            let local_rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect(&format!("{log_prefix} tokio runtime build failed"));
            let local_set = tokio::task::LocalSet::new();

            local_set.block_on(&local_rt, async {
                let exec = scx.get_exec("SESSION_REBUILD_EXEC");
                while let Some(msg) = rx.next().await {
                    match msg {
                        RebuildChanType::Session(session, session_expiry_interval)  => {

                            //重置session 信息到mqtt 内核
                            match SessionState::offline_restart(session.clone(), session_expiry_interval).await {
                                Err(e) => {
                                    log::warn!("{log_prefix} rebuild offline sessions error, {e:?}");
                                },
                                Ok(msg_tx) => {
                                    let mut session_entry =
                                        scx.extends.shared().await.entry(session.id.clone());

                                    let id = session_entry.id().clone();
                                    let task_fut = async move {
                                        if let Err(e) = session_entry.set(session, msg_tx).await {
                                            log::warn!("{log_prefix} {:?} rebuild offline sessions error, {:?}", session_entry.id(), e);
                                        }
                                    };
                                    if let Err(e) = exec.spawn(task_fut).await {
                                        log::warn!("{log_prefix} {:?} rebuild offline sessions error, {:?}", id, e.to_string());
                                    }

                                    let completed_count = exec.completed_count().await;
                                    if completed_count > 0 && completed_count % 5000 == 0 {
                                        log::debug!(
                                        "{log_prefix} {:?} rebuild offline sessions, completed_count: {}, active_count: {}, waiting_count: {}, rate: {:?}",
                                        id,
                                        exec.completed_count().await, exec.active_count(), exec.waiting_count(), exec.rate().await
                                    );
                                    }
                                }
                            }
                        },
                        RebuildChanType::Done(done_tx) => {
                            let _ = exec.flush().await;
                            let _ = done_tx.send(());
                            log::trace!(
                                "{log_prefix} rebuild offline sessions: {}, active_count: {}, waiting_count: {}, rate: {:?}",
                                exec.completed_count().await, exec.active_count(), exec.waiting_count(), exec.rate().await
                            );
                        }
                    }
                }
            });
            log::info!("{log_prefix}  Offline session rebuilding finished");
        });
        tx
    }
}

//
#[async_trait]
impl Plugin for StoragePlugin {
    #[inline]
    async fn init(&mut self) -> Result<()> {
        log::trace!("{log_prefix} init");
        self.register
            .add(
                Type::BeforeStartup,
                Box::new(StorageHandler::new(
                    self.scx.clone(),
                    self.storage_db.clone(),
                    self.cfg.clone(),
                    self.stored_session_infos.clone(),
                    self.rebuild_tx.clone(),
                )),
            )
            .await;
        self.register
            .add(
                Type::OfflineMessage,
                Box::new(OfflineMessageHandler::new(
                    self.cfg.clone(),
                    self.storage_db.clone(),
                )),
            )
            .await;
        self.register
            .add(
                Type::OfflineInflightMessages,
                Box::new(OfflineMessageHandler::new(
                    self.cfg.clone(),
                    self.storage_db.clone(),
                )),
            )
            .await;

        self.load_offline_session_infos().await?;
        Ok(())
    }

    #[inline]
    async fn get_config(&self) -> Result<serde_json::Value> {
        Ok(self.cfg.to_json())
    }

    #[inline]
    async fn start(&mut self) -> Result<()> {
        *self.scx.extends.session_mgr_mut().await = Box::new(self.session_mgr.clone());
        self.register.start().await;
        log::info!("{log_prefix} start ok");
        Ok(())
    }

    #[inline]
    async fn stop(&mut self) -> Result<bool> {
        Ok(false)
    }

    #[inline]
    async fn attrs(&self) -> serde_json::Value {
        async fn stats(storage_db: &StorageDB) -> (String, String, String, serde_json::Value) {
            let max_limit = 1000;
            let mut session_count = 0;
            let mut storage_db_map = storage_db.clone();
            {
                let now = std::time::Instant::now();
                let iter = storage_db_map.map_iter().await;
                if let Ok(mut iter) = iter {
                    while let Some(m) = iter.next().await {
                        if let Ok(m) = m {
                            log::debug!("map: {:?}", StoredKey::from(m.name().to_vec()));
                        }
                        session_count += 1;
                        if session_count >= max_limit {
                            break;
                        }
                    }
                }
                log::debug!("{log_prefix}  map_iter cost time: {:?}", now.elapsed());
            }

            let mut offline_session_count = 0;
            let mut offline_message_count = 0;
            let mut storage_db_list = storage_db.clone();
            {
                let now = std::time::Instant::now();
                let iter = storage_db_list.list_iter().await;
                if let Ok(mut iter) = iter {
                    while let Some(l) = iter.next().await {
                        if let Ok(mut l) = l {
                            log::debug!(
                                "{log_prefix}   list: {:?}",
                                StoredKey::from(l.name().to_vec())
                            );
                            if let Ok(mut l_iter) = l.iter::<OfflineMessageOptionType>().await {
                                while let Some(msg) = l_iter.next().await {
                                    if let Ok(Some(_)) = msg {
                                        offline_message_count += 1;
                                        if offline_message_count >= max_limit {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        offline_session_count += 1;
                        if offline_session_count >= max_limit {
                            break;
                        }
                    }
                }
                log::debug!("{log_prefix}   list_iter cost time: {:?}", now.elapsed());
            }
            let session_count = if session_count >= max_limit {
                format!("{session_count}+")
            } else {
                format!("{session_count}")
            };
            let offline_session_count = if offline_session_count >= max_limit {
                format!("{offline_session_count}+")
            } else {
                format!("{offline_session_count}")
            };
            let offline_message_count = if offline_message_count >= max_limit {
                format!("{offline_message_count}+")
            } else {
                format!("{offline_message_count}")
            };

            let storage_info = storage_db.info().await.unwrap_or_default();

            (
                session_count,
                offline_session_count,
                offline_message_count,
                storage_info,
            )
        }

        let (session_count, offline_session_count, offline_message_count, storage_info) =
            match tokio::time::timeout(Duration::from_secs(1), stats(&self.storage_db)).await {
                Ok((session_count, offline_session_count, offline_message_count, storage_info)) => {
                    (
                        session_count,
                        offline_session_count,
                        offline_message_count,
                        storage_info,
                    )
                }
                Err(_) => (
                    "Elapsed".into(),
                    "Elapsed".into(),
                    "Elapsed".into(),
                    serde_json::Value::Null,
                ),
            };

        json!({
            "session_count": session_count,
            "offline_session_count": offline_session_count,
            "offline_message_count": offline_message_count,
            "storage_info": storage_info
        })
    }
}
