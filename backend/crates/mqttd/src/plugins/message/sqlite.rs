#![allow(dead_code)]
#![allow(unused_imports)]
use super::config::PluginConfig;
use crate::message::log_prefix;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use futures::{
    channel::mpsc,
    {SinkExt, StreamExt},
};
use futures_time::{self, future::FutureExt};
use r2d2::{HandleError, Pool};
use r2d2_sqlite::SqliteConnectionManager;
use rmqtt::{
    message::MessageManager,
    retain::RetainTree,
    topic::Level,
    types::{
        ClientId, From, MsgID, NodeId, Publish, SharedGroup, StoredMessage, TimestampMillis, Topic,
        TopicFilter,
    },
    utils::timestamp_millis,
};
use rmqtt_storage::{StorageDB, Map, StorageMap};

use rusqlite::{OptionalExtension, params};
use rust_box::task_exec_queue::{Builder, SpawnExt, TaskExecQueue};
use std::collections::BTreeSet;
use std::convert::From as _;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::{runtime::Handle, sync::RwLock, task::spawn_blocking, time::sleep};

const FORWARDED_PREFIX: &[u8] = b"fwd_";

#[derive(Debug)]
struct CustomErrorHandler;

impl HandleError<rusqlite::Error> for CustomErrorHandler {
    fn handle_error(&self, err: rusqlite::Error) {
        log::error!("sqlite pool error: {}", err);
    }
}

fn get_pool() -> &'static Pool<SqliteConnectionManager> {
    static INSTANCE: OnceLock<Pool<SqliteConnectionManager>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let manager = SqliteConnectionManager::file("./message.db");
        Pool::builder()
            .error_handler(Box::new(CustomErrorHandler {}))
            .connection_timeout(Duration::from_secs(3)) //连接超时3秒
            .max_size(3) // 最多5个连接
            .idle_timeout(Some(Duration::from_secs(60 * 60))) //空闲连接
            .build(manager)
            .unwrap_or_else(|err| {
                panic!("sqlite pool create err:{}", err);
            })
    })
}
//建表语句
fn table_sql() -> &'static str {
    "CREATE TABLE IF NOT EXISTS message (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        topic TEXT NOT NULL,
        msg blob NOT NULL,
        qos integer NOT NULL,
        expiry_time_at integer NOT NULL
    );"
}

pub(super) fn setup() -> Result<()> {
    let conn = get_pool().get()?;
    //设备分类表
    conn.execute_batch(table_sql())?;
    log::info!("{log_prefix} sqlite setup ok");
    Ok(())
}

// 订阅该主题的客户端
type SubClientIds = Vec<(ClientId, Option<(TopicFilter, SharedGroup)>)>;

//From 来源
//Publish 发布
//Duration 有效期
//MsgID 消息ID
type Msg = ((From, Publish, Duration, MsgID), Option<SubClientIds>);

#[derive(Clone)]
pub(super) struct StorageMessageManager {
    inner: Arc<StorageMessageManagerInner>,
    pub(super) exec: TaskExecQueue,
}

impl StorageMessageManager {
    #[inline]
    pub(super) async fn new(
        _node_id: NodeId,
        cfg: Arc<PluginConfig>,
    ) -> Result<StorageMessageManager> {
        let messages_received_max = StorageMessageManagerInner::new_messages_counter()?;
        let id_generater = StorageMessageManagerInner::new_msg_id_generater()?;

        let queue_max = 3000;
        let (exec, task_runner) = Builder::default().workers(100).queue_max(queue_max).build();

        tokio::spawn(async move {
            task_runner.await;
        });

        let (msg_tx, msg_rx) = mpsc::channel::<Msg>(queue_max);
        let msg_queue_count = Arc::new(AtomicIsize::new(0));

        let inner = Arc::new(StorageMessageManagerInner {
            messages_received_max,
            msg_tx,
            msg_queue_count,
            id_generater,
        });
        Ok(Self { inner, exec }.serve(cfg, msg_rx))
    }

