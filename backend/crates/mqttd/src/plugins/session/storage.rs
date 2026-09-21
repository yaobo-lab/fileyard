use super::storage_info::{Basic, StoredSessionInfos};
use super::{OfflineMessageOptionType, log_prefix, make_list_stored_key, make_map_stored_key};
use async_trait::async_trait;
use kv_storage::{List, Map, StorageDB, StorageList, StorageMap};
use rmqtt::{
    Result,
    context::ServerContext,
    session::{DefaultSession, SessionLike, SessionManager},
    types::{
        ConnectInfo, ConnectInfoType, Disconnect, DisconnectInfo, FitterType, Id, IsPing,
        ListenerConfig, MessageQueueType, OutInflightType, Password, Reason, SessionSubMap,
        SessionSubs, SubscriptionOptions, Subscriptions, TimestampMillis, TopicFilter, UserName,
    },
    utils::timestamp_millis,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU8, Ordering};
use std::time::Duration;

pub(super) const LAST_TIME: &[u8] = b"1";
pub(super) const DISCONNECT_INFO: &[u8] = b"2";
pub(super) const SESSION_SUB_MAP: &[u8] = b"3";
pub(super) const BASIC: &[u8] = b"4";
pub(super) const INFLIGHT_MESSAGES: &[u8] = b"5";

#[derive(Clone)]
pub(super) struct StorageSessionManager {
    storage_db: StorageDB,
    _stored_session_infos: StoredSessionInfos,
}

impl StorageSessionManager {
    pub(super) fn new(
        storage_db: StorageDB,
        _stored_session_infos: StoredSessionInfos,
    ) -> StorageSessionManager {
        Self {
            storage_db,
            _stored_session_infos,
        }
    }
}

#[async_trait]
impl SessionManager for StorageSessionManager {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        id: Id,
        scx: ServerContext,
        listen_cfg: ListenerConfig,
        fitter: FitterType,
        subscriptions: SessionSubs,
        deliver_queue: MessageQueueType,
        outinflight: OutInflightType,
        conn_info: ConnectInfoType,
        created_at: TimestampMillis,
        connected_at: TimestampMillis,
        session_present: bool,
        superuser: bool,
        connected: bool,
        disconnect_info: Option<DisconnectInfo>,
        last_id: Option<Id>,
    ) -> Result<Arc<dyn SessionLike>> {
        let clean_start = conn_info.clean_start();
        let inner = DefaultSession::new(
            id,
            scx,
            listen_cfg,
            subscriptions,
            deliver_queue,
            outinflight,
            conn_info,
            created_at,
            connected_at,
            session_present,
            superuser,
            connected,
            disconnect_info,
        );

        //cur:          100@0.0.0.0:1883/10.0.3.36:57295/mqttx_b21cf15d22/aam_sub_mypc22/1758092536083
        //last_id: Some(100@0.0.0.0:1883/10.0.3.36:56228/mqttx_b21cf15d22/aam_sub_mypc22/1758092492867)
        log::debug!(
            "{log_prefix} session create cur id:{:?} last_id: {last_id:?}",
            inner.id()
        );

        if clean_start {
            Ok(Arc::new(inner))
        } else {
            let id_str = inner.id().to_string();
            //session 存储
            let session_info_store_map = self
                .storage_db
                .map(make_map_stored_key(id_str.as_str()), None)
                .await?;

            //离线消息存储
            let offline_msgs_store_list = self
                .storage_db
                .list(make_list_stored_key(id_str.as_str()), None)
                .await?;

            let s = Arc::new(StorageSession::new(
                inner,
                fitter,
                self.storage_db.clone(),
                session_info_store_map,
                offline_msgs_store_list,
            ));

            //清空 会话信息
            if connected {
                let s1 = s.clone();
                tokio::spawn(async move {
                    //保存 连接信息，订阅关系
                    if let Err(e) = s1.save_to_db().await {
                        log::error!("{log_prefix} save session info, err: {e:?}");
                    }

                    //上一个session_id
                    if let Some(last_id) = last_id {
                        let map = s1
                            .storage_db
                            .map(make_map_stored_key(last_id.to_string()), None)
                            .await;

                        let list = s1
                            .storage_db
                            .list(make_list_stored_key(last_id.to_string()), None)
                            .await;

                        if let Ok(map) = map {
                            if let Err(e) = map.clear().await {
                                log::warn!(
                                    "{log_prefix} session create remove mapdb last_id: {last_id:?}, err: {e:?}"
                                );
                            }
                        }

                        if let Ok(list) = list {
                            if let Err(e) = list.clear().await {
                                log::warn!(
                                    "{log_prefix} session create remove listdb, last_id: {last_id:?},err: {e:?}"
                                );
                            }
                        }
                    }
                });
            }
            Ok(s)
        }
    }
}

