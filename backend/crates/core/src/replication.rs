//! S3 Replication Module
//!
//! Provides async replication of uploaded files to a secondary S3-compatible bucket.
//! This is an enterprise durability feature that runs entirely in the background
//! and never blocks uploads.
//!
//! Modes:
//! - `backup`: Replicate uploads only. Deletions do NOT propagate (secondary is an archive).
//! - `mirror`: Keep secondary in sync. Deletions also replicate.

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Replication errors
#[derive(Debug, Error)]
pub enum ReplicationError {
    #[error("Replication is disabled")]
    Disabled,
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] clovalink_entity::DataError),
    #[error("S3 error: {0}")]
    S3Error(String),
    #[error("Source file not found: {0}")]
    SourceNotFound(String),
}

/// Replication mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplicationMode {
    /// Replicate uploads only. Deletions do NOT propagate.
    Backup,
    /// Keep secondary in sync. Deletions also replicate.
    Mirror,
}

impl Default for ReplicationMode {
    fn default() -> Self {
        ReplicationMode::Backup
    }
}

impl std::str::FromStr for ReplicationMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "backup" => Ok(ReplicationMode::Backup),
            "mirror" => Ok(ReplicationMode::Mirror),
            _ => Err(format!(
                "Invalid replication mode: {}. Use 'backup' or 'mirror'",
                s
            )),
        }
    }
}

/// Replication configuration loaded from environment
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub mode: ReplicationMode,
    pub retry_seconds: u64,
    pub workers: u32,
    pub max_retries: u32,
}

impl ReplicationConfig {
    /// Load configuration from the application TOML configuration.
    pub fn from_config() -> Self {
        let source = &types::config::get_config().replication;
        Self {
            enabled: source.enabled,
            endpoint: source.endpoint.clone().filter(|s| !s.is_empty()),
            bucket: source.bucket.clone(),
            region: source.region.clone(),
            access_key: source.access_key.clone(),
            secret_key: source.secret_key.clone(),
            mode: source.mode.parse().unwrap_or_default(),
            retry_seconds: source.retry_seconds,
            workers: source.workers,
            max_retries: source.max_retries,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ReplicationError> {
        if !self.enabled {
            return Ok(());
        }

        if self.bucket.is_empty() {
            return Err(ReplicationError::ConfigError(
                "REPLICATION_BUCKET is required".into(),
            ));
        }
        if self.access_key.is_empty() {
            return Err(ReplicationError::ConfigError(
                "REPLICATION_ACCESS_KEY is required".into(),
            ));
        }
        if self.secret_key.is_empty() {
            return Err(ReplicationError::ConfigError(
                "REPLICATION_SECRET_KEY is required".into(),
            ));
        }

        Ok(())
    }
}

/// Replication job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Replication job operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobOperation {
    Upload,
    Delete,
}

impl std::fmt::Display for JobOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobOperation::Upload => write!(f, "upload"),
            JobOperation::Delete => write!(f, "delete"),
        }
    }
}

/// A replication job record
pub type ReplicationJob = clovalink_entity::entities::replication_jobs::Model;

/// S3 client for replication to secondary bucket
pub struct ReplicationClient {
    client: Client,
    bucket: String,
}

impl ReplicationClient {
    /// Create a new replication client from config
    pub async fn new(config: &ReplicationConfig) -> Result<Self, ReplicationError> {
        config.validate()?;

        if !config.enabled {
            return Err(ReplicationError::Disabled);
        }

        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "replication",
        );

        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(credentials)
            .force_path_style(true); // Required for many S3-compatible services

        // Set custom endpoint if provided (for non-AWS S3-compatible services)
        if let Some(ref endpoint) = config.endpoint {
            s3_config_builder = s3_config_builder.endpoint_url(endpoint);
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: config.bucket.clone(),
        })
    }

    /// Copy an object from source bytes to the secondary bucket
    pub async fn replicate_object(&self, key: &str, data: Vec<u8>) -> Result<(), ReplicationError> {
        let body = ByteStream::from(data);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .map_err(|e| ReplicationError::S3Error(e.to_string()))?;

        Ok(())
    }

    /// Delete an object from the secondary bucket (for mirror mode)
    pub async fn delete_object(&self, key: &str) -> Result<(), ReplicationError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ReplicationError::S3Error(e.to_string()))?;

        Ok(())
    }

    /// Check if an object exists in the secondary bucket
    pub async fn object_exists(&self, key: &str) -> Result<bool, ReplicationError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                let service_err = e.into_service_error();
                if service_err.is_not_found() {
                    Ok(false)
                } else {
                    Err(ReplicationError::S3Error(service_err.to_string()))
                }
            }
        }
    }
}

