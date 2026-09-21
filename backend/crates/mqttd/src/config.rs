#![allow(dead_code)]
use anyhow::{Result, anyhow};
use config::{Config, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use toolkit_rs::logger::LogConfig;

#[derive(Debug, Serialize, Deserialize)]
struct Conf {
    pub app_no: String,
    pub upgrade_category_id: String,
    pub upgrade_category_no: String,

}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_no: String,
    pub upgrade_category_id: String,
    pub upgrade_category_no: String,
}
pub static CONF: OnceLock<AppConfig> = OnceLock::new();
//读取配置文件
pub fn init() -> Result<LogConfig> {
    let c = Config::builder()
        .add_source(File::new("./etc/mqttd.toml", FileFormat::Toml).required(true))
        .build()?;

    let mut cfg: Conf = c.try_deserialize::<Conf>()?;
    if cfg.app_no.is_empty() {
        return Err(anyhow::Error::msg("app_no 不能为空"));
    }


    let acfg = AppConfig {
        app_no: cfg.app_no,
        upgrade_category_id: cfg.upgrade_category_id,
        upgrade_category_no: cfg.upgrade_category_no,
    };
    CONF.set(acfg)
        .map_err(|_| anyhow!("settings init failed"))?;

    Ok(LogConfig::default())
}
