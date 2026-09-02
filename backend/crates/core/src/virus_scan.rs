//! ClamAV Virus Scanning Module
//!
//! Provides async virus scanning using ClamAV daemon (clamd).
//! Scanning is non-blocking - uploads complete immediately while scans run in background.
//!
//! Features:
//! - Async TCP connection to clamd daemon
//! - Per-tenant configuration (enable/disable, file types, actions)
//! - Background worker pool for concurrent scanning
//! - Performance metrics tracking
//! - Quarantine support for infected files

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

fn tenant_from_entity(m: clovalink_entity::entities::tenants::Model) -> Tenant {
    Tenant { id:m.id,name:m.name,domain:m.domain,plan:m.plan,status:m.status,compliance_mode:m.compliance_mode,
        encryption_standard:m.encryption_standard,retention_policy_days:m.retention_policy_days,
        storage_quota_bytes:m.storage_quota_bytes,storage_used_bytes:m.storage_used_bytes.unwrap_or(0),smtp_host:m.smtp_host,
        smtp_port:m.smtp_port,smtp_username:m.smtp_username,smtp_password:m.smtp_password,smtp_from:m.smtp_from,smtp_secure:m.smtp_secure,
        enable_totp:m.enable_totp,enable_passkeys:m.enable_passkeys,mfa_required:m.mfa_required,session_timeout_minutes:m.session_timeout_minutes,
        public_sharing_enabled:m.public_sharing_enabled,data_export_enabled:m.data_export_enabled,max_upload_size_bytes:m.max_upload_size_bytes,
        auth_methods:Some(m.auth_methods),approval_workflow_enabled:m.approval_workflow_enabled,backup_enabled:m.backup_enabled,
        auto_backup_enabled:m.auto_backup_enabled,auto_backup_cron:m.auto_backup_cron,auto_backup_retention_count:m.auto_backup_retention_count,
        created_at:m.created_at.into(),updated_at:m.updated_at.into() }
}

use crate::circuit_breaker::CircuitBreaker;
use crate::models::Tenant;
use crate::notification_service;
use crate::security_service;

// =============================================================================
// Errors
// =============================================================================

#[derive(Debug, Error)]
pub enum VirusScanError {
    #[error("Virus scanning is disabled")]
    Disabled,
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Database error: {0}")]
    DataError(#[from] clovalink_entity::DataError),
    #[error("ClamAV connection error: {0}")]
    ConnectionError(String),
    #[error("ClamAV connection timeout")]
    ConnectionTimeout,
    #[error("ClamAV operation timeout")]
    OperationTimeout,
    #[error("ClamAV scan error: {0}")]
    ScanError(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Circuit breaker is open - ClamAV unavailable")]
    CircuitOpen,
    #[error("Scan queue is full")]
    QueueFull,
}

// =============================================================================
// Configuration
// =============================================================================

/// Action to take when a virus is detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectionAction {
    /// Delete the infected file permanently
    Delete,
    /// Move to quarantine (can be restored by admin)
    Quarantine,
    /// Just flag the file, don't remove it
    Flag,
}

impl Default for DetectionAction {
    fn default() -> Self {
        DetectionAction::Quarantine
    }
}

impl std::str::FromStr for DetectionAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "delete" => Ok(DetectionAction::Delete),
            "quarantine" => Ok(DetectionAction::Quarantine),
            "flag" => Ok(DetectionAction::Flag),
            _ => Err(format!(
                "Invalid detection action: {}. Use 'delete', 'quarantine', or 'flag'",
                s
            )),
        }
    }
}

impl std::fmt::Display for DetectionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectionAction::Delete => write!(f, "delete"),
            DetectionAction::Quarantine => write!(f, "quarantine"),
            DetectionAction::Flag => write!(f, "flag"),
        }
    }
}

/// Global ClamAV configuration loaded from environment
#[derive(Debug, Clone)]
pub struct VirusScanConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
    pub workers: u32,
    pub max_file_size_mb: i64,
    /// Maximum pending jobs in queue (0 = unlimited)
    pub max_queue_size: i64,
}

impl VirusScanConfig {
    /// Load configuration from the application TOML configuration.
    pub fn from_config() -> Self {
        let source = &types::config::get_config().virus_scan;
        Self {
            enabled: source.enabled,
            host: source.host.clone(),
            port: source.port,
            timeout_ms: source.timeout_ms,
            workers: source.workers,
            max_file_size_mb: source.max_file_size_mb,
            max_queue_size: source.max_queue_size,
        }
    }