// =============================================================================
// Job Queue Functions
// =============================================================================

/// Enqueue a replication job for an uploaded file.
/// This is fire-and-forget - errors are logged but never propagate to callers.
pub async fn enqueue_upload(
    store: &clovalink_entity::DataStore,
    storage_path: &str,
    tenant_id: Uuid,
    size_bytes: Option<i64>,
) -> Result<Uuid, ReplicationError> {
    enqueue_job(
        store,
        storage_path,
        tenant_id,
        JobOperation::Upload,
        size_bytes,
    )
    .await
}

/// Enqueue a delete replication job (for mirror mode).
/// This is fire-and-forget - errors are logged but never propagate to callers.
pub async fn enqueue_delete(
    store: &clovalink_entity::DataStore,
    storage_path: &str,
    tenant_id: Uuid,
) -> Result<Uuid, ReplicationError> {
    enqueue_job(store, storage_path, tenant_id, JobOperation::Delete, None).await
}

/// Internal function to enqueue any replication job
async fn enqueue_job(
    store: &clovalink_entity::DataStore,
    storage_path: &str,
    tenant_id: Uuid,
    operation: JobOperation,
    size_bytes: Option<i64>,
) -> Result<Uuid, ReplicationError> {
    let operation_str = operation.to_string();
    let result = store
        .replication()
        .enqueue(storage_path, tenant_id, &operation_str, size_bytes)
        .await?;

    debug!(
        target: "replication",
        job_id = %result,
        storage_path = %storage_path,
        operation = %operation_str,
        "Enqueued replication job"
    );

    Ok(result)
}

/// Fetch the next pending job that's ready for processing
pub async fn fetch_next_job(store: &clovalink_entity::DataStore) -> Result<Option<ReplicationJob>, ReplicationError> {
    Ok(store.replication().fetch_next().await?)
}

/// Mark a job as completed
pub async fn complete_job(store: &clovalink_entity::DataStore, job_id: Uuid) -> Result<(), ReplicationError> {
    store.replication().complete(job_id).await?;
    Ok(())
}

/// Mark a job as failed, scheduling retry if attempts remain
pub async fn fail_job(
    store: &clovalink_entity::DataStore,
    job_id: Uuid,
    error: &str,
    retry_seconds: u64,
) -> Result<bool, ReplicationError> {
    Ok(store.replication().fail(job_id, error, retry_seconds).await?)
}

// =============================================================================
// Background Worker
// =============================================================================

/// Replication worker that processes jobs in the background
pub struct ReplicationWorker {
    store: clovalink_entity::DataStore,
    config: ReplicationConfig,
    replication_client: Option<Arc<ReplicationClient>>,
    primary_storage: Arc<dyn PrimaryStorageReader>,
    worker_id: u32,
}

