//! Extension-related handlers integrated with main API
//! 
//! This module provides helper functions for dispatching extension events
//! from within the main API handlers.

use sqlx::PgPool;
use uuid::Uuid;

/// Dispatch file upload event to extensions
pub async fn dispatch_file_upload(
    pool: &PgPool,
    redis_url: &str,
    company_id: Uuid,
    user_id: Uuid,
    file_id: Uuid,
    filename: &str,
    content_type: Option<&str>,
    size_bytes: i64,
    webhook_timeout_ms: u64,
) {
    let event = clovalink_extensions::FileEvent {
        company_id,
        user_id,
        file_id,
        filename: filename.to_string(),
        content_type: content_type.map(|s| s.to_string()),
        size_bytes,
        metadata: serde_json::json!({}),
    };

    clovalink_extensions::dispatch_file_event(pool, redis_url, event, webhook_timeout_ms).await;
}

