//! Redis-based background job queue for ClovaLink
//!
//! Provides a general-purpose job queue for offloading heavy operations:
//! - File processing (thumbnails, virus scanning)
//! - Bulk data exports
//! - Email sending
//! - Audit log archival
//! - Retention policy cleanup

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Queue configuration
const REDIS_QUEUE_PREFIX: &str = "clovalink:queue";
const DEFAULT_RETRY_ATTEMPTS: u32 = 3;
const DEFAULT_RETRY_DELAY_MS: u64 = 5000;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("Redis connection error: {0}")]
    ConnectionError(String),
    #[error("Redis command error: {0}")]
    CommandError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Job not found")]
    JobNotFound,
    #[error("Job failed: {0}")]
    JobFailed(String),
    #[error("Queue is empty")]
    QueueEmpty,
}

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for JobPriority {
    fn default() -> Self {
        JobPriority::Normal
    }
}

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Retrying,
}

/// Job types supported by the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    /// Process uploaded file (generate thumbnail, scan for viruses)
    FileProcessing {
        file_id: Uuid,
        tenant_id: Uuid,
        operations: Vec<String>, // ["thumbnail", "virus_scan", "metadata_extract"]
    },
    /// Export data for a user (GDPR data export)
    DataExport {
        user_id: Uuid,
        tenant_id: Uuid,
        export_type: String, // "full", "files_only", "metadata_only"
    },
    /// Send email notification
    SendEmail {
        tenant_id: Uuid,
        to: String,
        subject: String,
        body: String,
        template: Option<String>,
    },
    /// Archive old audit logs
    AuditLogArchival {
        tenant_id: Uuid,
        older_than_days: u32,
    },
    /// Apply retention policy (delete expired files)
    RetentionCleanup {
        tenant_id: Uuid,
    },
    /// Bulk file operation
    BulkFileOperation {
        tenant_id: Uuid,
        file_ids: Vec<Uuid>,
        operation: String, // "delete", "move", "copy"
        destination: Option<String>,
    },
    /// Custom job type for extensions
    Custom {
        job_type: String,
        payload: serde_json::Value,
    },
}

/// A job in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub priority: JobPriority,
    pub status: JobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
    pub result: Option<serde_json::Value>,
}

impl Job {
    pub fn new(job_type: JobType) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type,
            priority: JobPriority::default(),
            status: JobStatus::Pending,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: DEFAULT_RETRY_ATTEMPTS,
            error_message: None,
            result: None,
        }
    }

    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
}

/// Job queue backed by Redis
#[derive(Clone)]
pub struct JobQueue {
    conn: Arc<RwLock<redis::aio::ConnectionManager>>,
    queue_name: String,
}