/// Trait for reading from primary storage (implemented by the main storage backend)
#[async_trait::async_trait]
pub trait PrimaryStorageReader: Send + Sync {
    async fn download(
        &self,
        key: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

impl ReplicationWorker {
    /// Create a new replication worker
    pub async fn new(
        store: clovalink_entity::DataStore,
        config: ReplicationConfig,
        primary_storage: Arc<dyn PrimaryStorageReader>,
        worker_id: u32,
    ) -> Result<Self, ReplicationError> {
        let replication_client = if config.enabled {
            Some(Arc::new(ReplicationClient::new(&config).await?))
        } else {
            None
        };

        Ok(Self {
            store,
            config,
            replication_client,
            primary_storage,
            worker_id,
        })
    }

    /// Run the worker loop
    pub async fn run(self) {
        if !self.config.enabled {
            info!(
                target: "replication",
                worker_id = self.worker_id,
                "Replication worker disabled, exiting"
            );
            return;
        }

        info!(
            target: "replication",
            worker_id = self.worker_id,
            mode = ?self.config.mode,
            bucket = %self.config.bucket,
            "Replication worker started"
        );

        loop {
            match self.process_next_job().await {
                Ok(true) => {
                    // Job processed, immediately check for more
                    continue;
                }
                Ok(false) => {
                    // No jobs available, wait before polling again
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
                Err(e) => {
                    error!(
                        target: "replication",
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
    async fn process_next_job(&self) -> Result<bool, ReplicationError> {
        let job = match fetch_next_job(&self.store).await? {
            Some(job) => job,
            None => return Ok(false),
        };

        info!(
            target: "replication",
            worker_id = self.worker_id,
            job_id = %job.id,
            storage_path = %job.storage_path,
            operation = %job.operation,
            retry_count = job.retry_count.unwrap_or(0),
            "Processing replication job"
        );

        let client = self
            .replication_client
            .as_ref()
            .ok_or(ReplicationError::Disabled)?;

        let result = match job.operation.as_str() {
            "upload" => self.process_upload_job(client, &job).await,
            "delete" => self.process_delete_job(client, &job).await,
            _ => Err(ReplicationError::ConfigError(format!(
                "Unknown operation: {}",
                job.operation
            ))),
        };

        match result {
            Ok(()) => {
                complete_job(&self.store, job.id).await?;
                info!(
                    target: "replication",
                    worker_id = self.worker_id,
                    job_id = %job.id,
                    storage_path = %job.storage_path,
                    "Replication job completed successfully"
                );
            }
            Err(e) => {
                let is_permanent = fail_job(
                    &self.store,
                    job.id,
                    &e.to_string(),
                    self.config.retry_seconds,
                )
                .await?;

                if is_permanent {
                    error!(
                        target: "replication",
                        worker_id = self.worker_id,
                        job_id = %job.id,
                        storage_path = %job.storage_path,
                        error = %e,
                        "Replication job permanently failed after max retries"
                    );
                } else {
                    warn!(
                        target: "replication",
                        worker_id = self.worker_id,
                        job_id = %job.id,
                        storage_path = %job.storage_path,
                        error = %e,
                        retry_count = job.retry_count.unwrap_or(0) + 1,
                        "Replication job failed, will retry"
                    );
                }
            }
        }

        Ok(true)
    }

    /// Process an upload replication job
    async fn process_upload_job(
        &self,
        client: &ReplicationClient,
        job: &ReplicationJob,
    ) -> Result<(), ReplicationError> {
        // Download from primary storage
        let data = self
            .primary_storage
            .download(&job.storage_path)
            .await
            .map_err(|e| {
                ReplicationError::SourceNotFound(format!("{}: {}", job.storage_path, e))
            })?;

        // Upload to secondary storage
        client.replicate_object(&job.storage_path, data).await?;

        Ok(())
    }

    /// Process a delete replication job (mirror mode only)
    async fn process_delete_job(
        &self,
        client: &ReplicationClient,
        job: &ReplicationJob,
    ) -> Result<(), ReplicationError> {
        // Only process deletes in mirror mode
        if self.config.mode != ReplicationMode::Mirror {
            debug!(
                target: "replication",
                job_id = %job.id,
                "Skipping delete job - not in mirror mode"
            );
            return Ok(());
        }

        client.delete_object(&job.storage_path).await?;

        Ok(())
    }
}

// =============================================================================
// Admin/Status Functions
// =============================================================================

/// Replication status summary for admin dashboard
#[derive(Debug, Clone, Serialize)]
pub struct ReplicationStatus {
    pub enabled: bool,
    pub mode: String,
    pub bucket: String,
    pub pending_jobs: i64,
    pub processing_jobs: i64,
    pub failed_jobs: i64,
    pub completed_last_hour: i64,
    pub oldest_pending_age_seconds: Option<i64>,
}

/// Get replication status for admin dashboard
pub async fn get_status(
    store: &clovalink_entity::DataStore,
    config: &ReplicationConfig,
) -> Result<ReplicationStatus, ReplicationError> {
    let stats = store.replication().stats().await?;

    Ok(ReplicationStatus {
        enabled: config.enabled,
        mode: format!("{:?}", config.mode).to_lowercase(),
        bucket: if config.enabled {
            config.bucket.clone()
        } else {
            String::new()
        },
        pending_jobs: stats.pending_jobs,
        processing_jobs: stats.processing_jobs,
        failed_jobs: stats.failed_jobs,
        completed_last_hour: stats.completed_last_hour,
        oldest_pending_age_seconds: stats.oldest_pending_age_seconds,
    })
}

/// Get pending/failed jobs for admin review
pub async fn get_pending_jobs(
    store: &clovalink_entity::DataStore,
    status_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ReplicationJob>, ReplicationError> {
    let status = status_filter.unwrap_or("pending");

    Ok(store.replication().jobs(status, limit, offset).await?)
}

/// Retry all failed jobs (reset them to pending)
pub async fn retry_failed_jobs(store: &clovalink_entity::DataStore) -> Result<i64, ReplicationError> {
    Ok(store.replication().retry_failed().await?)
}