pub struct StorageSession {
    inner: DefaultSession,
    fitter: FitterType,
    //----------------------------------
    storage_db: StorageDB,
    session_info_store_map: StorageMap,
    offline_msg_store_list: StorageList,
    last_time: AtomicI64,
}

impl StorageSession {
    #[allow(clippy::too_many_arguments)]
    fn new(
        inner: DefaultSession,
        fitter: FitterType,
        storage_db: StorageDB,
        session_info_store_map: StorageMap,
        offline_msg_store_list: StorageList,
    ) -> Self {
        Self {
            inner,
            fitter,
            storage_db,
            session_info_store_map,
            offline_msg_store_list,
            last_time: AtomicI64::new(timestamp_millis()),
        }
    }

    #[inline]
    async fn update_last_time(&self, save_enable: bool) {
        let now = timestamp_millis();
        let old = self.last_time.swap(now, Ordering::SeqCst);
        if save_enable || (now - old) > (1000 * 60) {
            let id = self.id().clone();
            let session_info_store_map = self.session_info_store_map.clone();
            tokio::spawn(
                async move { Self::_update_last_time(&id, session_info_store_map, now).await },
            );
        }
    }

    #[inline]
    async fn _update_last_time(id: &Id, session_info_store_map: StorageMap, now: TimestampMillis) {
        if let Err(e) = session_info_store_map.insert(LAST_TIME, &now).await {
            log::warn!("{log_prefix} {id:?} save last time to db error, {e}");
        }
    }

    #[inline]
    pub(super) async fn delete_from_db(&self) {
        let id = self.id().clone();
        let session_info_store_map = self.session_info_store_map.clone();
        let offline_msg_store_list = self.offline_msg_store_list.clone();
        tokio::spawn(async move {
            Self::_delete_from_db(&id, &session_info_store_map, &offline_msg_store_list).await;
        });
    }

    #[inline]
    async fn _delete_from_db(
        id: &Id,
        session_info_store_map: &StorageMap,
        offline_msg_store_list: &StorageList,
    ) {
        if let Err(e) = session_info_store_map.clear().await {
            log::error!("{log_prefix} {id:?} remove session info error from db, {e}");
        }
        if let Err(e) = offline_msg_store_list.clear().await {
            log::error!("{log_prefix} {id:?} remove session offline messages error from db, {e}");
        }
    }

    #[inline]
    pub(super) async fn save_to_db(&self) -> Result<()> {
        self.update_last_time(true).await;
        self.save_basic_info().await;
        self.save_subscriptions().await;
        log::trace!("{log_prefix} {:?} save to db ...", self.id());
        Ok(())
    }

    #[inline]
    async fn save_basic_info(&self) {
        if let Err(e) = self._save_basic_info().await {
            log::error!("{log_prefix} save basic info error, {e:?}");
        }
    }

    #[inline]
    async fn _save_basic_info(&self) -> Result<()> {
        let basic = Basic {
            id: self.id().clone(),
            conn_info: self.connect_info().await?,
            created_at: self.created_at().await?,
            connected_at: self.connected_at().await?,
        };
        self.session_info_store_map.insert(BASIC, &basic).await?;
        Ok(())
    }