    /// Get clamd address string
    pub fn clamd_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// =============================================================================
// Per-Tenant Settings
// =============================================================================

/// Per-tenant virus scan settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantScanSettings {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub enabled: bool,
    pub file_types: Vec<String>, // Empty = scan all
    pub max_file_size_mb: i32,
    pub action_on_detect: String,
    pub notify_admin: bool,
    pub notify_uploader: bool,
    pub auto_suspend_uploader: bool,
    pub suspend_threshold: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for TenantScanSettings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id: Uuid::nil(),
            enabled: true,
            file_types: vec![],
            max_file_size_mb: 100,
            action_on_detect: "quarantine".to_string(),
            notify_admin: true,
            notify_uploader: false,
            auto_suspend_uploader: false,
            suspend_threshold: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Get tenant scan settings, creating defaults if none exist
pub async fn get_tenant_settings(
    store: &clovalink_entity::DataStore,
    tenant_id: Uuid,
) -> Result<TenantScanSettings, VirusScanError> {
    let m=store.virus_scan().settings(tenant_id).await?;
    Ok(TenantScanSettings{id:m.id,tenant_id:m.tenant_id,enabled:m.enabled,file_types:m.file_types.unwrap_or_default(),
        max_file_size_mb:m.max_file_size_mb.unwrap_or(100),action_on_detect:m.action_on_detect,notify_admin:m.notify_admin,
        notify_uploader:m.notify_uploader,auto_suspend_uploader:m.auto_suspend_uploader,suspend_threshold:m.suspend_threshold,
        created_at:m.created_at.into(),updated_at:m.updated_at.into()})
}

/// Update tenant scan settings
pub async fn update_tenant_settings(
    store: &clovalink_entity::DataStore,
    tenant_id: Uuid,
    enabled: Option<bool>,
    file_types: Option<Vec<String>>,
    max_file_size_mb: Option<i32>,
    action_on_detect: Option<String>,
    notify_admin: Option<bool>,
    notify_uploader: Option<bool>,
    auto_suspend_uploader: Option<bool>,
    suspend_threshold: Option<i32>,
) -> Result<TenantScanSettings, VirusScanError> {
    let m=store.virus_scan().update_settings(tenant_id,clovalink_entity::repositories::VirusScanSettingsPatch{
        enabled,file_types,max_file_size_mb,action_on_detect,notify_admin,notify_uploader,auto_suspend_uploader,suspend_threshold}).await?;
    Ok(TenantScanSettings{id:m.id,tenant_id:m.tenant_id,enabled:m.enabled,file_types:m.file_types.unwrap_or_default(),
        max_file_size_mb:m.max_file_size_mb.unwrap_or(100),action_on_detect:m.action_on_detect,notify_admin:m.notify_admin,
        notify_uploader:m.notify_uploader,auto_suspend_uploader:m.auto_suspend_uploader,suspend_threshold:m.suspend_threshold,
        created_at:m.created_at.into(),updated_at:m.updated_at.into()})
}

// =============================================================================
// ClamAV Client
// =============================================================================

/// Scan result from ClamAV
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub is_infected: bool,
    pub threat_name: Option<String>,
    pub scan_duration_ms: u64,
    pub scanner_version: Option<String>,
}

/// ClamAV client for communicating with clamd daemon
pub struct ClamAvClient {
    config: VirusScanConfig,
}

impl ClamAvClient {
    pub fn new(config: VirusScanConfig) -> Self {
        Self { config }
    }

