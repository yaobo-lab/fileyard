use async_trait::async_trait;
use config::PluginConfig;
use rmqtt::{
    Result,
    context::ServerContext,
    hook::{Register, Type},
    macros::Plugin,
    plugin::Plugin,
    register,
};
use std::sync::Arc;
mod config;
mod handler;
mod utils;
use handler::AuthHandler;
register!(AuthPlugin::new);

const log_prefix: &str = "[plugin-auth]";

#[derive(Plugin)]
struct AuthPlugin {
    scx: ServerContext,
    register: Box<dyn Register>,
    cfg: Arc<PluginConfig>,
}

impl AuthPlugin {
    #[inline]
    async fn new<S: Into<String>>(scx: ServerContext, name: S) -> Result<Self> {
        let name = name.into();
        let cfg = scx.plugins.read_config::<PluginConfig>(&name)?;
        let cfg = Arc::new(cfg);
        let register = scx.extends.hook_mgr().register();
        Ok(Self { scx, register, cfg })
    }
}

#[async_trait]
impl Plugin for AuthPlugin {
    #[inline]
    async fn init(&mut self) -> Result<()> {
        log::trace!("{log_prefix} init");

        let priority = self.cfg.priority;

        self.register
            .add_priority(
                Type::ClientSubscribeCheckAcl,
                priority,
                Box::new(AuthHandler::new(self.cfg.clone(), self.scx.clone())),
            )
            .await;
        self.register
            .add_priority(
                Type::MessagePublishCheckAcl,
                priority,
                Box::new(AuthHandler::new(self.cfg.clone(), self.scx.clone())),
            )
            .await;

        self.register
            .add_priority(
                Type::ClientAuthenticate,
                priority,
                Box::new(AuthHandler::new(self.cfg.clone(), self.scx.clone())),
            )
            .await;

        self.register
            .add_priority(
                Type::ClientDisconnected,
                priority,
                Box::new(AuthHandler::new(self.cfg.clone(), self.scx.clone())),
            )
            .await;
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

    #[inline]
    async fn attrs(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}