    #[inline]
    async fn save_subscriptions(&self) {
        let subs = self.inner.subscriptions.clone();
        let session_info_store_map = self.session_info_store_map.clone();
        tokio::spawn(async move {
            let subs = subs.read().await.clone();
            match tokio::time::timeout(
                Duration::from_secs(8),
                Self::_save_subscriptions(&session_info_store_map, &subs),
            )
            .await
            {
                Ok(()) => {}
                Err(e) => {
                    log::error!("{log_prefix} save subscriptions error, {e}");
                }
            }
        });
    }

    #[inline]
    async fn _save_subscriptions(session_info_store_map: &StorageMap, subs: &SessionSubMap) {
        if let Err(e) = session_info_store_map.insert(SESSION_SUB_MAP, subs).await {
            log::error!("{log_prefix} save subscriptions error, {e}");
        }
    }

    #[inline]
    async fn _subscriptions_clear(&self) -> Result<()> {
        self.inner.subscriptions.clear(self.context()).await;
        Ok(())
    }

    #[inline]
    async fn save_disconnect_info(&self) {
        let session_info_store_map = self.session_info_store_map.clone();
        let disconnect_info = self.inner.disconnect_info.read().await.clone();
        tokio::spawn(async move {
            if let Err(e) =
                Self::_save_disconnect_info(session_info_store_map, &disconnect_info).await
            {
                log::error!("{log_prefix} save disconnect info error, {e}");
            }
        });
    }

    #[inline]
    async fn _save_disconnect_info(
        session_info_store_map: StorageMap,
        disconnect_info: &DisconnectInfo,
    ) -> Result<()> {
        session_info_store_map
            .insert(DISCONNECT_INFO, disconnect_info)
            .await?;
        Ok(())
    }

    #[inline]
    async fn _disconnect_info(&self) -> DisconnectInfo {
        self.inner.disconnect_info.read().await.clone()
    }

    #[inline]
    async fn _set_map_stored_key_ttl(
        id: &Id,
        session_info_store_map: &StorageMap,
        session_expiry_interval_millis: i64,
    ) {
        match session_info_store_map
            .expire(session_expiry_interval_millis)
            .await
        {
            Err(e) => {
                log::warn!("{log_prefix} {id:?} set map ttl to db error, {e}");
            }
            Ok(res) => {
                log::trace!(
                    "{log_prefix} {:?} set map ttl to db ok, {:?}, {}",
                    id,
                    Duration::from_millis(session_expiry_interval_millis as u64),
                    res
                );
            }
        }
    }

    #[inline]
    async fn _set_list_stored_key_ttl(
        id: &Id,
        offline_msg_store_list: &StorageList,
        session_expiry_interval_millis: i64,
    ) {
        match offline_msg_store_list
            .expire(session_expiry_interval_millis)
            .await
        {
            Err(e) => {
                log::warn!("{log_prefix} {id:?} set list ttl to db error, {e}");
            }
            Ok(res) => {
                log::trace!(
                    "{log_prefix} {:?} set list ttl to db ok, {:?}, {}",
                    id,
                    Duration::from_millis(session_expiry_interval_millis as u64),
                    res
                );
            }
        }
    }

    #[inline]
    async fn _disconnected_set(
        id: &Id,
        offline_msg_store_list: StorageList,
        session_info_store_map: StorageMap,
        disconnect_info: DisconnectInfo,
        session_expiry_interval: i64,
    ) -> Result<()> {
        Self::_set_map_stored_key_ttl(id, &session_info_store_map, session_expiry_interval).await;
        match offline_msg_store_list
            .push::<OfflineMessageOptionType>(&None)
            .await
        {
            Ok(()) => {
                Self::_set_list_stored_key_ttl(
                    id,
                    &offline_msg_store_list,
                    session_expiry_interval,
                )
                .await;
            }
            Err(e) => {
                log::warn!("{log_prefix} {id:?} save offline messages error, {e}")
            }
        }

        Self::_save_disconnect_info(session_info_store_map, &disconnect_info).await?;

        Ok(())
    }
}

