#![allow(unused_imports)]
use async_trait::async_trait;
use config::PluginConfig;
use redis::RedisStorage;
use rmqtt::{
    Result, context::ServerContext, hook::Register, macros::Plugin, message::MessageManager,
    plugin::Plugin, register,
};
use rmqtt_storage::init_db;
use serde_json::{self, json};
use sqlite::SqliteStorage;
use std::sync::Arc;
mod config;
mod entity;
mod redis;
mod sqlite;

register!(StoragePlugin::new);

const log_prefix: &str = "[plugin-message]";

#[derive(Plugin)]
struct StoragePlugin {
    scx: ServerContext,
    cfg: Arc<PluginConfig>,
    register: Box<dyn Register>,
    message_mgr: MessageMgr,
}

impl StoragePlugin {
    // async fn new<S: Into<String>>(scx: ServerContext, name: S) -> Result<Self> {
    //     let name = name.into();
    //     let cfg = scx.plugins.read_config_default::<PluginConfig>(&name)?;
    //     let cfg = Arc::new(cfg);

    //     let storage = SqliteStorage::new(cfg.clone()).await?;
    //     let message_mgr = MessageMgr { storage };
    //     let register = scx.extends.hook_mgr().register();
    //     Ok(Self {
    //         scx,
    //         cfg,
    //         register,
    //         message_mgr,
    //     })
    // }

    async fn new<S: Into<String>>(scx: ServerContext, name: S) -> Result<Self> {
        let name = name.into();
        let mut cfg = scx.plugins.read_config_default::<PluginConfig>(&name)?;

        let node_id = scx.node.id();
        cfg.storage.redis.prefix = cfg
            .storage
            .redis
            .prefix
            .replace("{node}", &format!("{node_id}"));

        let storage_db = match init_db(&cfg.storage).await {
            Err(e) => {
                log::error!("{log_prefix} init storage db error, {e:?}");
                return Err(e);
            }
            Ok(db) => db,
        };

        let cfg = Arc::new(cfg);

        //存储管理器
        let storage = RedisStorage::new(node_id, cfg.clone(), storage_db).await?;
        let message_mgr = MessageMgr { storage };

        let register = scx.extends.hook_mgr().register();
        Ok(Self {
            scx,
            cfg,
            register,
            message_mgr,
        })
    }
}

#[async_trait]
impl Plugin for StoragePlugin {
    #[inline]
    async fn init(&mut self) -> Result<()> {
        log::trace!("{log_prefix}  init");
        self.message_mgr.restore_topic_tree().await?;
        Ok(())
    }

    #[inline]
    async fn get_config(&self) -> Result<serde_json::Value> {
        Ok(self.cfg.to_json())
    }

    #[inline]
    async fn start(&mut self) -> Result<()> {
        let mgr: Box<dyn MessageManager> = Box::new(self.message_mgr.storage.clone());
        *self.scx.extends.message_mgr_mut().await = mgr;
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
        self.message_mgr.info().await
    }
}

struct MessageMgr {
    //storage: SqliteStorage,
    storage: RedisStorage,
}

impl MessageMgr {
    async fn restore_topic_tree(&self) -> Result<()> {
        self.storage.restore_topic_tree().await?;
        Ok(())
    }

    async fn info(&self) -> serde_json::Value {
        let now = std::time::Instant::now();
        let msg_queue_count = self
            .storage
            .msg_queue_count
            .load(std::sync::atomic::Ordering::Relaxed);

        let topic_nodes = self.storage.topic_tree.read().await.nodes_size();
        let receiveds = self.storage.topic_tree.read().await.values_size();
        let cost_time = format!("{:?}", now.elapsed());
        json!({
            "msg_queue_count": msg_queue_count,
            "message": {
                "topic_nodes": topic_nodes,
                "receiveds": receiveds,
                "cost_time":cost_time,
            },
        })
    }
}
