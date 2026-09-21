use async_trait::async_trait;
use backoff::ExponentialBackoff;
use config::PluginConfig;
use rmqtt::{
    Result,
    context::ServerContext,
    hook::{Register, Type},
    macros::Plugin,
    plugin::{PackageInfo, Plugin},
    register,
    types::DashMap,
    utils::Counter,
};
use rust_box::task_exec_queue::{SpawnExt, TaskExecQueue};
use serde_json::{self, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;
use tokio::{
    self,
    sync::RwLock,
    sync::mpsc::{Receiver, Sender, channel},
    time,
};
use utils::{HookWriters, new_http_client};
mod config;
mod handler;
mod rule;
mod utils;
use handler::*;

register!(WebHookPlugin::new);

const log_prefix: &str = "[plugin-webhook]";

#[derive(Plugin)]
struct WebHookPlugin {
    scx: ServerContext,
    register: Box<dyn Register>,
    cfg: Arc<RwLock<PluginConfig>>,
    chan_queue_count: Arc<AtomicIsize>,
    tx: Arc<RwLock<Sender<Message>>>,
    exec: TaskExecQueue,
    fails: Arc<Counter>,
}

impl WebHookPlugin {
    #[inline]
    async fn new<S: Into<String>>(scx: ServerContext, name: S) -> Result<Self> {
        let name = name.into();
        let cfg = Arc::new(RwLock::new(Self::load_config(&scx, &name)?));
        let writers = Arc::new(DashMap::default());
        let chan_queue_count = Arc::new(AtomicIsize::new(0));
        let fails = Arc::new(Counter::new());
        let httpc = new_http_client()?;
        let (tx, exec) = Self::start(
            scx.clone(),
            httpc,
            cfg.clone(),
            writers,
            chan_queue_count.clone(),
            fails.clone(),
        )
        .await;
        let tx = Arc::new(RwLock::new(tx));
        let register = scx.extends.hook_mgr().register();
        Ok(Self {
            scx,
            register,
            cfg,
            chan_queue_count,
            tx,
            exec,
            fails,
        })
    }

    async fn start(
        scx: ServerContext,
        httpc: reqwest::Client,
        cfg: Arc<RwLock<PluginConfig>>,
        writers: HookWriters,
        chan_queue_count: Arc<AtomicIsize>,
        fails: Arc<Counter>,
    ) -> (Sender<Message>, TaskExecQueue) {
        let (tx, mut rx): (Sender<Message>, Receiver<Message>) =
            channel(cfg.read().await.queue_capacity);

        let (exec_tx, exec_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            log::info!("{}: start web-hook async worker.", log_prefix);
            let runner = async {
                let exec = scx.get_exec((
                    "WEB_HOOK_EXEC",
                    cfg.read().await.concurrency_limit,
                    cfg.read().await.queue_capacity,
                ));

                // 运行队列创建完成
                if exec_tx.send(exec.clone()).is_err() {
                    log::error!("{}: tokio oneshot channel send failed", log_prefix);
                }

                let backoff_strategy = Arc::new(cfg.read().await.get_backoff_strategy());
                loop {
                    let cfg = cfg.clone();
                    let writers = writers.clone();
                    let backoff_strategy = backoff_strategy.clone();

                    match rx.recv().await {
                        Some(msg) => {
                            //计数器
                            chan_queue_count.fetch_sub(1, Ordering::SeqCst);
                            log::trace!("{}: received web-hook Message: {msg:?}", log_prefix);

                            //等待队列空闲
                            if exec.is_full() {
                                loop {
                                    time::sleep(Duration::from_millis(1)).await;
                                    if !exec.is_full() {
                                        break;
                                    }
                                }
                            }

                            //并发 处理消息
                            Self::spawn_handle_msg(
                                &exec,
                                httpc.clone(),
                                cfg,
                                writers,
                                backoff_strategy,
                                msg,
                                fails.clone(),
                            )
                            .await;
                        }
                        None => {
                            log::info!("{}: web hook message channel is closed", log_prefix);
                            break;
                        }
                    }
                }
            };

            runner.await;
            log::info!("{}: exit web-hook async worker.", log_prefix);
        });

        //等待 web-hook 线程初始化完成
        let exec = exec_rx.await.expect(&format!(
            "{}: tokio oneshot channel recv failed",
            log_prefix
        ));
        (tx, exec)
    }

    #[inline]
    fn load_config(scx: &ServerContext, name: &str) -> Result<PluginConfig> {
        let mut cfg = scx
            .plugins
            .read_config_with::<PluginConfig>(name, &["urls"])?;
        cfg.merge_urls();
        Ok(cfg)
    }

    // 并发处理消息
    #[inline]
    async fn spawn_handle_msg(
        exec: &TaskExecQueue,
        httpc: reqwest::Client,
        cfg: Arc<RwLock<PluginConfig>>,
        writers: HookWriters,
        backoff_strategy: Arc<ExponentialBackoff>,
        msg: Message,
        fails: Arc<Counter>,
    ) {
        if let Err(e) = async move {
            let (typ, topic, data) = msg;
            if let Err(e) = utils::handle_msg(
                &httpc,
                cfg,
                writers,
                backoff_strategy,
                typ,
                topic,
                data,
                fails.as_ref(),
            )
            .await
            {
                log::warn!("Failed to build the web-hook message, {e:?}");
            }
        }
        .spawn(exec)
        .await
        {
            log::error!(
                "send web hook message failure, exec task error, {:?}",
                e.to_string()
            );
        }
    }
}

