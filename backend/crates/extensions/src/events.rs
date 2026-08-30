//! Extension event dispatching for file upload triggers

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Extension;
use crate::permissions::{check_extension_permission, Permission};
use crate::webhook::{dispatch_webhook, FileEventPayload};

/// File event for dispatching to extensions
#[derive(Debug, Clone)]
pub struct FileEvent {
    pub company_id: Uuid,
    pub user_id: Uuid,
    pub file_id: Uuid,
    pub filename: String,
    pub content_type: Option<String>,
    pub size_bytes: i64,
    pub metadata: serde_json::Value,
}

/// Dispatch a file upload event to all relevant file processor extensions
pub async fn dispatch_file_event(
    pool: &PgPool,
    redis_url: &str,
    event: FileEvent,
    webhook_timeout_ms: u64,
) {
    // Get all installed file processor extensions for this tenant
    let extensions = match get_file_processor_extensions(pool, event.company_id).await {
        Ok(exts) => exts,
        Err(e) => {
            tracing::error!("Failed to get file processor extensions: {:?}", e);
            return;
        }
    };

    if extensions.is_empty() {
        return;
    }

    // Check rate limiting using Redis
    let redis_client = match redis::Client::open(redis_url) {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("Failed to connect to Redis for rate limiting: {:?}", e);
            return;
        }
    };

    for extension in extensions {
        // Check if extension has the required permission
        let has_permission = check_extension_permission(
            pool,
            extension.id,
            event.company_id,
            Permission::FileProcessorRun,
        )
        .await
        .unwrap_or(false);

        if !has_permission {
            tracing::warn!(
                "Extension {} does not have file_processor:run permission for tenant {}",
                extension.id,
                event.company_id
            );
            continue;
        }

        // Check file type filter
        if !should_process_file(&extension, &event).await {
            continue;
        }

        // Check rate limit
        if !check_rate_limit(&redis_client, &extension.id, 60).await {
            tracing::warn!(
                "Rate limit exceeded for extension {} - skipping file event",
                extension.id
            );
            continue;
        }

        // Build payload
        let payload = FileEventPayload {
            company_id: event.company_id.to_string(),
            user_id: event.user_id.to_string(),
            file_id: event.file_id.to_string(),
            filename: event.filename.clone(),
            content_type: event.content_type.clone(),
            size_bytes: event.size_bytes,
            event: "file_uploaded".to_string(),
            metadata: event.metadata.clone(),
            timestamp: Utc::now().to_rfc3339(),
        };

        // Dispatch webhook (non-blocking)
        let pool_clone = pool.clone();
        let extension_clone = extension.clone();
        tokio::spawn(async move {
            match dispatch_webhook(
                &pool_clone,
                &extension_clone,
                "file_uploaded",
                &payload,
                webhook_timeout_ms,
            )
            .await
            {
                Ok((status, _)) => {
                    tracing::info!(
                        "File event dispatched to extension {} - status {}",
                        extension_clone.id,
                        status
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to dispatch file event to extension {}: {:?}",
                        extension_clone.id,
                        e
                    );
                }
            }
        });
    }
}

/// Get all active file processor extensions installed for a tenant
async fn get_file_processor_extensions(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<Vec<Extension>, sqlx::Error> {
    sqlx::query_as!(
        Extension,
        r#"
        SELECT e.id, e.tenant_id, e.name, e.slug, e.description, e.extension_type,
               e.manifest_url, e.webhook_url, e.public_key, e.signature_algorithm,
               e.status, e.allowed_tenant_ids, e.created_at, e.updated_at
        FROM extensions e
        JOIN extension_installations ei ON e.id = ei.extension_id
        WHERE ei.tenant_id = $1
          AND ei.enabled = true
          AND e.status = 'active'
          AND e.extension_type = 'file_processor'
        "#,
        tenant_id
    )
    .fetch_all(pool)
    .await
}

/// Check if a file should be processed based on extension's filter config
async fn should_process_file(_extension: &Extension, _event: &FileEvent) -> bool {
    // For now, process all files. 
    // In the future, check extension_event_triggers table for file type filters
    true
}

/// Check rate limit for an extension using Redis
async fn check_rate_limit(
    redis_client: &redis::Client,
    extension_id: &Uuid,
    limit_per_minute: u32,
) -> bool {
    let key = format!("clovalink:ratelimit:ext:{}", extension_id);
    
    let mut conn = match redis_client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to get Redis connection: {:?}", e);
            return true; // Allow on Redis error
        }
    };

    // Increment counter with 60 second expiry
    let result: Result<(i32, i32), redis::RedisError> = redis::pipe()
        .atomic()
        .incr(&key, 1)
        .expire(&key, 60)
        .query_async(&mut conn)
        .await;

    match result {
        Ok((count, _)) => count <= limit_per_minute as i32,
        Err(e) => {
            tracing::error!("Redis rate limit check failed: {:?}", e);
            true // Allow on error
        }
    }
}

/// Dispatch event for file deletion
pub async fn dispatch_file_deleted_event(
    pool: &PgPool,
    _redis_url: &str,
    company_id: Uuid,
    user_id: Uuid,
    file_id: Uuid,
    filename: String,
    webhook_timeout_ms: u64,
) {
    let event = FileEvent {
        company_id,
        user_id,
        file_id,
        filename,
        content_type: None,
        size_bytes: 0,
        metadata: serde_json::json!({"deleted": true}),
    };

    // Similar to file upload, but with "file_deleted" event type
    let extensions = match get_file_processor_extensions(pool, company_id).await {
        Ok(exts) => exts,
        Err(e) => {
            tracing::error!("Failed to get file processor extensions: {:?}", e);
            return;
        }
    };

    for extension in extensions {
        let has_permission = check_extension_permission(
            pool,
            extension.id,
            company_id,
            Permission::FileProcessorRun,
        )
        .await
        .unwrap_or(false);

        if !has_permission {
            continue;
        }

        let payload = FileEventPayload {
            company_id: company_id.to_string(),
            user_id: user_id.to_string(),
            file_id: file_id.to_string(),
            filename: event.filename.clone(),
            content_type: None,
            size_bytes: 0,
            event: "file_deleted".to_string(),
            metadata: serde_json::json!({}),
            timestamp: Utc::now().to_rfc3339(),
        };

        let pool_clone = pool.clone();
        let extension_clone = extension.clone();
        tokio::spawn(async move {
            let _ = dispatch_webhook(
                &pool_clone,
                &extension_clone,
                "file_deleted",
                &payload,
                webhook_timeout_ms,
            )
            .await;
        });
    }
}