impl JobQueue {
    /// Create a new job queue instance
    pub async fn new(redis_url: &str, queue_name: &str) -> Result<Self, QueueError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        let conn = redis::aio::ConnectionManager::new(client)
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(RwLock::new(conn)),
            queue_name: format!("{}:{}", REDIS_QUEUE_PREFIX, queue_name),
        })
    }

    /// Enqueue a new job
    pub async fn enqueue(&self, job: Job) -> Result<Uuid, QueueError> {
        let mut conn = self.conn.write().await;
        let job_id = job.id;

        // Serialize the job
        let job_json = serde_json::to_string(&job)
            .map_err(|e| QueueError::SerializationError(e.to_string()))?;

        // Store job data
        let job_key = format!("{}:job:{}", self.queue_name, job_id);
        let _: () = conn
            .set(&job_key, &job_json)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        // Add to priority queue (sorted set with priority as score)
        let queue_key = format!("{}:pending", self.queue_name);
        let score = job.priority as i64 * 1_000_000_000 + chrono::Utc::now().timestamp();
        let _: () = conn
            .zadd(&queue_key, job_id.to_string(), score)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        debug!("Enqueued job {} with priority {:?}", job_id, job.priority);
        Ok(job_id)
    }

    /// Dequeue the next job (highest priority, oldest first)
    pub async fn dequeue(&self) -> Result<Option<Job>, QueueError> {
        let mut conn = self.conn.write().await;

        let queue_key = format!("{}:pending", self.queue_name);

        // Get the highest priority job (highest score)
        let job_ids: Vec<String> = conn
            .zrevrange(&queue_key, 0, 0)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        if job_ids.is_empty() {
            return Ok(None);
        }

        let job_id = &job_ids[0];

        // Remove from pending queue
        let _: () = conn
            .zrem(&queue_key, job_id)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        // Get job data
        let job_key = format!("{}:job:{}", self.queue_name, job_id);
        let job_json: Option<String> = conn
            .get(&job_key)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        match job_json {
            Some(json) => {
                let mut job: Job = serde_json::from_str(&json)
                    .map_err(|e| QueueError::SerializationError(e.to_string()))?;

                // Update job status
                job.status = JobStatus::Processing;
                job.started_at = Some(chrono::Utc::now());

                // Save updated job
                let updated_json = serde_json::to_string(&job)
                    .map_err(|e| QueueError::SerializationError(e.to_string()))?;
                let _: () = conn
                    .set(&job_key, &updated_json)
                    .await
                    .map_err(|e| QueueError::CommandError(e.to_string()))?;

                // Add to processing set with TTL for stale job detection
                let processing_key = format!("{}:processing", self.queue_name);
                let _: () = conn
                    .zadd(&processing_key, job_id, chrono::Utc::now().timestamp())
                    .await
                    .map_err(|e| QueueError::CommandError(e.to_string()))?;

                debug!("Dequeued job {}", job.id);
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    /// Mark a job as completed
    pub async fn complete(&self, job_id: Uuid, result: Option<serde_json::Value>) -> Result<(), QueueError> {
        let mut conn = self.conn.write().await;

        let job_key = format!("{}:job:{}", self.queue_name, job_id);
        let job_json: Option<String> = conn
            .get(&job_key)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        match job_json {
            Some(json) => {
                let mut job: Job = serde_json::from_str(&json)
                    .map_err(|e| QueueError::SerializationError(e.to_string()))?;

                job.status = JobStatus::Completed;
                job.completed_at = Some(chrono::Utc::now());
                job.result = result;

                // Save updated job
                let updated_json = serde_json::to_string(&job)
                    .map_err(|e| QueueError::SerializationError(e.to_string()))?;
                let _: () = conn
                    .set_ex(&job_key, &updated_json, 86400) // Keep completed jobs for 24 hours
                    .await
                    .map_err(|e| QueueError::CommandError(e.to_string()))?;

                // Remove from processing set
                let processing_key = format!("{}:processing", self.queue_name);
                let _: () = conn
                    .zrem(&processing_key, job_id.to_string())
                    .await
                    .map_err(|e| QueueError::CommandError(e.to_string()))?;

                info!("Job {} completed successfully", job_id);
                Ok(())
            }
            None => Err(QueueError::JobNotFound),
        }
    }

    /// Mark a job as failed (will retry if retries remaining)
    pub async fn fail(&self, job_id: Uuid, error: &str) -> Result<bool, QueueError> {
        let mut conn = self.conn.write().await;

        let job_key = format!("{}:job:{}", self.queue_name, job_id);
        let job_json: Option<String> = conn
            .get(&job_key)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        match job_json {
            Some(json) => {
                let mut job: Job = serde_json::from_str(&json)
                    .map_err(|e| QueueError::SerializationError(e.to_string()))?;

                job.retry_count += 1;
                job.error_message = Some(error.to_string());

                // Remove from processing set
                let processing_key = format!("{}:processing", self.queue_name);
                let _: () = conn
                    .zrem(&processing_key, job_id.to_string())
                    .await
                    .map_err(|e| QueueError::CommandError(e.to_string()))?;

                if job.retry_count < job.max_retries {
                    // Re-queue for retry
                    job.status = JobStatus::Retrying;
                    warn!(
                        "Job {} failed (attempt {}/{}), retrying: {}",
                        job_id, job.retry_count, job.max_retries, error
                    );

                    let updated_json = serde_json::to_string(&job)
                        .map_err(|e| QueueError::SerializationError(e.to_string()))?;
                    let _: () = conn
                        .set(&job_key, &updated_json)
                        .await
                        .map_err(|e| QueueError::CommandError(e.to_string()))?;

                    // Add back to pending queue with delay
                    let queue_key = format!("{}:pending", self.queue_name);
                    let delay_score = chrono::Utc::now().timestamp()
                        + (DEFAULT_RETRY_DELAY_MS as i64 * job.retry_count as i64 / 1000);
                    let _: () = conn
                        .zadd(&queue_key, job_id.to_string(), delay_score)
                        .await
                        .map_err(|e| QueueError::CommandError(e.to_string()))?;

                    Ok(true) // Will retry
                } else {
                    // Max retries exceeded
                    job.status = JobStatus::Failed;
                    job.completed_at = Some(chrono::Utc::now());
                    error!(
                        "Job {} permanently failed after {} attempts: {}",
                        job_id, job.retry_count, error
                    );

                    let updated_json = serde_json::to_string(&job)
                        .map_err(|e| QueueError::SerializationError(e.to_string()))?;
                    let _: () = conn
                        .set_ex(&job_key, &updated_json, 604800) // Keep failed jobs for 7 days
                        .await
                        .map_err(|e| QueueError::CommandError(e.to_string()))?;

                    Ok(false) // Won't retry
                }
            }
            None => Err(QueueError::JobNotFound),
        }
    }

    /// Get job by ID
    pub async fn get_job(&self, job_id: Uuid) -> Result<Option<Job>, QueueError> {
        let mut conn = self.conn.write().await;

        let job_key = format!("{}:job:{}", self.queue_name, job_id);
        let job_json: Option<String> = conn
            .get(&job_key)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        match job_json {
            Some(json) => {
                let job: Job = serde_json::from_str(&json)
                    .map_err(|e| QueueError::SerializationError(e.to_string()))?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    /// Get queue statistics
    pub async fn stats(&self) -> Result<QueueStats, QueueError> {
        let mut conn = self.conn.write().await;

        let pending_key = format!("{}:pending", self.queue_name);
        let processing_key = format!("{}:processing", self.queue_name);

        let pending: i64 = conn
            .zcard(&pending_key)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        let processing: i64 = conn
            .zcard(&processing_key)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        Ok(QueueStats {
            pending: pending as u64,
            processing: processing as u64,
        })
    }

    /// Clean up stale processing jobs (jobs that have been processing too long)
    pub async fn cleanup_stale_jobs(&self, max_age_secs: u64) -> Result<u64, QueueError> {
        let mut conn = self.conn.write().await;

        let processing_key = format!("{}:processing", self.queue_name);
        let cutoff = chrono::Utc::now().timestamp() - max_age_secs as i64;

        // Get stale job IDs
        let stale_jobs: Vec<String> = conn
            .zrangebyscore(&processing_key, 0, cutoff)
            .await
            .map_err(|e| QueueError::CommandError(e.to_string()))?;

        let count = stale_jobs.len() as u64;

        for job_id in stale_jobs {
            // Remove from processing
            let _: () = conn
                .zrem(&processing_key, &job_id)
                .await
                .map_err(|e| QueueError::CommandError(e.to_string()))?;

            // Re-queue for retry
            let queue_key = format!("{}:pending", self.queue_name);
            let _: () = conn
                .zadd(&queue_key, &job_id, chrono::Utc::now().timestamp())
                .await
                .map_err(|e| QueueError::CommandError(e.to_string()))?;

            warn!("Re-queued stale job: {}", job_id);
        }

        Ok(count)
    }
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending: u64,
    pub processing: u64,
}

/// Background worker that processes jobs from the queue
pub struct JobWorker {
    queue: JobQueue,
    running: Arc<RwLock<bool>>,
}

impl JobWorker {
    pub fn new(queue: JobQueue) -> Self {
        Self {
            queue,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the worker (processes jobs in a loop)
    pub async fn start<F, Fut>(&self, processor: F)
    where
        F: Fn(Job) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send,
    {
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        info!("Job worker started");

        loop {
            {
                let running = self.running.read().await;
                if !*running {
                    break;
                }
            }

            match self.queue.dequeue().await {
                Ok(Some(job)) => {
                    let job_id = job.id;
                    let processor = processor.clone();

                    match processor(job).await {
                        Ok(result) => {
                            if let Err(e) = self.queue.complete(job_id, result).await {
                                error!("Failed to mark job {} as complete: {}", job_id, e);
                            }
                        }
                        Err(error) => {
                            if let Err(e) = self.queue.fail(job_id, &error).await {
                                error!("Failed to mark job {} as failed: {}", job_id, e);
                            }
                        }
                    }
                }
                Ok(None) => {
                    // No jobs available, wait before checking again
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!("Error dequeuing job: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        info!("Job worker stopped");
    }

    /// Stop the worker
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let job = Job::new(JobType::RetentionCleanup {
            tenant_id: Uuid::new_v4(),
        })
        .with_priority(JobPriority::High)
        .with_max_retries(5);

        assert_eq!(job.priority, JobPriority::High);
        assert_eq!(job.max_retries, 5);
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn test_job_serialization() {
        let job = Job::new(JobType::SendEmail {
            tenant_id: Uuid::new_v4(),
            to: "test@example.com".to_string(),
            subject: "Test".to_string(),
            body: "Hello".to_string(),
            template: None,
        });

        let json = serde_json::to_string(&job).unwrap();
        let parsed: Job = serde_json::from_str(&json).unwrap();

        assert_eq!(job.id, parsed.id);
    }
}