    /// Get connection timeout duration
    fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.config.timeout_ms.min(10000)) // Max 10s for connect
    }

    /// Get operation timeout duration (for scan operations)
    fn operation_timeout(&self) -> Duration {
        Duration::from_millis(self.config.timeout_ms)
    }

    /// Ping clamd to check if it's running
    pub async fn ping(&self) -> Result<bool, VirusScanError> {
        let connect_timeout = self.connect_timeout();
        let op_timeout = Duration::from_secs(5); // Short timeout for ping

        let mut stream = timeout(
            connect_timeout,
            TcpStream::connect(self.config.clamd_addr()),
        )
        .await
        .map_err(|_| VirusScanError::ConnectionTimeout)?
        .map_err(|e| VirusScanError::ConnectionError(e.to_string()))?;

        timeout(op_timeout, stream.write_all(b"zPING\0"))
            .await
            .map_err(|_| VirusScanError::OperationTimeout)??;

        let mut response = vec![0u8; 64];
        let n = timeout(op_timeout, stream.read(&mut response))
            .await
            .map_err(|_| VirusScanError::OperationTimeout)??;

        let response_str = String::from_utf8_lossy(&response[..n]);
        Ok(response_str.trim().trim_end_matches('\0') == "PONG")
    }

    /// Get ClamAV version
    pub async fn version(&self) -> Result<String, VirusScanError> {
        let connect_timeout = self.connect_timeout();
        let op_timeout = Duration::from_secs(5); // Short timeout for version

        let mut stream = timeout(
            connect_timeout,
            TcpStream::connect(self.config.clamd_addr()),
        )
        .await
        .map_err(|_| VirusScanError::ConnectionTimeout)?
        .map_err(|e| VirusScanError::ConnectionError(e.to_string()))?;

        timeout(op_timeout, stream.write_all(b"zVERSION\0"))
            .await
            .map_err(|_| VirusScanError::OperationTimeout)??;

        let mut response = vec![0u8; 256];
        let n = timeout(op_timeout, stream.read(&mut response))
            .await
            .map_err(|_| VirusScanError::OperationTimeout)??;

        let version = String::from_utf8_lossy(&response[..n])
            .trim()
            .trim_end_matches('\0')
            .to_string();
        Ok(version)
    }

    /// Scan file data using INSTREAM command
    pub async fn scan_bytes(&self, data: &[u8]) -> Result<ScanResult, VirusScanError> {
        let start = Instant::now();
        let connect_timeout = self.connect_timeout();
        let op_timeout = self.operation_timeout();

        let mut stream = timeout(
            connect_timeout,
            TcpStream::connect(self.config.clamd_addr()),
        )
        .await
        .map_err(|_| VirusScanError::ConnectionTimeout)?
        .map_err(|e| VirusScanError::ConnectionError(e.to_string()))?;

        // Send INSTREAM command
        timeout(op_timeout, stream.write_all(b"zINSTREAM\0"))
            .await
            .map_err(|_| VirusScanError::OperationTimeout)??;

        // Send data in chunks with length prefix
        const CHUNK_SIZE: usize = 2048;
        for chunk in data.chunks(CHUNK_SIZE) {
            let len = (chunk.len() as u32).to_be_bytes();
            timeout(op_timeout, stream.write_all(&len))
                .await
                .map_err(|_| VirusScanError::OperationTimeout)??;
            timeout(op_timeout, stream.write_all(chunk))
                .await
                .map_err(|_| VirusScanError::OperationTimeout)??;
        }

        // Send zero-length chunk to signal end
        timeout(op_timeout, stream.write_all(&[0u8; 4]))
            .await
            .map_err(|_| VirusScanError::OperationTimeout)??;

        // Read response
        let mut response = vec![0u8; 1024];
        let n = timeout(op_timeout, stream.read(&mut response))
            .await
            .map_err(|_| VirusScanError::OperationTimeout)??;

        let duration_ms = start.elapsed().as_millis() as u64;
        let response_str = String::from_utf8_lossy(&response[..n])
            .trim()
            .trim_end_matches('\0')
            .to_string();

        // Parse response: "stream: OK" or "stream: VirusName FOUND"
        let (is_infected, threat_name) = if response_str.ends_with("OK") {
            (false, None)
        } else if response_str.contains("FOUND") {
            // Extract virus name from "stream: VirusName FOUND"
            let parts: Vec<&str> = response_str.split(':').collect();
            if parts.len() >= 2 {
                let threat = parts[1].trim().trim_end_matches(" FOUND").to_string();
                (true, Some(threat))
            } else {
                (true, Some("Unknown".to_string()))
            }
        } else if response_str.contains("ERROR") {
            return Err(VirusScanError::ScanError(response_str));
        } else {
            (false, None)
        };

        Ok(ScanResult {
            is_infected,
            threat_name,
            scan_duration_ms: duration_ms,
            scanner_version: None,
        })
    }
}

// =============================================================================
// Job Queue
// =============================================================================

/// Virus scan job record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJob {
    pub id: Uuid,
    pub file_id: Uuid,
    pub tenant_id: Uuid,
    pub status: String,
    pub priority: i32,
    pub retry_count: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Enqueue a virus scan job for a file
