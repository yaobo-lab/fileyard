use anyhow::anyhow;
use config::{Config, File, Source};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::OnceLock};
use toolkit_rs::{logger::LogConfig, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conf {
    pub log: LogConfig,
    pub web: WebServerConf,
    pub database: DatabaseConf,
    pub storage: StorageConf,
    pub redis: RedisConf,
    pub extensions: ExtensionsConf,
    pub cdn: CdnConf,
    pub transfer: TransferConf,
    pub replication: ReplicationConf,
    pub virus_scan: VirusScanConf,
    pub backup: BackupConf,
    pub api_usage: ApiUsageConf,
    pub cors: CorsConf,
    pub auth: AuthConf,
    pub discord: DiscordConf,
    pub rate_limit: RateLimitConf,
    pub frontend_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerConf {
    pub port: u16,
    pub listen_addr: String,
    pub max_concurrent_requests: usize,
    pub request_timeout_secs: u64,
    pub base_url: String,
}

impl WebServerConf {
    pub fn into_addr(&self) -> String {
        format!("{}:{}", self.listen_addr, self.port)
    }

    pub fn into_http_addr(&self) -> String {
        format!("http://{}:{}", self.listen_addr, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConf {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConf {
    pub kind: String,
    pub local_path: String,
    pub encryption_key: Option<String>,
    pub s3_bucket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConf {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionsConf {
    pub webhook_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnConf {
    pub use_presigned_urls: bool,
    pub presigned_url_expiry_secs: u64,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferConf {
    pub small_concurrent: usize,
    pub medium_concurrent: usize,
    pub large_concurrent: usize,
    pub large_bandwidth_mbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConf {
    pub enabled: bool,
    pub endpoint: Option<String>,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub mode: String,
    pub retry_seconds: u64,
    pub workers: u32,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirusScanConf {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
    pub workers: u32,
    pub max_file_size_mb: i64,
    pub max_queue_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConf {
    pub master_key: Option<String>,
    pub max_concurrent: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiUsageConf {
    pub enabled: bool,
    pub sample_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConf {
    pub environment: String,
    pub dev_mode: bool,
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConf {
    pub jwt_secret: String,
    pub jwt_secret_secondary: Option<String>,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_expiry_secs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConf {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConf {
    pub trust_all_proxies: bool,
    pub trusted_proxy_ips: Vec<String>,
    pub per_ip_requests_per_sec: u32,
    pub per_ip_burst_size: u32,
}

static CONFIG: OnceLock<Conf> = OnceLock::new();

pub fn init_config(config: Conf) -> AppResult<()> {
    CONFIG
        .set(config)
        .map_err(|_| anyhow!("configuration is already initialized"))
}

pub fn get_config() -> &'static Conf {
    CONFIG.get_or_init(|| {
        read_config("etc/config.toml").unwrap_or_else(|_| {
            let workspace_config = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("etc/config.toml");
            read_config(
                workspace_config
                    .to_str()
                    .expect("Configuration path is not valid UTF-8"),
            )
            .expect("Failed to read etc/config.toml")
        })
    })
}

//读取配置文件
pub fn read_config<'de, T: serde::Deserialize<'de>>(cfg_file: &str) -> AppResult<T> {
    let (cfg, _) = read_config_with_required(cfg_file, true, &[])?;
    Ok(cfg)
}

pub fn read_config_default<'de, T: serde::Deserialize<'de>>(cfg_file: &str) -> AppResult<T> {
    let (cfg, def) = read_config_with_required(cfg_file, false, &[])?;
    if def {
        log::warn!(
            "The configuration for  '{cfg_file}' does not exist, default values will be used!"
        );
    }
    Ok(cfg)
}

pub fn read_config_with<'de, T: serde::Deserialize<'de>>(
    cfg_file: &str,
    env_list_keys: &[&str],
) -> AppResult<T> {
    let (cfg, _) = read_config_with_required(cfg_file, true, env_list_keys)?;
    Ok(cfg)
}

pub fn read_config_default_with<'de, T: serde::Deserialize<'de>>(
    cfg_file: &str,
    env_list_keys: &[&str],
) -> AppResult<T> {
    let (cfg, def) = read_config_with_required(cfg_file, false, env_list_keys)?;
    if def {
        log::warn!(
            "The configuration for  '{cfg_file}' does not exist, default values will be used!"
        );
    }
    Ok(cfg)
}

pub fn read_config_with_required<'de, T: serde::Deserialize<'de>>(
    cfg_file: &str,
    required: bool,
    env_list_keys: &[&str],
) -> AppResult<(T, bool)> {
    let path = Path::new(cfg_file);
    if !path.is_file() {
        return Err(anyhow!(format!("not found: {cfg_file}")));
    }
    let builder = Config::builder().add_source(File::from(path).required(required));
    let mut env = config::Environment::with_prefix(&format!("gateway_"));
    if !env_list_keys.is_empty() {
        env = env.try_parsing(true).list_separator(" ");
        for key in env_list_keys {
            env = env.with_list_parse_key(key);
        }
    }

    let builder = builder.add_source(env);
    let s = builder.build()?;
    let count = s.collect()?.len();
    Ok((s.try_deserialize::<T>()?, count == 0))
}
