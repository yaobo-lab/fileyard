use anyhow::anyhow;
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
    #[serde(default)]
    pub wecom: WeComConf,
    #[serde(default)]
    pub gitlab: GitlabConf,
    #[serde(default)]
    pub mqtt: MqttConf,
    pub rate_limit: RateLimitConf,
    pub frontend_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConf {
    pub enabled: bool,
    pub config_path: String,
    pub plugins_dir: String,
}

impl Default for MqttConf {
    fn default() -> Self {
        Self {
            enabled: true,
            config_path: "etc/mqttd.toml".to_string(),
            plugins_dir: "etc/plugins".to_string(),
        }
    }
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeComConf {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub corp_id: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub corp_secret: String,
    #[serde(default)]
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitlabConf {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_gitlab_url")]
    pub url: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub default_project_id: Option<String>,
    #[serde(default = "default_gitlab_timeout")]
    pub timeout_secs: u64,
}

impl Default for GitlabConf {
    fn default() -> Self {
        Self {
            enabled: false,
            url: default_gitlab_url(),
            token: String::new(),
            default_project_id: None,
            timeout_secs: default_gitlab_timeout(),
        }
    }
}

fn default_gitlab_url() -> String {
    "https://gitlab.com".to_string()
}

fn default_gitlab_timeout() -> u64 {
    30
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

// 使用 toolkit-rs 中的 config 共用方法
pub use toolkit_rs::config::{
    read_config, read_config_default, read_config_default_with, read_config_with,
    read_config_with_required,
};