#[async_trait]
impl Plugin for WebHookPlugin {
    #[inline]
    async fn init(&mut self) -> Result<()> {
        log::trace!("[plugin] web hook init");
        let tx = self.tx.clone();
        let chan_queue_count = self.chan_queue_count.clone();

        self.register
            .add(
                Type::ClientConnected,
                Box::new(WebHookHandler {
                    tx: tx.clone(),
                    chan_queue_count: chan_queue_count.clone(),
                }),
            )
            .await;
        self.register
            .add(
                Type::ClientDisconnected,
                Box::new(WebHookHandler {
                    tx: tx.clone(),
                    chan_queue_count: chan_queue_count.clone(),
                }),
            )
            .await;

        self.register
            .add(
                Type::MessagePublish,
                Box::new(WebHookHandler {
                    tx: tx.clone(),
                    chan_queue_count: chan_queue_count.clone(),
                }),
            )
            .await;

        self.register
            .add(
                Type::MessageDropped,
                Box::new(WebHookHandler {
                    tx: tx.clone(),
                    chan_queue_count: chan_queue_count.clone(),
                }),
            )
            .await;

        Ok(())
    }

    #[inline]
    async fn get_config(&self) -> Result<serde_json::Value> {
        self.cfg.read().await.to_json()
    }

    #[inline]
    async fn load_config(&mut self) -> Result<()> {
        let new_cfg = Self::load_config(&self.scx, self.name())?;
        *self.cfg.write().await = new_cfg;
        Ok(())
    }

    #[inline]
    async fn start(&mut self) -> Result<()> {
        self.register.start().await;
        log::info!("[plugin] web hook start ok");
        Ok(())
    }

    #[inline]
    async fn stop(&mut self) -> Result<bool> {
        self.register.stop().await;
        log::debug!("[plugin] web hook stop");
        Ok(true)
    }

    //指标逻辑
    #[inline]
    async fn attrs(&self) -> serde_json::Value {
        let chan_queue_count = self.chan_queue_count.load(Ordering::SeqCst);
        let exec = &self.exec;
        json!({
            "chan_queue_count": chan_queue_count,
            "task_exec_queue": {
                "active_count": exec.active_count(),
                "waiting_count": exec.waiting_count(),
                "completed_count": exec.completed_count().await,
                "failure_count": self.fails.count(),
            }
        })
    }
}