#[async_trait]
impl SessionLike for StorageSession {
    #[inline]
    fn id(&self) -> &Id {
        self.inner.id()
    }

    #[inline]
    fn context(&self) -> &ServerContext {
        self.inner.context()
    }

    #[inline]
    fn listen_cfg(&self) -> &ListenerConfig {
        self.inner.listen_cfg()
    }

    #[inline]
    fn deliver_queue(&self) -> &MessageQueueType {
        self.inner.deliver_queue()
    }

    #[inline]
    fn out_inflight(&self) -> &OutInflightType {
        self.inner.out_inflight()
    }

    #[inline]
    async fn subscriptions(&self) -> Result<SessionSubs> {
        self.inner.subscriptions().await
    }

    #[inline]
    async fn subscriptions_add(
        &self,
        topic_filter: TopicFilter,
        opts: SubscriptionOptions,
    ) -> Result<Option<SubscriptionOptions>> {
        let opts = self.inner.subscriptions_add(topic_filter, opts).await?;
        self.save_subscriptions().await;
        Ok(opts)
    }

    #[inline]
    async fn subscriptions_remove(
        &self,
        topic_filter: &str,
    ) -> Result<Option<(TopicFilter, SubscriptionOptions)>> {
        let sub = self.inner.subscriptions_remove(topic_filter).await?;
        self.save_subscriptions().await;
        Ok(sub)
    }

    #[inline]
    async fn subscriptions_drain(&self) -> Result<Subscriptions> {
        let subs = self.inner.subscriptions_drain().await?;
        self.save_subscriptions().await;
        Ok(subs)
    }

    #[inline]
    async fn subscriptions_extend(&self, other: Subscriptions) -> Result<()> {
        self.inner.subscriptions_extend(other).await?;
        self.save_subscriptions().await;
        Ok(())
    }

    #[inline]
    async fn created_at(&self) -> Result<TimestampMillis> {
        self.inner.created_at().await
    }

    #[inline]
    async fn session_present(&self) -> Result<bool> {
        self.inner.session_present().await
    }

    #[inline]
    async fn connect_info(&self) -> Result<Arc<ConnectInfo>> {
        self.inner.connect_info().await
    }

    #[inline]
    fn username(&self) -> Option<&UserName> {
        self.inner.username()
    }

    #[inline]
    fn password(&self) -> Option<&Password> {
        self.inner.password()
    }

    #[inline]
    async fn protocol(&self) -> Result<u8> {
        self.inner.protocol().await
    }

    #[inline]
    async fn superuser(&self) -> Result<bool> {
        self.inner.superuser().await
    }

    #[inline]
    async fn connected(&self) -> Result<bool> {
        self.inner.connected().await
    }

    #[inline]
    async fn connected_at(&self) -> Result<TimestampMillis> {
        self.inner.connected_at().await
    }

    #[inline]
    async fn disconnected_at(&self) -> Result<TimestampMillis> {
        self.inner.disconnected_at().await
    }
    #[inline]
    async fn disconnected_reasons(&self) -> Result<Vec<Reason>> {
        self.inner.disconnected_reasons().await
    }
    #[inline]
    async fn disconnected_reason(&self) -> Result<Reason> {
        self.inner.disconnected_reason().await
    }
    #[inline]
    async fn disconnected_reason_has(&self) -> bool {
        self.inner.disconnected_reason_has().await
    }
    #[inline]
    async fn disconnected_reason_add(&self, r: Reason) -> Result<()> {
        self.inner.disconnected_reason_add(r).await?;
        self.save_disconnect_info().await;
        log::trace!("{log_prefix} {:?} disconnected_reason_add ... ", self.id());
        Ok(())
    }
    #[inline]
    async fn disconnected_reason_take(&self) -> Result<Reason> {
        let r = self.inner.disconnected_reason_take().await;
        log::debug!("{log_prefix} {:?} disconnected_reason_take ... ", self.id());
        r
    }
    #[inline]
    async fn disconnect(&self) -> Result<Option<Disconnect>> {
        self.inner.disconnect().await
    }
    #[inline]
    async fn disconnected_set(&self, d: Option<Disconnect>, reason: Option<Reason>) -> Result<()> {
        let session_expiry_interval =
            self.fitter.session_expiry_interval(d.as_ref()).as_millis() as i64;
        log::trace!(
            "{:?} disconnected_set session_expiry_interval: {:?}",
            self.id(),
            session_expiry_interval
        );

        self.inner.disconnected_set(d, reason).await?;

        let id = self.id().clone();
        let offline_msg_store_list = self.offline_msg_store_list.clone();
        let session_info_store_map = self.session_info_store_map.clone();
        let disconnect_info = self._disconnect_info().await;

        tokio::spawn(async move {
            if let Err(e) = Self::_disconnected_set(
                &id,
                offline_msg_store_list,
                session_info_store_map,
                disconnect_info,
                session_expiry_interval,
            )
            .await
            {
                log::error!("{id:?} disconnected set error, {e}")
            }
        });

        log::trace!("{:?} disconnected_set ... ", self.id());
        Ok(())
    }

