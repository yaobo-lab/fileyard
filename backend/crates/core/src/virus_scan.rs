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
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::circuit_breaker::CircuitBreaker;
use crate::models::Tenant;
use crate::security_service;
use crate::notification_service;

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
    DatabaseError(#[from] sqlx::Error),
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
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("CLAMAV_ENABLED")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
            host: std::env::var("CLAMAV_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("CLAMAV_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3310),
            timeout_ms: std::env::var("CLAMAV_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30000),
            workers: std::env::var("CLAMAV_WORKERS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
            max_file_size_mb: std::env::var("CLAMAV_MAX_FILE_SIZE_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            max_queue_size: std::env::var("CLAMAV_MAX_QUEUE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000), // Default 10k pending jobs max
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<TenantScanSettings, VirusScanError> {
    let settings = sqlx::query_as::<_, TenantScanSettings>(
        r#"
        SELECT id, tenant_id, enabled, file_types, max_file_size_mb,
               action_on_detect, notify_admin, notify_uploader,
               auto_suspend_uploader, suspend_threshold, created_at, updated_at
        FROM virus_scan_settings
        WHERE tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;

    match settings {
        Some(s) => Ok(s),
        None => {
            // Create default settings for this tenant
            let new_settings = sqlx::query_as::<_, TenantScanSettings>(
                r#"
                INSERT INTO virus_scan_settings (tenant_id)
                VALUES ($1)
                ON CONFLICT (tenant_id) DO NOTHING
                RETURNING id, tenant_id, enabled, file_types, max_file_size_mb,
                          action_on_detect, notify_admin, notify_uploader,
                          auto_suspend_uploader, suspend_threshold, created_at, updated_at
                "#,
            )
            .bind(tenant_id)
            .fetch_optional(pool)
            .await?;

            match new_settings {
                Some(s) => Ok(s),
                None => {
                    // Race condition - fetch again
                    sqlx::query_as::<_, TenantScanSettings>(
                        r#"
                        SELECT id, tenant_id, enabled, file_types, max_file_size_mb,
                               action_on_detect, notify_admin, notify_uploader,
                               auto_suspend_uploader, suspend_threshold, created_at, updated_at
                        FROM virus_scan_settings
                        WHERE tenant_id = $1
                        "#,
                    )
                    .bind(tenant_id)
                    .fetch_one(pool)
                    .await
                    .map_err(VirusScanError::from)
                }
            }
        }
    }
}

/// Update tenant scan settings
pub async fn update_tenant_settings(
    pool: &PgPool,
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
    // Ensure settings exist first
    get_tenant_settings(pool, tenant_id).await?;

    let settings = sqlx::query_as::<_, TenantScanSettings>(
        r#"
        UPDATE virus_scan_settings
        SET 
            enabled = COALESCE($2, enabled),
            file_types = COALESCE($3, file_types),
            max_file_size_mb = COALESCE($4, max_file_size_mb),
            action_on_detect = COALESCE($5, action_on_detect),
            notify_admin = COALESCE($6, notify_admin),
            notify_uploader = COALESCE($7, notify_uploader),
            auto_suspend_uploader = COALESCE($8, auto_suspend_uploader),
            suspend_threshold = COALESCE($9, suspend_threshold),
            updated_at = NOW()
        WHERE tenant_id = $1
        RETURNING id, tenant_id, enabled, file_types, max_file_size_mb,
                  action_on_detect, notify_admin, notify_uploader,
                  auto_suspend_uploader, suspend_threshold, created_at, updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(enabled)
    .bind(file_types)
    .bind(max_file_size_mb)
    .bind(action_on_detect)
    .bind(notify_admin)
    .bind(notify_uploader)
    .bind(auto_suspend_uploader)
    .bind(suspend_threshold)
    .fetch_one(pool)
    .await?;

    Ok(settings)
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

        let mut stream = timeout(connect_timeout, TcpStream::connect(self.config.clamd_addr()))
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

        let mut stream = timeout(connect_timeout, TcpStream::connect(self.config.clamd_addr()))
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

        let mut stream = timeout(connect_timeout, TcpStream::connect(self.config.clamd_addr()))
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
    pool: &PgPool,
    file_id: Uuid,
    tenant_id: Uuid,
    priority: i32,
) -> Result<Uuid, VirusScanError> {
    enqueue_scan_with_backpressure(pool, file_id, tenant_id, priority, 0).await
}

/// Enqueue a virus scan job with backpressure control
/// 
/// If `max_queue_size` > 0, will reject with QueueFull error if the pending
/// queue exceeds that limit. Set to 0 to disable backpressure.
pub async fn enqueue_scan_with_backpressure(
    pool: &PgPool,
    file_id: Uuid,
    tenant_id: Uuid,
    priority: i32,
    max_queue_size: i64,
) -> Result<Uuid, VirusScanError> {
    // Check queue size if backpressure is enabled
    if max_queue_size > 0 {
        let queue_size: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM virus_scan_jobs WHERE status IN ('pending', 'scanning')"
        )
        .fetch_one(pool)
        .await?;

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

    let job_id = Uuid::new_v4();

    let result = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO virus_scan_jobs (id, file_id, tenant_id, priority)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (file_id) WHERE status IN ('pending', 'scanning')
        DO UPDATE SET priority = GREATEST(virus_scan_jobs.priority, EXCLUDED.priority)
        RETURNING id
        "#,
    )
    .bind(job_id)
    .bind(file_id)
    .bind(tenant_id)
    .bind(priority)
    .fetch_one(pool)
    .await?;

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
pub async fn fetch_next_job(pool: &PgPool) -> Result<Option<ScanJob>, VirusScanError> {
    let job = sqlx::query_as::<_, ScanJob>(
        r#"
        UPDATE virus_scan_jobs
        SET status = 'scanning', last_attempt_at = NOW(), updated_at = NOW()
        WHERE id = (
            SELECT id FROM virus_scan_jobs
            WHERE status = 'pending'
              AND (next_retry_at IS NULL OR next_retry_at <= NOW())
            ORDER BY priority DESC, created_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, file_id, tenant_id, status, priority, retry_count,
                  last_attempt_at, next_retry_at, error_message, created_at, updated_at
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(job)
}

/// Mark a scan job as completed
pub async fn complete_job(pool: &PgPool, job_id: Uuid) -> Result<(), VirusScanError> {
    sqlx::query(
        r#"
        UPDATE virus_scan_jobs
        SET status = 'completed', error_message = NULL, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Mark a scan job as skipped (file too large, wrong type, etc.)
pub async fn skip_job(pool: &PgPool, job_id: Uuid, reason: &str) -> Result<(), VirusScanError> {
    sqlx::query(
        r#"
        UPDATE virus_scan_jobs
        SET status = 'skipped', error_message = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(reason)
    .execute(pool)
    .await?;

    Ok(())
}

/// Calculate exponential backoff delay in seconds based on retry count
fn calculate_backoff_delay(retry_count: i32) -> i64 {
    match retry_count {
        0 => 30,     // 30 seconds
        1 => 120,    // 2 minutes
        2 => 600,    // 10 minutes
        _ => 600,    // Cap at 10 minutes
    }
}

/// Mark a scan job as failed with exponential backoff retry
pub async fn fail_job(pool: &PgPool, job_id: Uuid, error: &str) -> Result<(), VirusScanError> {
    // Get current retry count to calculate backoff
    let retry_count: Option<i32> = sqlx::query_scalar(
        "SELECT retry_count FROM virus_scan_jobs WHERE id = $1"
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await?;

    let current_retry = retry_count.unwrap_or(0);
    let backoff_secs = calculate_backoff_delay(current_retry);

    sqlx::query(
        r#"
        UPDATE virus_scan_jobs
        SET 
            retry_count = retry_count + 1,
            error_message = $2,
            status = CASE 
                WHEN retry_count + 1 >= 3 THEN 'failed'
                ELSE 'pending'
            END,
            next_retry_at = CASE 
                WHEN retry_count + 1 < 3 THEN NOW() + ($3 || ' seconds')::interval
                ELSE NULL
            END,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(error)
    .bind(backoff_secs.to_string())
    .execute(pool)
    .await?;

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
pub async fn requeue_job(pool: &PgPool, job_id: Uuid, reason: &str) -> Result<(), VirusScanError> {
    sqlx::query(
        r#"
        UPDATE virus_scan_jobs
        SET 
            status = 'pending',
            error_message = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .bind(reason)
    .execute(pool)
    .await?;

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
    pool: &PgPool,
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
    let result_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO virus_scan_results (
            id, file_id, tenant_id, scan_job_id, is_infected, threat_name,
            file_size_bytes, scan_duration_ms, scanner_version, signature_version, action_taken
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(result_id)
    .bind(file_id)
    .bind(tenant_id)
    .bind(job_id)
    .bind(is_infected)
    .bind(threat_name)
    .bind(file_size_bytes)
    .bind(scan_duration_ms)
    .bind(scanner_version)
    .bind(signature_version)
    .bind(action_taken)
    .execute(pool)
    .await?;

    Ok(result_id)
}

/// Update file scan status
pub async fn update_file_scan_status(
    pool: &PgPool,
    file_id: Uuid,
    status: &str,
) -> Result<(), VirusScanError> {
    sqlx::query(
        r#"
        UPDATE files_metadata SET scan_status = $2 WHERE id = $1
        "#,
    )
    .bind(file_id)
    .bind(status)
    .execute(pool)
    .await?;

    Ok(())
}

/// Check user's malware count and suspend if threshold reached
pub async fn check_and_suspend_uploader(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    threshold: i32,
    file_id: Uuid,
    file_name: &str,
    threat_name: &str,
) -> Result<bool, VirusScanError> {
    // Increment malware count (upsert)
    let count_result: (i32,) = sqlx::query_as(
        r#"
        INSERT INTO user_malware_counts (user_id, tenant_id, count, last_offense_at)
        VALUES ($1, $2, 1, NOW())
        ON CONFLICT (user_id, tenant_id)
        DO UPDATE SET 
            count = user_malware_counts.count + 1,
            last_offense_at = NOW(),
            updated_at = NOW()
        RETURNING count
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;

    let offense_count = count_result.0;
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
        sqlx::query(
            r#"
            UPDATE users 
            SET is_suspended = true, 
                suspension_reason = $2,
                suspended_at = NOW()
            WHERE id = $1 AND is_suspended = false
            "#,
        )
        .bind(user_id)
        .bind(format!(
            "Auto-suspended: Uploaded {} infected file(s). Last: {} infected with {}",
            offense_count, file_name, threat_name
        ))
        .execute(pool)
        .await?;

        // Create security alert for suspension
        if let Err(e) = security_service::alert_user_suspended_malware(
            pool,
            tenant_id,
            user_id,
            offense_count,
            file_id,
            file_name,
            threat_name,
        ).await {
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
    async fn download(&self, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Virus scan worker that processes jobs in the background
pub struct VirusScanWorker {
    pool: PgPool,
    config: VirusScanConfig,
    client: ClamAvClient,
    storage: Arc<dyn FileStorageReader>,
    worker_id: u32,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl VirusScanWorker {
    /// Create a new virus scan worker
    pub fn new(
        pool: PgPool,
        config: VirusScanConfig,
        storage: Arc<dyn FileStorageReader>,
        worker_id: u32,
        circuit_breaker: Arc<CircuitBreaker>,
    ) -> Self {
        let client = ClamAvClient::new(config.clone());
        Self {
            pool,
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
        pool: PgPool,
        config: VirusScanConfig,
        storage: Arc<dyn FileStorageReader>,
        worker_id: u32,
    ) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(
            format!("clamav-worker-{}", worker_id),
            5,   // failure threshold
            30,  // recovery timeout seconds
            3,   // success threshold to close
        ));
        Self::new(pool, config, storage, worker_id, circuit_breaker)
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
        let job = match fetch_next_job(&self.pool).await? {
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
        let settings = get_tenant_settings(&self.pool, job.tenant_id).await?;

        // Check if scanning is enabled for this tenant
        if !settings.enabled {
            skip_job(&self.pool, job.id, "Scanning disabled for tenant").await?;
            update_file_scan_status(&self.pool, job.file_id, "skipped").await?;
            return Ok(true);
        }

        // Get file info
        let file_info: Option<(String, i64, Option<String>)> = sqlx::query_as(
            r#"
            SELECT storage_path, size_bytes, content_type
            FROM files_metadata WHERE id = $1
            "#,
        )
        .bind(job.file_id)
        .fetch_optional(&self.pool)
        .await?;

        let (storage_path, file_size, _content_type) = match file_info {
            Some(info) => info,
            None => {
                skip_job(&self.pool, job.id, "File not found").await?;
                return Ok(true);
            }
        };

        // Check file size limit
        let max_size_bytes = (settings.max_file_size_mb as i64) * 1024 * 1024;
        if file_size > max_size_bytes {
            skip_job(
                &self.pool,
                job.id,
                &format!(
                    "File size {} exceeds limit {} MB",
                    file_size, settings.max_file_size_mb
                ),
            )
            .await?;
            update_file_scan_status(&self.pool, job.file_id, "skipped").await?;
            return Ok(true);
        }

        // Check file type filter
        if !settings.file_types.is_empty() {
            let ext = storage_path
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_lowercase();
            if !settings.file_types.iter().any(|t| t.to_lowercase() == ext) {
                skip_job(
                    &self.pool,
                    job.id,
                    &format!("File type '{}' not in scan list", ext),
                )
                .await?;
                update_file_scan_status(&self.pool, job.file_id, "skipped").await?;
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
            requeue_job(&self.pool, job.id, "Circuit breaker open - ClamAV unavailable").await?;
            // Sleep a bit before the next iteration to avoid spinning
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            return Ok(true);
        }

        // Download file for scanning
        let file_data = match self.storage.download(&storage_path).await {
            Ok(data) => data,
            Err(e) => {
                fail_job(&self.pool, job.id, &format!("Download failed: {}", e)).await?;
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
                fail_job(&self.pool, job.id, &format!("Scan failed: {}", e)).await?;
                update_file_scan_status(&self.pool, job.file_id, "error").await?;
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
                    sqlx::query("UPDATE files_metadata SET is_deleted = true, deleted_at = NOW() WHERE id = $1")
                        .bind(job.file_id)
                        .execute(&self.pool)
                        .await?;
                    Some("deleted")
                }
                DetectionAction::Quarantine => {
                    // Record in quarantine table with file size and owner
                    sqlx::query(
                        r#"
                        INSERT INTO quarantined_files (
                            original_file_id, tenant_id, original_filename, original_path,
                            storage_path, threat_name, file_size_bytes, owner_id
                        )
                        SELECT $1, tenant_id, name, COALESCE(parent_path, ''), storage_path, $2, size_bytes, owner_id
                        FROM files_metadata WHERE id = $1
                        "#,
                    )
                    .bind(job.file_id)
                    .bind(&scan_result.threat_name)
                    .execute(&self.pool)
                    .await?;
                    // Mark file as quarantined
                    sqlx::query("UPDATE files_metadata SET is_deleted = true, deleted_at = NOW() WHERE id = $1")
                        .bind(job.file_id)
                        .execute(&self.pool)
                        .await?;
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
            &self.pool,
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
        update_file_scan_status(&self.pool, job.file_id, status).await?;

        // Complete the job
        complete_job(&self.pool, job.id).await?;

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
            let file_info: Option<(String, Option<Uuid>, Option<String>, Option<String>)> = sqlx::query_as(
                r#"
                SELECT 
                    fm.name,
                    fm.owner_id,
                    u.email,
                    u.role
                FROM files_metadata fm
                LEFT JOIN users u ON u.id = fm.owner_id
                WHERE fm.id = $1
                "#
            )
            .bind(job.file_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

            if let Some((file_name, uploader_id, uploader_email, uploader_role)) = file_info {
                // Create security alert
                if let Err(e) = security_service::alert_malware_detected(
                    &self.pool,
                    job.tenant_id,
                    uploader_id,
                    job.file_id,
                    &file_name,
                    threat,
                    action_str,
                    uploader_email.as_deref(),
                ).await {
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
                    let tenant: Option<Tenant> = sqlx::query_as(
                        "SELECT * FROM tenants WHERE id = $1"
                    )
                    .bind(job.tenant_id)
                    .fetch_optional(&self.pool)
                    .await
                    .ok()
                    .flatten();

                    if let Some(tenant) = tenant {
                        if let Err(e) = notification_service::notify_malware_detection(
                            &self.pool,
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
                        ).await {
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
                                &self.pool,
                                user_id,
                                job.tenant_id,
                                settings.suspend_threshold,
                                job.file_id,
                                &file_name,
                                threat,
                            ).await {
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
    pool: &PgPool, 
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

    // Get job counts
    let job_stats: (Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'pending'),
            COUNT(*) FILTER (WHERE status = 'scanning'),
            COUNT(*) FILTER (WHERE status = 'failed')
        FROM virus_scan_jobs
        "#,
    )
    .fetch_one(pool)
    .await?;

    // Get scan metrics from last hour
    let scan_stats: (Option<i64>, Option<i64>, Option<f64>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            COUNT(*) FILTER (WHERE is_infected = true),
            AVG(scan_duration_ms)::FLOAT8,
            SUM(file_size_bytes)::BIGINT
        FROM virus_scan_results
        WHERE scanned_at > NOW() - INTERVAL '1 hour'
        "#,
    )
    .fetch_one(pool)
    .await?;

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

    let pending = job_stats.0.unwrap_or(0);
    let scanning = job_stats.1.unwrap_or(0);

    Ok(ScanMetrics {
        enabled: config.enabled,
        clamd_connected,
        clamd_version,
        pending_jobs: pending,
        scanning_jobs: scanning,
        failed_jobs: job_stats.2.unwrap_or(0),
        scans_last_hour: scan_stats.0.unwrap_or(0),
        infections_last_hour: scan_stats.1.unwrap_or(0),
        avg_scan_duration_ms: scan_stats.2,
        total_bytes_scanned_last_hour: scan_stats.3.unwrap_or(0),
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
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
    infected_only: bool,
) -> Result<ScanHistoryResponse, VirusScanError> {
    // Get total count
    let total: (i64,) = if infected_only {
        sqlx::query_as(
            "SELECT COUNT(*) FROM virus_scan_results WHERE tenant_id = $1 AND is_infected = true"
        )
        .bind(tenant_id)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT COUNT(*) FROM virus_scan_results WHERE tenant_id = $1"
        )
        .bind(tenant_id)
        .fetch_one(pool)
        .await?
    };

    let items = if infected_only {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT json_build_object(
                'id', r.id,
                'file_id', r.file_id,
                'file_name', f.name,
                'scan_status', CASE WHEN r.is_infected THEN 'infected' ELSE 'clean' END,
                'threat_name', r.threat_name,
                'file_size_bytes', r.file_size_bytes,
                'scan_duration_ms', r.scan_duration_ms,
                'action_taken', r.action_taken,
                'scanned_at', r.scanned_at
            )
            FROM virus_scan_results r
            LEFT JOIN files_metadata f ON f.id = r.file_id
            WHERE r.tenant_id = $1 AND r.is_infected = true
            ORDER BY r.scanned_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT json_build_object(
                'id', r.id,
                'file_id', r.file_id,
                'file_name', f.name,
                'scan_status', CASE WHEN r.is_infected THEN 'infected' ELSE 'clean' END,
                'threat_name', r.threat_name,
                'file_size_bytes', r.file_size_bytes,
                'scan_duration_ms', r.scan_duration_ms,
                'action_taken', r.action_taken,
                'scanned_at', r.scanned_at
            )
            FROM virus_scan_results r
            LEFT JOIN files_metadata f ON f.id = r.file_id
            WHERE r.tenant_id = $1
            ORDER BY r.scanned_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    Ok(ScanHistoryResponse {
        items,
        total: total.0,
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
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<QuarantineListResponse, VirusScanError> {
    // Get total count
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM quarantined_files WHERE tenant_id = $1 AND permanently_deleted_at IS NULL"
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;

    // Get quarantined files with uploader info
    let results = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Option<i64>, DateTime<Utc>, Option<Uuid>, Option<String>, Option<String>)>(
        r#"
        SELECT 
            qf.id,
            qf.original_file_id,
            qf.original_filename,
            qf.original_path,
            qf.threat_name,
            qf.file_size_bytes,
            qf.quarantined_at,
            qf.owner_id,
            u.name,
            u.email
        FROM quarantined_files qf
        LEFT JOIN users u ON u.id = qf.owner_id
        WHERE qf.tenant_id = $1 AND qf.permanently_deleted_at IS NULL
        ORDER BY qf.quarantined_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let items: Vec<QuarantinedFileResponse> = results
        .into_iter()
        .map(|(id, file_id, file_name, original_path, threat_name, size, quarantined_at, owner_id, owner_name, owner_email)| {
            QuarantinedFileResponse {
                id,
                file_id,
                file_name,
                original_path,
                threat_name,
                original_size: size.unwrap_or(0),
                quarantined_at,
                uploader_id: owner_id,
                uploader_name: owner_name,
                uploader_email: owner_email,
            }
        })
        .collect();

    Ok(QuarantineListResponse {
        items,
        total: total.0,
        limit,
        offset,
    })
}


