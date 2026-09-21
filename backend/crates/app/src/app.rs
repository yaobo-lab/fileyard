use crate::storage::{EncryptedLocalStorage, LocalStorage, S3Storage, Storage};
use app_core::cache::Cache;
use app_extensions::routes::ExtensionState;
use sea_orm_migration::MigratorTrait;
use std::sync::Arc;
use std::time::Duration;
use toolkit_rs::logger;
use toolkit_rs::painc::{set_panic_handler, PaincConf};

use crate::{
    api::{health, settings_backup},
    middleware::{self, ApiUsageWriter, TransferScheduler},
    router,
};

/// Adapter to make the storage implement PrimaryStorageReader for replication
struct PrimaryStorageAdapter(Arc<dyn Storage>);

#[async_trait::async_trait]
impl app_core::replication::PrimaryStorageReader for PrimaryStorageAdapter {
    async fn download(
        &self,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        self.0.download(key).await
    }
}

/// Adapter to make the storage implement FileStorageReader for virus scanning
struct VirusScanStorageAdapter(Arc<dyn Storage>);

#[async_trait::async_trait]
impl app_core::virus_scan::FileStorageReader for VirusScanStorageAdapter {
    async fn download(
        &self,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        self.0.download(key).await
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.0.delete(key).await
    }
}

// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// The only PostgreSQL capability exposed to application code.
    pub store: app_entity::DataStore,
    pub storage: Arc<dyn Storage>,
    pub redis_url: String,
    pub cache: Option<Cache>,
    pub extension_webhook_timeout_ms: u64,
    // CDN / Presigned URL configuration
    pub use_presigned_urls: bool,
    pub presigned_url_expiry: u64,
    pub cdn_domain: Option<String>,
    // Transfer scheduling for downloads/uploads
    pub scheduler: Arc<TransferScheduler>,
    // S3 Replication configuration
    pub replication_config: app_core::replication::ReplicationConfig,
    // Virus scanning configuration
    pub virus_scan_config: app_core::virus_scan::VirusScanConfig,
    // ClamAV circuit breaker (shared across workers)
    pub clamav_circuit_breaker: Option<Arc<app_core::circuit_breaker::CircuitBreaker>>,
    // Backup circuit breaker + concurrency limit
    pub backup_circuit_breaker: Arc<app_core::circuit_breaker::CircuitBreaker>,
    pub backup_semaphore: Arc<tokio::sync::Semaphore>,
}