    fn serve(self, _cfg: Arc<PluginConfig>, mut msg_rx: mpsc::Receiver<Msg>) -> Self {
        let msg_mgr = self.clone();
        let msg_queue_count1 = self.msg_queue_count.clone();

        // 消息 存储 处理 每 50 条消息存一次
        tokio::spawn(async move {
            let msg_fwds_count = Arc::new(AtomicIsize::new(0));
            let msg_insert_count = 50;
            let mut merger_msgs = Vec::new();

            while let Some(msg) = msg_rx.next().await {
                merger_msgs.push(msg);

                //批量写 50 条数据
                while merger_msgs.len() < msg_insert_count {
                    match tokio::time::timeout(Duration::from_millis(0), msg_rx.next()).await {
                        Ok(Some(msg)) => {
                            merger_msgs.push(msg);
                        }
                        _ => break,
                    }
                }

                log::debug!("{log_prefix} msgs save len: {}", merger_msgs.len());

                let msgs = std::mem::take(&mut merger_msgs);

                //更新队列缓存
                msg_queue_count1.fetch_sub(msgs.len() as isize, Ordering::Relaxed);
                msg_fwds_count.fetch_add(1, Ordering::SeqCst);

                while msg_fwds_count.load(Ordering::SeqCst) > msg_insert_count as isize {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }

                let msg_fwds_count1 = msg_fwds_count.clone();
                let msg_mgr = msg_mgr.clone();
                tokio::spawn(async move {
                    if let Err(e) = msg_mgr._batch_msg_insert(msgs).await {
                        log::warn!("{log_prefix} _batch_msg_insert err: {e:?}");
                    }
                    msg_fwds_count1.fetch_sub(1, Ordering::SeqCst);
                });
            }
            log::error!("{log_prefix} recv failed because receiver is gone");
        });

        //清除过期消息
        tokio::spawn(async move {
            sleep(Duration::from_secs(20)).await;
            loop {
                sleep(Duration::from_secs(30)).await;
            }
        });
        self
    }
}

impl Deref for StorageMessageManager {
    type Target = StorageMessageManagerInner;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

pub(super) struct StorageMessageManagerInner {
    messages_received_max: AtomicIsize,
    msg_tx: mpsc::Sender<Msg>,
    pub(super) msg_queue_count: Arc<AtomicIsize>,
    id_generater: AtomicUsize,
}

//mesage id 处理
impl StorageMessageManagerInner {
    //存储取消息最大ID
    #[inline]
    async fn save_msg_id(&self) -> Result<()> {
        // let curr_msg_id = self.id_generater.load(Ordering::SeqCst);
        // self.storage_db.insert("id_generater", &curr_msg_id).await?;
        Ok(())
    }

    //初始化消息ID 值
    #[inline]
    fn new_msg_id_generater() -> Result<AtomicUsize> {
        let conn = get_pool().get()?;
        let mut stmt = conn.prepare("SELECT MAX(id) as id from message")?;
        let v: Option<usize> = stmt.query_row(params![], |row| row.get(0)?).optional()?;
        if let Some(curr_msg_id) = v {
            Ok(AtomicUsize::new(curr_msg_id))
        } else {
            Ok(AtomicUsize::new(1))
        }
    }

    //
    #[inline]
    fn _next_msg_id(&self) -> usize {
        self.id_generater.fetch_add(1, Ordering::SeqCst)
    }

    #[inline]
    async fn messages_counter_add(&self, _v: isize) -> Result<()> {
        // self.storage_db
        //     .counter_incr("messages_received_max", vals)
        //     .await?;
        Ok(())
    }

    #[inline]
    fn new_messages_counter() -> Result<AtomicIsize> {
        // let max = storage_db
        //     .counter_get("messages_received_max")
        //     .await?
        //     .unwrap_or_default();
        //   Ok(AtomicIsize::new(max))
        Ok(AtomicIsize::new(100))
    }

    #[inline]
    fn messages_received_count_add(&self, len: isize) {
        self.messages_received_max.fetch_add(len, Ordering::SeqCst);
    }
}

//message curd 处理
impl StorageMessageManagerInner {
    //外键
    #[inline]
    fn make_client_key(client_id: &str) -> Vec<u8> {
        [FORWARDED_PREFIX, client_id.as_bytes()].concat()
    }

