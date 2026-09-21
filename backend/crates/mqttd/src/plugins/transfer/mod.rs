use super::PublishParams;
use async_trait::async_trait;
use config::PluginConfig;
use rmqtt::{
    Result,
    context::ServerContext,
    hook::{Register, Type},
    macros::Plugin,
    plugin::Plugin,
    register,
    utils::Counter,
};
use serde_json::{self, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use tokio::{
    self,
    sync::RwLock,
    sync::mpsc::{Receiver, Sender, channel},
};
mod config;
mod handler;
use handler::*;

register!(TransferPlugin::new);

const log_prefix: &str = "[plugin-transfer]";

#[derive(Plugin)]
struct TransferPlugin {
    register: Box<dyn Register>,
    cfg: Arc<PluginConfig>,
    tx: Arc<RwLock<Sender<PublishParams>>>,
    chan_queue_count: Arc<AtomicIsize>,
    fails: Arc<Counter>,
}

impl TransferPlugin {
    #[inline]
    async fn new<S: Into<String>>(scx: ServerContext, name: S) -> Result<Self> {
        let name = name.into();
        let cfg = Arc::new(Self::load_config(&scx, &name)?);
        let chan_queue_count = Arc::new(AtomicIsize::new(0));
        let fails = Arc::new(Counter::new());

        let tx = Self::start(
            scx.clone(),
            cfg.clone(),
            chan_queue_count.clone(),
            fails.clone(),
        )
        .await;

        let tx = Arc::new(RwLock::new(tx));
        let register = scx.extends.hook_mgr().register();
        Ok(Self {
            register,
            cfg,
            chan_queue_count,
            tx,
            fails,
        })
    }

    async fn start(
        scx: ServerContext,
        cfg: Arc<PluginConfig>,
        chan_queue_count: Arc<AtomicIsize>,
        fails: Arc<Counter>,
    ) -> Sender<PublishParams> {
        // 消息队列的数量
        let (tx, mut rx): (Sender<PublishParams>, Receiver<PublishParams>) =
            channel(cfg.queue_capacity);

        tokio::spawn(async move {
            log::debug!("{log_prefix}: start async worker.");
            let runner = async {
                loop {
                    match rx.recv().await {
                        Some(msg) => {
                            //计数器
                            chan_queue_count.fetch_sub(1, Ordering::SeqCst);
                            log::trace!("transfter message to {:?}", msg.topic);
                            if let Err(e) = super::re_publish_message(msg, &scx, None).await {
                                log::error!("{log_prefix}: message publish err: {e:?}");
                            }
                        }
                        None => {
                            fails.current_inc();
                            log::warn!("{log_prefix}: message channel is closed");
                            break;
                        }
                    }
                }
            };

            runner.await;
            log::info!("{log_prefix}: exit web-hook async worker.");
        });

        tx
    }

    #[inline]
    fn load_config(scx: &ServerContext, name: &str) -> Result<PluginConfig> {
        let cfg = scx
            .plugins
            .read_config_with::<PluginConfig>(name, &["urls"])?;
        Ok(cfg)
    }
}

#[async_trait]
impl Plugin for TransferPlugin {
    #[inline]
    async fn init(&mut self) -> Result<()> {
        log::trace!("{log_prefix} init");
        let tx = self.tx.clone();
        let chan_queue_count = self.chan_queue_count.clone();
        let priority = self.cfg.priority;

        self.register
            .add_priority(
                Type::ClientDisconnected,
                priority,
                Box::new(TransferHandler {
                    tx: tx.clone(),
                    chan_queue_count: chan_queue_count.clone(),
                }),
            )
            .await;

        self.register
            .add(
                Type::MessageDropped,
                Box::new(TransferHandler {
                    tx: tx.clone(),
                    chan_queue_count: chan_queue_count.clone(),
                }),
            )
            .await;

        self.register
            .add(
                Type::MessagePublish,
                Box::new(TransferHandler {
                    tx: tx.clone(),
                    chan_queue_count: chan_queue_count.clone(),
                }),
            )
            .await;

        Ok(())
    }

    #[inline]
    async fn get_config(&self) -> Result<serde_json::Value> {
        self.cfg.to_json()
    }

    #[inline]
    async fn load_config(&mut self) -> Result<()> {
        log::warn!("{log_prefix} not supported reload config");
        Ok(())
    }

    #[inline]
    async fn start(&mut self) -> Result<()> {
        self.register.start().await;
        log::info!("{log_prefix} start ok");
        Ok(())
    }

    #[inline]
    async fn stop(&mut self) -> Result<bool> {
        self.register.stop().await;
        log::trace!("{log_prefix} stop");
        Ok(true)
    }

    //指标逻辑
    #[inline]
    async fn attrs(&self) -> serde_json::Value {
        let chan_queue_count = self.chan_queue_count.load(Ordering::SeqCst);
        json!({
            "chan_queue_count": chan_queue_count,
            "failure_count": self.fails.count()
        })
    }
}