///
/// If `max_queue_size` is provided and > 0, will reject with QueueFull error
/// if the pending queue exceeds that limit.
pub async fn enqueue_scan(
    store: &clovalink_entity::DataStore,
    file_id: Uuid,
    tenant_id: Uuid,
    priority: i32,
) -> Result<Uuid, VirusScanError> {
    enqueue_scan_with_backpressure(store, file_id, tenant_id, priority, 0).await
}

/// Enqueue a virus scan job with backpressure control
///
/// If `max_queue_size` > 0, will reject with QueueFull error if the pending
/// queue exceeds that limit. Set to 0 to disable backpressure.
pub async fn enqueue_scan_with_backpressure(
    store: &clovalink_entity::DataStore,
    file_id: Uuid,
    tenant_id: Uuid,
    priority: i32,
    max_queue_size: i64,
) -> Result<Uuid, VirusScanError> {
    // Check queue size if backpressure is enabled
    if max_queue_size > 0 {
        let queue_size=store.virus_scan().queue_size().await?;

        if queue_size >= max_queue_size {
            warn!(
                target: "virus_scan",
                queue_size = queue_size,
                max_queue_size = max_queue_size,
                file_id = %file_id,
                "Virus scan queue full, rejecting job"
            );
            return Err(VirusScanError::QueueFull);
        }
    }

    let result=store.virus_scan().enqueue(file_id,tenant_id,priority).await?;

    debug!(
        target: "virus_scan",
        job_id = %result,
        file_id = %file_id,
        tenant_id = %tenant_id,
        "Enqueued virus scan job"
    );

    Ok(result)
}

/// Fetch the next pending scan job
pub async fn fetch_next_job(store: &clovalink_entity::DataStore) -> Result<Option<ScanJob>, VirusScanError> {
    Ok(store.virus_scan().fetch_next().await?.map(|m|ScanJob{id:m.id,file_id:m.file_id,tenant_id:m.tenant_id,status:m.status,priority:m.priority,retry_count:m.retry_count,last_attempt_at:m.last_attempt_at.map(Into::into),next_retry_at:m.next_retry_at.map(Into::into),error_message:m.error_message,created_at:m.created_at.into(),updated_at:m.updated_at.into()}))
}

/// Mark a scan job as completed
pub async fn complete_job(store: &clovalink_entity::DataStore, job_id: Uuid) -> Result<(), VirusScanError> {
    store.virus_scan().set_job_status(job_id,"completed",None).await?;
    Ok(())
}

/// Mark a scan job as skipped (file too large, wrong type, etc.)
pub async fn skip_job(store: &clovalink_entity::DataStore, job_id: Uuid, reason: &str) -> Result<(), VirusScanError> {
    store.virus_scan().set_job_status(job_id,"skipped",Some(reason)).await?;
    Ok(())
}

/// Calculate exponential backoff delay in seconds based on retry count
fn calculate_backoff_delay(retry_count: i32) -> i64 {
    match retry_count {
        0 => 30,  // 30 seconds
        1 => 120, // 2 minutes
        2 => 600, // 10 minutes
        _ => 600, // Cap at 10 minutes
    }
}

/// Mark a scan job as failed with exponential backoff retry
pub async fn fail_job(store: &clovalink_entity::DataStore, job_id: Uuid, error: &str) -> Result<(), VirusScanError> {
    // Get current retry count to calculate backoff
    let retry_count=store.virus_scan().job(job_id).await?.map(|j|j.retry_count);

    let current_retry = retry_count.unwrap_or(0);
    let backoff_secs = calculate_backoff_delay(current_retry);

    store.virus_scan().fail_job(job_id,error,backoff_secs).await?;

    info!(
        target: "virus_scan",
        job_id = %job_id,
        retry_count = current_retry + 1,
        backoff_secs = backoff_secs,
        "Job failed, scheduled retry with exponential backoff"
    );

    Ok(())
}

/// Requeue a job for later processing (circuit breaker open, no retry count increment)
pub async fn requeue_job(store: &clovalink_entity::DataStore, job_id: Uuid, reason: &str) -> Result<(), VirusScanError> {
    store.virus_scan().set_job_status(job_id,"pending",Some(reason)).await?;

    debug!(
        target: "virus_scan",
        job_id = %job_id,
        reason = reason,
        "Job requeued for later processing"
    );

    Ok(())
}

// =============================================================================
// Scan Results
// =============================================================================