    //批量插入数据
    #[inline]
    async fn _batch_msg_insert(&self, msgs: Vec<Msg>) -> Result<()> {
        let mut count = 0;
        for ((from, publish, expiry_interval, msg_id), sub_client_ids) in msgs {
            //
            let mut topic = match Topic::from_str(&publish.topic) {
                Err(e) => {
                    log::warn!("{log_prefix} topic::from_str error, {e:?}");
                    continue;
                }
                Ok(topic) => topic,
            };

            //
            let expiry_time_at = timestamp_millis() + expiry_interval.as_millis() as i64;
            let smsg = StoredMessage {
                msg_id,
                from,
                publish,
                expiry_time_at,
            };

            //
            let msg_key = msg_id.to_be_bytes();

            //存储smsg
            if let Err(e) = msg_store
                .insert(DATA, &smsg)
                .timeout(futures_time::time::Duration::from_millis(5000))
                .await
                .map_err(|_e| anyhow!("{log_prefix} map.insert timeout"))?
            {
                log::warn!("{log_prefix} store to db error, {e:?}, message: {smsg:?}");
                continue;
            }

            // insert client_id
            if let Some(client_id) = sub_client_ids {
                self._sub_client_insert(&msg_store, client_id).await?;
            }

            //topic
            topic.push(Level::Normal(msg_id.to_string()));
            self.topic_tree.write().await.insert(&topic, msg_id);
            self.topic_list
                .write()
                .await
                .insert((expiry_time_at, topic));

            count += 1;
        }

        //
        self.messages_received_count_add(count);
        if let Err(e) = self
            .messages_counter_add(count)
            .timeout(futures_time::time::Duration::from_millis(5000))
            .await
            .map_err(|_e| anyhow!("{log_prefix} storage_messages_counter_add timeout"))?
        {
            log::warn!("{log_prefix} messages_received_counter add error, {e:?}");
        }

        Ok(())
    }

    //插入要转的内容
    #[inline]
    async fn _sub_client_insert(
        &self,
        msg_store: &StorageMap,
        client_id_subs: SubClientIds,
    ) -> Result<()> {
        for (client_id, opts) in client_id_subs {
            //
            if let Err(e) = msg_store
                .insert(Self::make_client_key(&client_id), &opts)
                .timeout(futures_time::time::Duration::from_millis(5000))
                .await
                .map_err(|_e| anyhow!("_sub_client_insert insert timeout"))?
            {
                log::warn!(
                    "{log_prefix} _sub_client_insert error, client_id: {:?}, msg_map name: {:?}, error: {:?}",
                    client_id,
                    String::from_utf8_lossy(msg_store.name()),
                    e,
                );
            }
        }
        Ok(())
    }

    //客户端上线，获取MQTT 缓存的消息
    #[inline]
    async fn _get(
        &self,
        client_id: &str,
        topic_str: &str,
        group: Option<&SharedGroup>,
    ) -> Result<Vec<(MsgID, From, Publish)>> {
        let inner = self;

        let mut topic = Topic::from_str(topic_str).map_err(|e| anyhow!(format!("{:?}", e)))?;

        if !topic
            .levels()
            .last()
            .map(|l| matches!(l, Level::MultiWildcard))
            .unwrap_or_default()
        {
            topic.push(Level::SingleWildcard);
        }

        let matcheds: Vec<_> = inner
            .topic_tree
            .read()
            .await
            .matches(&topic)
            .into_iter()
            .map(|(_t, msg_id)| msg_id)
            .collect();

        let messages = futures::future::join_all(matcheds.into_iter().map(|msg_id| async move {
            let msg_key = msg_id.to_be_bytes();

            let msg_store_map = self.storage_db.map(msg_key, None).await;

            match msg_store_map {
                Ok(mut store) => {
                    // Get the message
                    let client_subs = self
                        ._client_haveing_sub(&mut store, client_id, topic_str, group)
                        .await
                        .unwrap_or_default();

                    if client_subs {
                        None
                    } else if let Ok(Some(msg)) = inner._get_message(&store).await {
                        log::debug!(
                            "{log_prefix} _get msg: {:?}, msg.is_expiry(): {}",
                            msg,
                            msg.is_expiry()
                        );
                        if msg.is_expiry() {
                            None
                        } else {
                            let opts = group.map(|g| (TopicFilter::from(topic_str), g.clone()));
                            if let Err(e) =
                                store.insert(Self::make_client_key(client_id), &opts).await
                            {
                                log::warn!("{log_prefix} _get::insert error, {e:?}");
                            }
                            Some((msg_id, msg.from, msg.publish))
                        }
                    } else {
                        None
                    }
                }
                Err(e) => {
                    log::warn!("{log_prefix} _get new map error, {e:?}");
                    None
                }
            }
        }))
        .await
        .into_iter()
        .flatten()
        .collect();

        Ok(messages)
    }