pub async fn run() {
    set_panic_handler(PaincConf {
        version: "".to_string(),
        build_time: "".to_string(),
        painc_exit: true,
    });

    let config = types::config::get_config().clone();

    let cf = config.log.clone();
    logger::setup(cf).expect("日志初始化失败");

    // Initialize Storage
    let storage_type = &config.storage.kind;
    let encryption_key = config
        .storage
        .encryption_key
        .as_ref()
        .filter(|key| !key.is_empty());

    let storage: Arc<dyn Storage> = if storage_type == "s3" {
        let bucket = config.storage.s3_bucket.clone();
        if encryption_key.is_some() {
            log::info!("S3 storage uses provider-side encryption (ENCRYPTION_KEY ignored for S3)");
        }
        Arc::new(S3Storage::new(bucket).await)
    } else {
        // Local storage - optionally enable ChaCha20-Poly1305 encryption
        if let Some(ref key_base64) = encryption_key {
            match EncryptedLocalStorage::from_base64_key(&config.storage.local_path, key_base64) {
                Ok(encrypted_storage) => {
                    log::info!("Local storage encryption ENABLED (ChaCha20-Poly1305)");
                    Arc::new(encrypted_storage)
                }
                Err(e) => {
                    log::error!(
                        "Failed to initialize encrypted storage: {}. Falling back to unencrypted.",
                        e
                    );
                    log::warn!("ENCRYPTION_KEY is set but invalid - files will NOT be encrypted!");
                    Arc::new(LocalStorage::new(&config.storage.local_path))
                }
            }
        } else {
            log::info!("Local storage encryption DISABLED (set ENCRYPTION_KEY to enable)");
            Arc::new(LocalStorage::new(&config.storage.local_path))
        }
    };

    // Initialize Redis URL
    let redis_url = config.redis.url.clone();

    // Initialize Redis Cache
    let cache = match Cache::new(&redis_url).await {
        Ok(c) => {
            log::info!("Redis cache initialized successfully");
            Some(c)
        }
        Err(e) => {
            log::warn!("Failed to initialize Redis cache (caching disabled): {}", e);
            None
        }
    };

    // Extension webhook timeout
    let extension_webhook_timeout_ms = config.extensions.webhook_timeout_ms;

    // Initialize Database with production pool settings
    let database_url = &config.database.url;

    // Pool configuration from environment or defaults
    let max_connections = config.database.max_connections;
    let min_connections = config.database.min_connections;
    let acquire_timeout_secs = config.database.acquire_timeout_secs;
    let idle_timeout_secs = config.database.idle_timeout_secs;
    let max_lifetime_secs = config.database.max_lifetime_secs;

    log::info!(
        "Connecting to database (max_conn: {}, min_conn: {})...",
        max_connections,
        min_connections
    );

    let db = app_entity::connect(app_entity::DatabaseConfig {
        url: database_url,
        max_connections,
        min_connections,
        acquire_timeout: Duration::from_secs(acquire_timeout_secs),
        idle_timeout: Duration::from_secs(idle_timeout_secs),
        max_lifetime: Duration::from_secs(max_lifetime_secs),
        sqlx_logging: false,
    })
    .await
    .expect("Failed to connect to database through SeaORM");

    // Run the immutable SQL baseline and all subsequent migrations through SeaORM.
    app_migration::Migrator::up(&db, None)
        .await
        .expect("Failed to run SeaORM migrations");

    log::info!("Database connected successfully with optimized pool settings");
    let store = app_entity::DataStore::new(db.clone());

    // CDN / Presigned URL configuration (optional, disabled by default for backwards compatibility)
    let use_presigned_urls = config.cdn.use_presigned_urls;
    let presigned_url_expiry = config.cdn.presigned_url_expiry_secs;
    let cdn_domain = config.cdn.domain.clone();

    if use_presigned_urls {
        log::info!(
            "Presigned URLs enabled (expiry: {}s, CDN: {:?})",
            presigned_url_expiry,
            cdn_domain
        );
    }

    // Initialize transfer scheduler for prioritized downloads/uploads
    let scheduler = Arc::new(TransferScheduler::with_config(
        middleware::TransferSchedulerConfig {
            small_concurrent: config.transfer.small_concurrent,
            medium_concurrent: config.transfer.medium_concurrent,
            large_concurrent: config.transfer.large_concurrent,
            large_bandwidth_bps: config.transfer.large_bandwidth_mbps * 1024 * 1024,
        },
    ));
    log::info!("Transfer scheduler initialized");

    // Load S3 replication configuration
    let replication_config = app_core::replication::ReplicationConfig {
        enabled: config.replication.enabled,
        endpoint: config
            .replication
            .endpoint
            .clone()
            .filter(|value| !value.is_empty()),
        bucket: config.replication.bucket.clone(),
        region: config.replication.region.clone(),
        access_key: config.replication.access_key.clone(),
        secret_key: config.replication.secret_key.clone(),
        mode: config
            .replication
            .mode
            .parse()
            .expect("Invalid replication.mode"),
        retry_seconds: config.replication.retry_seconds,
        workers: config.replication.workers,
        max_retries: config.replication.max_retries,
    };
    if replication_config.enabled {
        if let Err(e) = replication_config.validate() {
            log::error!(
                "Replication configuration error: {}. Disabling replication.",
                e
            );
        } else {
            log::info!(
                "S3 replication enabled: mode={:?}, bucket={}, workers={}",
                replication_config.mode,
                replication_config.bucket,
                replication_config.workers
            );
        }
    } else {
        log::info!("S3 replication disabled");
    }

    // Validate BACKUP_MASTER_KEY — required for backup at-rest encryption
    match config
        .backup
        .master_key
        .as_deref()
        .filter(|key| !key.is_empty())
    {
        Some(k) if k.len() >= 32 => {
            log::info!(
                "BACKUP_MASTER_KEY validated ({} chars) — backup at-rest encryption enabled",
                k.len()
            );
        }
        Some(k) => {
            log::error!("BACKUP_MASTER_KEY is too short ({} chars, minimum 32). Backup at-rest encryption will fail. Generate with: openssl rand -base64 48", k.len());
        }
        None => {
            log::warn!("BACKUP_MASTER_KEY not set — backup passphrase at-rest encryption disabled. Set it to enable scheduled backups. Generate with: openssl rand -base64 48");
        }
    }

    // Mark server start time for uptime tracking
    health::mark_server_start();

    // Initialize API usage tracking
    let api_usage_enabled = config.api_usage.enabled;

    let api_usage_writer = if api_usage_enabled {
        log::info!("API usage tracking enabled");
        Some(Arc::new(ApiUsageWriter::new(store.clone())))
    } else {
        log::info!("API usage tracking disabled");
        None
    };

    // Load virus scan configuration
    let virus_scan_config = app_core::virus_scan::VirusScanConfig {
        enabled: config.virus_scan.enabled,
        host: config.virus_scan.host.clone(),
        port: config.virus_scan.port,
        timeout_ms: config.virus_scan.timeout_ms,
        workers: config.virus_scan.workers,
        max_file_size_mb: config.virus_scan.max_file_size_mb,
        max_queue_size: config.virus_scan.max_queue_size,
    };
    if virus_scan_config.enabled {
        log::info!(
            "ClamAV virus scanning enabled: host={}, port={}, workers={}",
            virus_scan_config.host,
            virus_scan_config.port,
            virus_scan_config.workers
        );
    } else {
        log::info!("ClamAV virus scanning disabled");
    }

    // Create ClamAV circuit breaker if virus scanning is enabled
    let clamav_circuit_breaker = if virus_scan_config.enabled {
        Some(Arc::new(
            app_core::circuit_breaker::CircuitBreaker::new(
                "clamav", 5,  // failure threshold - opens after 5 consecutive failures
                30, // recovery timeout - tries half-open after 30 seconds
                3,  // success threshold - closes after 3 successes in half-open
            ),
        ))
    } else {
        None
    };

    let app_state = Arc::new(AppState {
        store,
        storage: storage.clone(),
        redis_url: redis_url.clone(),
        cache,
        extension_webhook_timeout_ms,
        use_presigned_urls,
        presigned_url_expiry,
        cdn_domain,
        scheduler,
        replication_config: replication_config.clone(),
        virus_scan_config: virus_scan_config.clone(),
        clamav_circuit_breaker: clamav_circuit_breaker.clone(),
        backup_circuit_breaker: Arc::new(app_core::circuit_breaker::CircuitBreaker::new(
            "backup", 3,  // failure threshold - opens after 3 infrastructure failures
            60, // recovery timeout - tries half-open after 60 seconds
            2,  // success threshold - closes after 2 successes in half-open
        )),
        backup_semaphore: Arc::new(tokio::sync::Semaphore::new(config.backup.max_concurrent)),
    });

    // Extension state for extension routes
    let extension_state = Arc::new(ExtensionState {
        store: app_state.store.clone(),
        redis_url: redis_url.clone(),
        webhook_timeout_ms: extension_webhook_timeout_ms,
    });

    // Start automation scheduler in background
    let scheduler_store = app_state.store.clone();
    let scheduler_redis_url = redis_url.clone();
    tokio::spawn(async move {
        match app_extensions::scheduler::Scheduler::new(
            scheduler_store,
            &scheduler_redis_url,
            extension_webhook_timeout_ms,
        )
        .await
        {
            Ok(scheduler) => {
                log::info!("Starting automation scheduler...");
                scheduler.start().await;
            }
            Err(e) => {
                log::error!("Failed to start automation scheduler: {:?}", e);
            }
        }
    });

    // Start backup scheduler in background
    {
        let backup_store = app_state.store.clone();
        let backup_storage = storage.clone();
        let backup_cb = app_state.backup_circuit_breaker.clone();
        let backup_sem = app_state.backup_semaphore.clone();
        let backup_redis = redis_url.clone();
        tokio::spawn(async move {
            settings_backup::start_backup_scheduler(
                backup_store,
                backup_storage,
                backup_cb,
                backup_sem,
                backup_redis,
            )
            .await;
        });
    }

    // Start S3 replication workers if enabled
    if replication_config.enabled && replication_config.validate().is_ok() {
        let worker_count = replication_config.workers;
        log::info!("Starting {} replication workers...", worker_count);

        for worker_id in 0..worker_count {
            let worker_store = app_state.store.clone();
            let worker_config = replication_config.clone();
            let worker_storage = storage.clone();

            tokio::spawn(async move {
                let storage_reader = Arc::new(PrimaryStorageAdapter(worker_storage));

                match app_core::replication::ReplicationWorker::new(
                    worker_store,
                    worker_config,
                    storage_reader,
                    worker_id,
                )
                .await
                {
                    Ok(worker) => {
                        worker.run().await;
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to start replication worker: worker_id={}, error={}",
                            worker_id,
                            e
                        );
                    }
                }
            });
        }
    }

    // Start virus scan workers if enabled
    if virus_scan_config.enabled {
        let worker_count = virus_scan_config.workers;
        log::info!("Starting {} virus scan workers...", worker_count);

        let cb = clamav_circuit_breaker
            .clone()
            .expect("Circuit breaker should exist when ClamAV is enabled");

        for worker_id in 0..worker_count {
            let worker_store = app_state.store.clone();
            let worker_config = virus_scan_config.clone();
            let worker_storage = storage.clone();
            let worker_circuit_breaker = cb.clone();

            tokio::spawn(async move {
                let storage_reader = Arc::new(VirusScanStorageAdapter(worker_storage));
                let worker = app_core::virus_scan::VirusScanWorker::new(
                    worker_store,
                    worker_config,
                    storage_reader,
                    worker_id,
                    worker_circuit_breaker,
                );
                worker.run().await;
            });
        }
    }

    // 监听 MQTT 客户端连接/断开事件并登记到数据库表 mqtt_clients
    let store_for_mqtt = app_state.store.clone();
    mqttd::events::set_on_client_connected({
        let store = store_for_mqtt.clone();
        move |event| {
            let store = store.clone();
            tokio::spawn(async move {
                log::info!(
                    "登记 MQTT 客户端连接: username={}, client_id={}, ip={:?}",
                    event.username,
                    event.client_id,
                    event.ip_address
                );
                if let Err(e) = store
                    .mqtt_clients()
                    .upsert_connected(
                        &event.username,
                        &event.client_id,
                        event.ip_address.as_deref(),
                        event.port,
                        event.proto_ver,
                        event.keepalive,
                        event.clean_start,
                    )
                    .await
                {
                    log::error!("登记 MQTT 客户端连接失败: {e}");
                }
            });
        }
    });

    mqttd::events::set_on_client_disconnected({
        let store = store_for_mqtt;
        move |event| {
            let store = store.clone();
            tokio::spawn(async move {
                log::info!(
                    "标记 MQTT 客户端断开: username={:?}, client_id={:?}, reason={:?}",
                    event.username,
                    event.client_id,
                    event.reason
                );
                if let Err(e) = store
                    .mqtt_clients()
                    .mark_disconnected(
                        event.username.as_deref(),
                        event.client_id.as_deref(),
                        event.reason.as_deref(),
                    )
                    .await
                {
                    log::error!("标记 MQTT 客户端断开失败: {e}");
                }
            });
        }
    });

    let mqtt_api = mqttd::plugins::restapi::EmbeddedApi::default();
    // 启动 MQTT 服务器（如果启用）
    if config.mqtt.enabled {
        let mqtt_config_path = config.mqtt.config_path.clone();
        let mqtt_plugins_dir = config.mqtt.plugins_dir.clone();
        let mqtt_api = mqtt_api.clone();
        tokio::spawn(async move {
            log::info!("正在启动内置 MQTT 服务器 (配置: {}, 插件目录: {})...", mqtt_config_path, mqtt_plugins_dir);
            if let Err(e) = mqttd::server::run_server_with_api(
                &mqtt_config_path,
                Some(&mqtt_plugins_dir),
                mqtt_api,
            )
            .await
            {
                log::error!("内置 MQTT 服务器运行失败: {e}");
            }
        });
    } else {
        log::info!("内置 MQTT 服务器未启用 (config.mqtt.enabled = false)");
    }

    // 构建完整的 Axum 路由器与全局中间件（参考 docs/web/router 模块化设计）
    let app = router::routers(
        app_state.clone(),
        &config,
        api_usage_writer,
        extension_state,
    )
    .layer(axum::Extension(mqtt_api));

    // Run server
    let listener = tokio::net::TcpListener::bind(config.web.into_addr())
        .await
        .unwrap();

    log::info!("🚀 Server listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}
