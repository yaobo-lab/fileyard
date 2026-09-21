use async_trait::async_trait;
use config::PluginConfig;
use rmqtt::{
    Result,
    context::ServerContext,
    hook::Register,
    macros::Plugin,
    plugin::{PackageInfo, Plugin},
    register,
};
use std::sync::Arc;
use tokio::{self, sync::RwLock, sync::oneshot};

mod api;
mod clients;
mod config;
mod plugin;
mod subs;
mod types;

type ShutdownTX = oneshot::Sender<()>;
type PluginConfigType = Arc<RwLock<PluginConfig>>;

register!(HttpApiPlugin::new);
const log_prefix: &str = "[plugin-restapi]";

#[derive(Plugin)]
struct HttpApiPlugin {
    scx: ServerContext,
    register: Box<dyn Register>,
    cfg: PluginConfigType,
    shutdown_tx: Option<ShutdownTX>,
}

impl HttpApiPlugin {
    #[inline]
    async fn new<S: Into<String>>(scx: ServerContext, name: S) -> Result<Self> {
        let name = name.into();
        let cfg = Arc::new(RwLock::new(scx.plugins.read_config::<PluginConfig>(&name)?));
        let register = scx.extends.hook_mgr().register();
        let shutdown_tx = Some(Self::start(scx.clone(), cfg.clone()).await);
        Ok(Self {
            scx,
            register,
            cfg,
            shutdown_tx,
        })
    }

    async fn start(scx: ServerContext, cfg: PluginConfigType) -> ShutdownTX {
        let (shutdown_tx, shutdown_rx): (oneshot::Sender<()>, oneshot::Receiver<()>) =
            oneshot::channel();
        let http_laddr = cfg.read().await.http_laddr;
        tokio::spawn(async move {
            if let Err(e) = api::listen_and_serve(scx, http_laddr, cfg, shutdown_rx).await {
                log::error!("{log_prefix} {e:?}");
            }
            log::info!("{log_prefix} Exit http server...http://{http_laddr:?}");
        });
        shutdown_tx
    }
}

#[async_trait]
impl Plugin for HttpApiPlugin {
    #[inline]
    async fn init(&mut self) -> Result<()> {
        log::trace!("{log_prefix} init");
        Ok(())
    }

    #[inline]
    async fn get_config(&self) -> Result<serde_json::Value> {
        self.cfg.read().await.to_json()
    }

    #[inline]
    async fn load_config(&mut self) -> Result<()> {
        let new_cfg = self.scx.plugins.read_config::<PluginConfig>(self.name())?;
        if !self.cfg.read().await.changed(&new_cfg) {
            return Ok(());
        }
        let restart_enable = self.cfg.read().await.restart_enable(&new_cfg);
        if restart_enable {
            let new_cfg = Arc::new(RwLock::new(new_cfg));
            if let Some(tx) = self.shutdown_tx.take() {
                if let Err(e) = tx.send(()) {
                    log::warn!("{log_prefix} shutdown_tx send fail, {e:?}");
                }
            }
            self.shutdown_tx = Some(Self::start(self.scx.clone(), new_cfg.clone()).await);
            self.cfg = new_cfg;
        } else {
            *self.cfg.write().await = new_cfg;
        }
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
        log::trace!("{log_prefix} stop");
        Ok(false)
    }
}