    #[inline]
    async fn on_drop(&self) -> Result<()> {
        log::trace!("{log_prefix} {:?} StorageSession on_drop ...", self.id());
        if let Err(e) = self._subscriptions_clear().await {
            log::error!(
                "{log_prefix} {:?} subscriptions clear error, {}",
                self.id(),
                e
            );
        }
        self.delete_from_db().await;
        Ok(())
    }

    #[inline]
    async fn keepalive(&self, ping: IsPing) {
        if ping {
            self.update_last_time(true).await;
        } else {
            self.update_last_time(false).await;
        }
    }
}

// const SESSION_PRESENT: u8 = 0b00000001;
// const SUPERUSER: u8 = 0b00000010;
// const CONNECTED: u8 = 0b00000100;
// const REMOVE_AND_SAVE: u8 = 0b00001000;

//const EMPTY: u8 = u8::MIN;
//const ALL: u8 = u8::MAX;

pub trait AtomicFlags {
    type T;
    #[allow(dead_code)]
    fn empty() -> Self;
    #[allow(dead_code)]
    fn all() -> Self;
    #[allow(dead_code)]
    fn get(&self) -> Self::T;
    #[allow(dead_code)]
    fn insert(&self, other: Self::T);
    #[allow(dead_code)]
    fn contains(&self, other: Self::T) -> bool;
    #[allow(dead_code)]
    fn remove(&self, other: Self::T);
    #[allow(dead_code)]
    fn equal_exchange(
        &self,
        current: Self::T,
        new: Self::T,
        mask: Self::T,
    ) -> std::result::Result<Self::T, Self::T>;
    #[allow(dead_code)]
    fn difference(&self, other: Self::T) -> Self::T;
}

impl AtomicFlags for AtomicU8 {
    type T = u8;

    #[inline]
    fn empty() -> Self {
        AtomicU8::new(0)
    }

    #[inline]
    fn all() -> Self {
        AtomicU8::new(0xff)
    }

    #[inline]
    fn get(&self) -> Self::T {
        self.load(Ordering::SeqCst)
    }

    #[inline]
    fn insert(&self, other: Self::T) {
        self.fetch_or(other, Ordering::SeqCst);
    }

    #[inline]
    fn contains(&self, other: Self::T) -> bool {
        self.get() & other == other
    }

    #[inline]
    fn remove(&self, other: Self::T) {
        self.store(self.difference(other), Ordering::SeqCst);
    }

    #[inline]
    fn equal_exchange(
        &self,
        current: Self::T,
        new: Self::T,
        mask: Self::T,
    ) -> std::result::Result<Self::T, Self::T> {
        self.fetch_update(Ordering::SeqCst, Ordering::SeqCst, move |v| {
            let flags = current & mask;
            if (v & mask) == flags {
                Some((v & !flags) | (new & mask))
            } else {
                None
            }
        })
    }

    #[inline]
    fn difference(&self, other: Self::T) -> Self::T {
        self.get() & !other
    }
}