/// Record a scan result
pub async fn record_scan_result(
    store: &clovalink_entity::DataStore,
    file_id: Uuid,
    tenant_id: Uuid,
    job_id: Option<Uuid>,
    is_infected: bool,
    threat_name: Option<&str>,
    file_size_bytes: i64,
    scan_duration_ms: i32,
    scanner_version: Option<&str>,
    signature_version: Option<&str>,
    action_taken: Option<&str>,
) -> Result<Uuid, VirusScanError> {
    Ok(store.virus_scan().record_result(clovalink_entity::repositories::NewVirusScanResult{file_id,tenant_id,job_id,infected:is_infected,threat:threat_name,size:file_size_bytes,duration:scan_duration_ms,scanner:scanner_version,signature:signature_version,action:action_taken}).await?)
}

/// Update file scan status
pub async fn update_file_scan_status(
    store: &clovalink_entity::DataStore,
    file_id: Uuid,
    status: &str,
) -> Result<(), VirusScanError> {
    store.virus_scan().set_file_scan_status(file_id,status).await?;
    Ok(())
}

/// Check user's malware count and suspend if threshold reached
pub async fn check_and_suspend_uploader(
    store: &clovalink_entity::DataStore,
    user_id: Uuid,
    tenant_id: Uuid,
    threshold: i32,
    file_id: Uuid,
    file_name: &str,
    threat_name: &str,
) -> Result<bool, VirusScanError> {
    let offense_count=store.virus_scan().record_offense(user_id,tenant_id).await?;
    info!(
        target: "virus_scan",
        user_id = %user_id,
        offense_count = offense_count,
        threshold = threshold,
        "User malware offense count updated"
    );

    // Check if threshold is reached
    if offense_count >= threshold {
        // Suspend the user
        store.virus_scan().suspend_user(user_id,format!(
            "Auto-suspended: Uploaded {} infected file(s). Last: {} infected with {}",
            offense_count, file_name, threat_name
        )).await?;

        // Create security alert for suspension
        if let Err(e) = security_service::alert_user_suspended_malware(
            store,
            tenant_id,
            user_id,
            offense_count,
            file_id,
            file_name,
            threat_name,
        )
        .await
        {
            error!(
                target: "virus_scan",
                user_id = %user_id,
                error = %e,
                "Failed to create security alert for auto-suspension"
            );
        }

        warn!(
            target: "virus_scan",
            user_id = %user_id,
            offense_count = offense_count,
            threshold = threshold,
            "User auto-suspended for uploading malware"
        );

        return Ok(true);
    }

    Ok(false)
}

// =============================================================================
// Background Worker
// =============================================================================

