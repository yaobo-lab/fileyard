use async_trait::async_trait;
use config::PluginConfig;
use kv_storage::init_db;
use rmqtt::{
    Result, context::ServerContext, hook::Register, macros::Plugin, plugin::Plugin, register,
    retain::RetainStorage,
};
use serde_json::{self};
use std::sync::Arc;
mod config;
mod storage;
register!(RetainerPlugin::new);

const log_prefix: &str = "[plugin-retainer]";

#[derive(Plugin)]
struct RetainerPlugin {
    scx: ServerContext,
    register: Box<dyn Register>,
    cfg: Arc<PluginConfig>,
    store: storage::Retainer,
}

impl RetainerPlugin {
    #[inline]
    async fn new<N: Into<String>>(scx: ServerContext, name: N) -> Result<Self> {
        let name = name.into();
        let cfg = scx.plugins.read_config::<PluginConfig>(&name)?;
        let storage_db = init_db(&cfg.storage).await?;
        let register = scx.extends.hook_mgr().register();
        let cfg = Arc::new(cfg);
        let store = storage::Retainer::new(cfg.clone(), storage_db).await?;
        Ok(Self {
            scx,
            register,
            cfg,
            store,
        })
    }
}

#[async_trait]
impl Plugin for RetainerPlugin {
    #[inline]
    async fn init(&mut self) -> Result<()> {
        log::trace!("{log_prefix} init");
        Ok(())
    }

    #[inline]
    async fn get_config(&self) -> Result<serde_json::Value> {
        self.cfg.clone().to_json()
    }

    #[inline]
    async fn load_config(&mut self) -> Result<()> {
        log::warn!("{log_prefix} not supported reload config");
        Ok(())
    }

    #[inline]
    async fn start(&mut self) -> Result<()> {
        let r: Box<dyn RetainStorage> = Box::new(self.store.clone());
        *self.scx.extends.retain_mut().await = r;
        self.register.start().await;
        log::info!("{log_prefix} start ok");
        Ok(())
    }

    #[inline]
    async fn stop(&mut self) -> Result<bool> {
        log::trace!("{log_prefix} stop, cannot be stopped");
        Ok(false)
    }

    #[inline]
    async fn attrs(&self) -> serde_json::Value {
        self.store.info().await
    }
}
