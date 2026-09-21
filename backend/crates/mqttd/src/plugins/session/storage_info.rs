use super::OfflineMessageOptionType;
use super::log_prefix;

use bytes::Bytes;
use rmqtt::{
    inflight::OutInflightMessage,
    types::{
        ClientId, ConnectInfo, DashMap, DisconnectInfo, From, Id, Publish, SessionSubMap,
        TimestampMillis,
    },
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::Deref;
use std::sync::Arc;

pub(super) type StoredKey = Bytes;

pub(super) struct StoredSessionInfo {
    pub id_key: StoredKey,
    pub basic: Basic,
    pub subs: Option<SessionSubMap>,
    pub disconnect_info: Option<DisconnectInfo>,
    pub offline_messages: Vec<(From, Publish)>,
    pub inflight_messages: Vec<OutInflightMessage>,
    pub last_time: TimestampMillis,
}

impl StoredSessionInfo {
    #[inline]
    pub fn from(id_key: StoredKey, basic: Basic) -> Self {
        let last_time = basic.connected_at;
        Self {
            id_key,
            basic,
            subs: None,
            disconnect_info: None,
            offline_messages: Vec::new(),
            inflight_messages: Vec::new(),
            last_time,
        }
    }

    #[inline]
    #[allow(clippy::mutable_key_type)]
    pub fn set_subs(&mut self, subs: SessionSubMap) {
        self.subs.replace(subs);
    }

    #[inline]
    pub fn set_disconnect_info(&mut self, disconnect_info: DisconnectInfo) {
        self.disconnect_info.replace(disconnect_info);
    }

    #[inline]
    pub fn set_last_time(&mut self, last_time: TimestampMillis) {
        if self.last_time < last_time {
            self.last_time = last_time;
        }
    }
}

#[derive(Clone)]
pub(super) struct StoredSessionInfos(Arc<DashMap<ClientId, Vec<StoredSessionInfo>>>);

impl Deref for StoredSessionInfos {
    type Target = Arc<DashMap<ClientId, Vec<StoredSessionInfo>>>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StoredSessionInfos {
    #[inline]
    pub fn new() -> Self {
        Self(Arc::new(DashMap::default()))
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn add(&mut self, stored: StoredSessionInfo) {
        self.0
            .entry(stored.basic.id.client_id.clone())
            .or_default()
            .push(stored);
    }

    #[inline]
    pub fn set_offline_messages(
        &mut self,
        id_key: StoredKey,
        offline_messages: Vec<OfflineMessageOptionType>,
    ) -> bool {
        let mut exist = false;
        log::trace!("{log_prefix} set_offline_messages id_key: {id_key:?}");
        for (cid, f, p) in offline_messages.into_iter().flatten() {
            if let Some(mut entry) = self.0.get_mut(&cid) {
                let storeds = entry.value_mut();
                for stored in storeds {
                    if stored.id_key == id_key {
                        exist = true;
                        stored.offline_messages.push((f, p));
                        break;
                    }
                }
            }
        }
        exist
    }

    //保留最后一个
    #[inline]
    pub fn retain_latests(&mut self) -> Vec<StoredKey> {
        let mut removeds = Vec::new();
        for mut entry in self.0.iter_mut() {
            let storeds = entry.value_mut();

            if storeds.len() > 1 {
                if let Some(mut latest) = storeds.pop() {
                    while let Some(item) = storeds.pop() {
                        if item.last_time > latest.last_time {
                            removeds.push(latest.id_key);
                            latest = item;
                        } else {
                            removeds.push(item.id_key);
                        }
                    }
                    storeds.push(latest);
                }
            }
        }
        removeds
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct Basic {
    pub id: Id,
    #[serde(
        serialize_with = "Basic::serialize_conn_info",
        deserialize_with = "Basic::deserialize_conn_info"
    )]
    pub conn_info: Arc<ConnectInfo>,
    pub created_at: TimestampMillis,
    pub connected_at: TimestampMillis,
}

impl Basic {
    #[inline]
    fn serialize_conn_info<S>(
        conn_info: &Arc<ConnectInfo>,
        s: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        conn_info.as_ref().serialize(s)
    }

    #[inline]
    pub fn deserialize_conn_info<'de, D>(
        deserializer: D,
    ) -> std::result::Result<Arc<ConnectInfo>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Arc::new(ConnectInfo::deserialize(deserializer)?))
    }
}