/// Trait for file storage access (implemented by storage backend)
#[async_trait::async_trait]
pub trait FileStorageReader: Send + Sync {
    async fn download(
        &self,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Virus scan worker that processes jobs in the background
pub struct VirusScanWorker {
    store: clovalink_entity::DataStore,
    config: VirusScanConfig,
    client: ClamAvClient,
    storage: Arc<dyn FileStorageReader>,
    worker_id: u32,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl VirusScanWorker {
    /// Create a new virus scan worker
    pub fn new(
        store: clovalink_entity::DataStore,
        config: VirusScanConfig,
        storage: Arc<dyn FileStorageReader>,
        worker_id: u32,
        circuit_breaker: Arc<CircuitBreaker>,
    ) -> Self {
        let client = ClamAvClient::new(config.clone());
        Self {
            store,
            config,
            client,
            storage,
            worker_id,
            circuit_breaker,
        }
    }

    /// Create a new virus scan worker with default circuit breaker
    /// (5 failures to open, 30s recovery, 3 successes to close)
    pub fn with_default_circuit_breaker(
        store: clovalink_entity::DataStore,
        config: VirusScanConfig,
        storage: Arc<dyn FileStorageReader>,
        worker_id: u32,
    ) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(
            format!("clamav-worker-{}", worker_id),
            5,  // failure threshold
            30, // recovery timeout seconds
            3,  // success threshold to close
        ));
        Self::new(store, config, storage, worker_id, circuit_breaker)
    }

    /// Run the worker loop
    pub async fn run(self) {
        if !self.config.enabled {
            info!(
                target: "virus_scan",
                worker_id = self.worker_id,
                "Virus scan worker disabled, exiting"
            );
            return;
        }

        info!(
            target: "virus_scan",
            worker_id = self.worker_id,
            clamd_addr = %self.config.clamd_addr(),
            "Virus scan worker started"
        );

        // Wait for clamd to be available
        loop {
            match self.client.ping().await {
                Ok(true) => {
                    info!(
                        target: "virus_scan",
                        worker_id = self.worker_id,
                        "Connected to ClamAV daemon"
                    );
                    break;
                }
                Ok(false) | Err(_) => {
                    warn!(
                        target: "virus_scan",
                        worker_id = self.worker_id,
                        "Waiting for ClamAV daemon..."
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }

        loop {
            match self.process_next_job().await {
                Ok(true) => {
                    // Job processed, immediately check for more
                    continue;
                }
                Ok(false) => {
                    // No jobs available, wait before polling again
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                Err(e) => {
                    error!(
                        target: "virus_scan",
                        worker_id = self.worker_id,
                        error = %e,
                        "Worker error, sleeping before retry"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
            }
        }
    }

    /// Process the next available job
    async fn process_next_job(&self) -> Result<bool, VirusScanError> {
        let job = match fetch_next_job(&self.store).await? {
            Some(job) => job,
            None => return Ok(false),
        };

        info!(
            target: "virus_scan",
            worker_id = self.worker_id,
            job_id = %job.id,
            file_id = %job.file_id,
            "Processing virus scan job"
        );

        // Get tenant settings
        let settings = get_tenant_settings(&self.store, job.tenant_id).await?;

        // Check if scanning is enabled for this tenant
        if !settings.enabled {
            skip_job(&self.store, job.id, "Scanning disabled for tenant").await?;
            update_file_scan_status(&self.store, job.file_id, "skipped").await?;
            return Ok(true);
        }

        // Get file info
        let file_info = self.store.virus_scan().file(job.file_id).await?
            .map(|f|(f.storage_path,f.size_bytes,f.content_type));

        let (storage_path, file_size, _content_type) = match file_info {
            Some(info) => info,
            None => {
                skip_job(&self.store, job.id, "File not found").await?;
                return Ok(true);
            }
        };

        // Check file size limit
        let max_size_bytes = (settings.max_file_size_mb as i64) * 1024 * 1024;
        if file_size > max_size_bytes {
            skip_job(
                &self.store,
                job.id,
                &format!(
                    "File size {} exceeds limit {} MB",
                    file_size, settings.max_file_size_mb
                ),
            )
            .await?;
            update_file_scan_status(&self.store, job.file_id, "skipped").await?;
            return Ok(true);
        }

        // Check file type filter
        if !settings.file_types.is_empty() {
            let ext = storage_path.rsplit('.').next().unwrap_or("").to_lowercase();
            if !settings.file_types.iter().any(|t| t.to_lowercase() == ext) {
                skip_job(
                    &self.store,
                    job.id,
                    &format!("File type '{}' not in scan list", ext),
                )
                .await?;
                update_file_scan_status(&self.store, job.file_id, "skipped").await?;
                return Ok(true);
            }
        }

        // Check circuit breaker before proceeding
        if !self.circuit_breaker.allow_request() {
            warn!(
                target: "virus_scan",
                worker_id = self.worker_id,
                job_id = %job.id,
                "Circuit breaker is open, requeuing job"
            );
            requeue_job(
                &self.store,
                job.id,
                "Circuit breaker open - ClamAV unavailable",
            )
            .await?;
            // Sleep a bit before the next iteration to avoid spinning
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            return Ok(true);
        }

        // Download file for scanning
        let file_data = match self.storage.download(&storage_path).await {
            Ok(data) => data,
            Err(e) => {
                fail_job(&self.store, job.id, &format!("Download failed: {}", e)).await?;
                return Ok(true);
            }
        };

        // Scan the file with circuit breaker protection
        let scan_result = match self.client.scan_bytes(&file_data).await {
            Ok(result) => {
                self.circuit_breaker.record_success();
                result
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                fail_job(&self.store, job.id, &format!("Scan failed: {}", e)).await?;
                update_file_scan_status(&self.store, job.file_id, "error").await?;
                return Ok(true);
            }
        };

        // Get scanner version for metrics
        let scanner_version = self.client.version().await.ok();

        // Determine action
        let action_taken = if scan_result.is_infected {
            let action: DetectionAction = settings
                .action_on_detect
                .parse()
                .unwrap_or(DetectionAction::Quarantine);

            match action {
                DetectionAction::Delete => {
                    // Delete the infected file
                    if let Err(e) = self.storage.delete(&storage_path).await {
                        error!(
                            target: "virus_scan",
                            file_id = %job.file_id,
                            error = %e,
                            "Failed to delete infected file"
                        );
                    }
                    // Mark file as deleted in database
                    self.store.virus_scan().mark_file_deleted(job.file_id).await?;
                    Some("deleted")
                }
                DetectionAction::Quarantine => {
                    // Record in quarantine table with file size and owner
                    self.store.virus_scan().quarantine(job.file_id,scan_result.threat_name.as_deref().unwrap_or("Unknown")).await?;
                    Some("quarantined")
                }
                DetectionAction::Flag => {
                    // Just update scan status, don't delete
                    Some("flagged")
                }
            }
        } else {
            None
        };

        // Record scan result
        record_scan_result(
            &self.store,
            job.file_id,
            job.tenant_id,
            Some(job.id),
            scan_result.is_infected,
            scan_result.threat_name.as_deref(),
            file_size,
            scan_result.scan_duration_ms as i32,
            scanner_version.as_deref(),
            None,
            action_taken,
        )
        .await?;

        // Update file scan status
        let status = if scan_result.is_infected {
            "infected"
        } else {
            "clean"
        };
        update_file_scan_status(&self.store, job.file_id, status).await?;

        // Complete the job
        complete_job(&self.store, job.id).await?;

        if scan_result.is_infected {
            warn!(
                target: "virus_scan",
                worker_id = self.worker_id,
                job_id = %job.id,
                file_id = %job.file_id,
                threat_name = ?scan_result.threat_name,
                action = ?action_taken,
                duration_ms = scan_result.scan_duration_ms,
                "Virus detected!"
            );

            // Send security alert and notifications
            let threat = scan_result.threat_name.as_deref().unwrap_or("Unknown");
            let action_str = action_taken.unwrap_or("flagged");

            // Get file info for notifications
            let file_info=self.store.virus_scan().notification_file(job.file_id).await.ok().flatten();

            if let Some((file_name, uploader_id, uploader_email, uploader_role)) = file_info {
                // Create security alert
                if let Err(e) = security_service::alert_malware_detected(
                    &self.store,
                    job.tenant_id,
                    uploader_id,
                    job.file_id,
                    &file_name,
                    threat,
                    action_str,
                    uploader_email.as_deref(),
                )
                .await
                {
                    error!(
                        target: "virus_scan",
                        file_id = %job.file_id,
                        error = %e,
                        "Failed to create security alert for malware detection"
                    );
                }

                // Send notifications if configured
                if settings.notify_admin || settings.notify_uploader {
                    // Get tenant for notifications
                    let tenant=self.store.security().tenant(job.tenant_id).await.ok().flatten().map(tenant_from_entity);

                    if let Some(tenant) = tenant {
                        if let Err(e) = notification_service::notify_malware_detection(
                            &self.store,
                            &tenant,
                            job.file_id,
                            &file_name,
                            threat,
                            action_str,
                            uploader_id,
                            uploader_email.as_deref(),
                            uploader_role.as_deref(),
                            settings.notify_admin,
                            settings.notify_uploader,
                        )
                        .await
                        {
                            error!(
                                target: "virus_scan",
                                file_id = %job.file_id,
                                error = %e,
                                "Failed to send malware detection notifications"
                            );
                        }
                    }
                }

                // Auto-suspend uploader if enabled (skip for admins)
                if settings.auto_suspend_uploader {
                    if let Some(user_id) = uploader_id {
                        let is_admin = uploader_role
                            .as_deref()
                            .map(|r| r == "Admin" || r == "SuperAdmin")
                            .unwrap_or(false);

                        if !is_admin {
                            // Increment malware count and check threshold
                            if let Err(e) = check_and_suspend_uploader(
                                &self.store,
                                user_id,
                                job.tenant_id,
                                settings.suspend_threshold,
                                job.file_id,
                                &file_name,
                                threat,
                            )
                            .await
                            {
                                error!(
                                    target: "virus_scan",
                                    user_id = %user_id,
                                    error = %e,
                                    "Failed to check/suspend uploader after malware detection"
                                );
                            }
                        } else {
                            info!(
                                target: "virus_scan",
                                user_id = %user_id,
                                role = ?uploader_role,
                                "Skipping auto-suspend for admin user"
                            );
                        }
                    }
                }
            }
        } else {
            info!(
                target: "virus_scan",
                worker_id = self.worker_id,
                job_id = %job.id,
                file_id = %job.file_id,
                duration_ms = scan_result.scan_duration_ms,
                "File scanned - clean"
            );
        }

        Ok(true)
    }
}

// =============================================================================
// Metrics & Status
// =============================================================================

/// Virus scan metrics for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct ScanMetrics {
    pub enabled: bool,
    pub clamd_connected: bool,
    pub clamd_version: Option<String>,
    pub pending_jobs: i64,
    pub scanning_jobs: i64,
    pub failed_jobs: i64,
    pub scans_last_hour: i64,
    pub infections_last_hour: i64,
    pub avg_scan_duration_ms: Option<f64>,
    pub total_bytes_scanned_last_hour: i64,
    // Queue size for backpressure monitoring
    pub queue_size: i64,
    pub max_queue_size: i64,
    // Circuit breaker state
    pub circuit_breaker_state: String,
    pub circuit_breaker_failures: u32,
}

/// Get virus scan metrics for admin dashboard
pub async fn get_metrics(
    store: &clovalink_entity::DataStore,
    config: &VirusScanConfig,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<ScanMetrics, VirusScanError> {
    let client = ClamAvClient::new(config.clone());
    let clamd_connected = client.ping().await.unwrap_or(false);
    let clamd_version = if clamd_connected {
        client.version().await.ok()
    } else {
        None
    };

    let stats=store.virus_scan().metrics().await?;

    // Get circuit breaker state
    let (cb_state, cb_failures) = if let Some(cb) = circuit_breaker {
        let metrics = cb.metrics();
        let state_str = match metrics.state {
            crate::circuit_breaker::CircuitState::Closed => "closed",
            crate::circuit_breaker::CircuitState::Open => "open",
            crate::circuit_breaker::CircuitState::HalfOpen => "half_open",
        };
        (state_str.to_string(), metrics.failure_count)
    } else {
        ("unknown".to_string(), 0)
    };

    let pending = stats.pending;
    let scanning = stats.scanning;

    Ok(ScanMetrics {
        enabled: config.enabled,
        clamd_connected,
        clamd_version,
        pending_jobs: pending,
        scanning_jobs: scanning,
        failed_jobs: stats.failed,
        scans_last_hour: stats.scans,
        infections_last_hour: stats.infections,
        avg_scan_duration_ms: stats.average,
        total_bytes_scanned_last_hour: stats.bytes,
        queue_size: pending + scanning,
        max_queue_size: config.max_queue_size,
        circuit_breaker_state: cb_state,
        circuit_breaker_failures: cb_failures,
    })
}

/// Paginated scan history response
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanHistoryResponse {
    pub items: Vec<serde_json::Value>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Get scan history for a tenant with pagination
pub async fn get_scan_history(
    store: &clovalink_entity::DataStore,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
    infected_only: bool,
) -> Result<ScanHistoryResponse, VirusScanError> {
    let (items,total)=store.virus_scan().history(tenant_id,limit.max(0) as u64,offset.max(0) as u64,infected_only).await?;

    Ok(ScanHistoryResponse {
        items,
        total,
        limit,
        offset,
    })
}

/// Quarantined file response with uploader info
#[derive(Debug, Serialize, Deserialize)]
pub struct QuarantinedFileResponse {
    pub id: Uuid,
    pub file_id: Uuid,
    pub file_name: String,
    pub original_path: String,
    pub threat_name: String,
    pub original_size: i64,
    pub quarantined_at: DateTime<Utc>,
    pub uploader_id: Option<Uuid>,
    pub uploader_name: Option<String>,
    pub uploader_email: Option<String>,
}

/// Paginated quarantine response
#[derive(Debug, Serialize, Deserialize)]
pub struct QuarantineListResponse {
    pub items: Vec<QuarantinedFileResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Get quarantined files for a tenant with uploader info and pagination
pub async fn get_quarantined_files(
    store: &clovalink_entity::DataStore,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<QuarantineListResponse, VirusScanError> {
    let (results,total)=store.virus_scan().quarantined(tenant_id,limit.max(0) as u64,offset.max(0) as u64).await?;

    let items: Vec<QuarantinedFileResponse> = results
        .into_iter()
        .map(
            |row| {
                QuarantinedFileResponse {
                    id:row.model.id,file_id:row.model.original_file_id,file_name:row.model.original_filename,
                    original_path:row.model.original_path,threat_name:row.model.threat_name,
                    original_size:row.model.file_size_bytes.unwrap_or(0),quarantined_at:row.model.quarantined_at.into(),
                    uploader_id:row.model.owner_id,uploader_name:row.owner_name,uploader_email:row.owner_email,
                }
            },
        )
        .collect();

    Ok(QuarantineListResponse {
        items,
        total,
        limit,
        offset,
    })
}