    //检查客户端是否有订阅主题
    #[inline]
    async fn _client_haveing_sub(
        &self,
        msg_map: &mut StorageMap,
        client_id: &str,
        topic_filter: &str,
        group: Option<&SharedGroup>,
    ) -> Result<bool> {
        let key = Self::make_client_key(client_id);
        if msg_map.contains_key(key).await? {
            log::debug!("{log_prefix}  contains_key client_id: {client_id:?}");
            return Ok(true);
        }
        if let Some(group) = group {
            let mut iter = msg_map
                .prefix_iter::<_, Option<(TopicFilter, SharedGroup)>>(FORWARDED_PREFIX)
                .await?;
            while let Some(item) = iter.next().await {
                log::debug!("{log_prefix}   item: {item:?}");
                match item {
                    Ok((_, Some((tf, g)))) => {
                        if g == group && tf == topic_filter {
                            return Ok(true);
                        }
                    }
                    Ok((_, None)) => {}
                    Err(e) => {
                        log::warn!("{log_prefix}  traverse forwardeds error, {e:?}");
                        return Err(anyhow!(e));
                    }
                }
            }
        }
        Ok(false)
    }

    #[inline]
    async fn _get_message(&self, msg_map: &StorageMap) -> Result<Option<StoredMessage>> {
        msg_map.get::<_, StoredMessage>(DATA).await
    }
}

//实现消息存储接口
#[async_trait]
impl MessageManager for StorageMessageManager {
    //获取下一个消息ID
    #[inline]
    fn next_msg_id(&self) -> MsgID {
        self._next_msg_id()
    }

    //存储消息
    #[inline]
    async fn store(
        &self,
        msg_id: MsgID,
        from: From,
        p: Publish,
        expiry_interval: Duration,
        sub_client_ids: Option<SubClientIds>,
    ) -> Result<()> {
        log::debug!(
            "{log_prefix} store msg_id: {}, payload:{:?}, expiry_interval:{}s",
            msg_id,
            p.payload,
            expiry_interval.as_secs()
        );

        let res = self
            .msg_tx
            .clone()
            .send(((from, p, expiry_interval, msg_id), sub_client_ids))
            .timeout(futures_time::time::Duration::from_millis(3500))
            .await
            .map_err(|e| anyhow!(e));

        match res {
            Ok(Ok(())) => {
                self.msg_queue_count.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Ok(Err(e)) => {
                log::warn!("{log_prefix} store error, {e:?}");
                Err(anyhow!(e))
            }
            Err(e) => {
                log::warn!("{log_prefix} store timeout, {e:?}");
                Err(anyhow!(e))
            }
        }
    }

    //获取客户端订阅的保留消息
    #[inline]
    async fn get(
        &self,
        client_id: &str,
        topic: &str,
        group: Option<&SharedGroup>,
    ) -> Result<Vec<(MsgID, From, Publish)>> {
        log::info!("{log_prefix} get client_id: {client_id}, topic:{topic},group:{group:?}");

        let now = std::time::Instant::now();
        let inner = self.inner.clone();
        let client_id = ClientId::from(client_id);
        let topic_filter = TopicFilter::from(topic);
        let group = group.cloned();

        //获取数据库匹配的订阅消息
        let matcheds = async move { inner._get(&client_id, &topic_filter, group.as_ref()).await }
            .spawn(&self.exec)
            .result()
            .timeout(futures_time::time::Duration::from_millis(3000))
            .await;

        let messages = match matcheds {
            Ok(Ok(Ok(res))) => res,
            Ok(Ok(Err(e))) => {
                log::error!(
                    "{log_prefix} [MessageManager] get error, {:?}",
                    e.to_string()
                );
                return Err(e);
            }
            Ok(Err(e)) => {
                log::error!(
                    "{log_prefix} [MessageManager] get error, {:?}",
                    e.to_string()
                );
                return Err(anyhow!(e.to_string()));
            }
            Err(e) => {
                log::warn!("{log_prefix} [MessageManager] get timeout, {e:?}");
                vec![]
            }
        };

        //耗时
        if now.elapsed().as_millis() > 900 {
            log::debug!(
                "{log_prefix} [MessageManager] get cost time: {:?}, waiting_count: {:?}",
                now.elapsed(),
                self.exec.waiting_count()
            );
        }

        Ok(messages)
    }

    //
    #[inline]
    async fn count(&self) -> isize {
        self.topic_list.read().await.len() as isize
    }

    #[inline]
    async fn max(&self) -> isize {
        self.messages_received_max.load(Ordering::SeqCst)
    }

    //是否启用
    #[inline]
    fn enable(&self) -> bool {
        true
    }

    // 在“获取”操作期间是否需要合并来自各个节点的数据
    #[inline]
    fn should_merge_on_get(&self) -> bool {
        false
    }
}
