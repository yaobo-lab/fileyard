use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Extension,
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use crate::AppState;
use clovalink_auth::{AuthUser, require_admin};
use clovalink_core::models::Tenant;
use clovalink_core::notification_service;
use chrono::Utc;

/// Manually trigger cleanup of expired files
/// POST /api/cron/cleanup
/// Requires Admin role
pub async fn cleanup_expired_files(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // 1. Get all tenants and their retention policies
    let tenants = sqlx::query!(
        "SELECT id, retention_policy_days FROM tenants"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch tenants: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut deleted_count = 0;

    for tenant in tenants {
        let retention_days = tenant.retention_policy_days;
        
        // Skip tenants with infinite retention (0 = never auto-delete from trash)
        if retention_days == 0 {
            tracing::debug!("Skipping tenant {} - infinite retention policy", tenant.id);
            continue;
        }
        
        // Calculate cutoff date
        let cutoff_date = Utc::now() - chrono::Duration::days(retention_days as i64);

        // 2. Find expired files for this tenant
        let expired_files = sqlx::query!(
            "SELECT name, storage_path FROM files_metadata WHERE tenant_id = $1 AND is_deleted = true AND deleted_at < $2",
            tenant.id,
            cutoff_date
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch expired files: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        for file in expired_files {
            // 3. Delete from storage
            // Note: storage_path should already point to .trash/...
            if let Err(e) = state.storage.delete(&file.storage_path).await {
                tracing::error!("Failed to delete file from storage: {:?}, error: {:?}", file.storage_path, e);
                // Continue to next file even if storage deletion fails? 
                // Ideally yes, but maybe we should keep metadata if storage fails?
                // For now, we'll log and proceed to delete metadata to keep DB clean.
            }

            // 4. Delete from database
            if let Err(e) = sqlx::query!(
                "DELETE FROM files_metadata WHERE tenant_id = $1 AND name = $2",
                tenant.id,
                file.name
            )
            .execute(&state.pool)
            .await {
                tracing::error!("Failed to delete file metadata: {:?}, error: {:?}", file.name, e);
            } else {
                deleted_count += 1;
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "deleted_count": deleted_count,
        "message": format!("Successfully cleaned up {} expired files", deleted_count)
    })))
}

/// Check for expiring file requests and send notifications
/// POST /api/cron/expiring-requests
/// Requires Admin role
pub async fn notify_expiring_requests(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    let now = Utc::now();
    let _one_day = now + chrono::Duration::days(1);
    let three_days = now + chrono::Duration::days(3);

    // Find requests expiring within 3 days that haven't been notified
    let expiring_requests: Vec<(Uuid, String, Uuid, chrono::DateTime<Utc>, Uuid)> = sqlx::query_as(
        r#"
        SELECT fr.id, fr.name, fr.created_by, fr.expires_at, fr.tenant_id
        FROM file_requests fr
        WHERE fr.status = 'active'
          AND fr.expires_at > $1
          AND fr.expires_at <= $2
          AND NOT EXISTS (
            SELECT 1 FROM notifications n 
            WHERE n.notification_type = 'request_expiring' 
              AND n.metadata->>'request_id' = fr.id::text
              AND n.created_at > ($1 - interval '1 day')
          )
        "#
    )
    .bind(now)
    .bind(three_days)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch expiring requests: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut notification_count = 0;

    for (request_id, request_name, created_by, expires_at, tenant_id) in expiring_requests {
        // Calculate days until expiry
        let duration = expires_at - now;
        let days_until = duration.num_days() as i32;

        // Get user details
        let user: Option<(String, String)> = sqlx::query_as(
            "SELECT email, role FROM users WHERE id = $1"
        )
        .bind(created_by)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if let Some((user_email, user_role)) = user {
            // Get tenant
            if let Ok(tenant) = sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE id = $1")
                .bind(tenant_id)
                .fetch_one(&state.pool)
                .await
            {
                let _ = notification_service::notify_expiring_request(
                    &state.pool,
                    &tenant,
                    created_by,
                    &user_email,
                    &user_role,
                    &request_name,
                    request_id,
                    days_until,
                ).await;
                notification_count += 1;
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "notification_count": notification_count,
        "message": format!("Sent {} expiring request notifications", notification_count)
    })))
}

/// Check storage quotas and send warnings
/// POST /api/cron/storage-warnings
/// Requires Admin role
pub async fn check_storage_quotas(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Get tenants with storage quotas
    let tenants: Vec<Tenant> = sqlx::query_as(
        "SELECT * FROM tenants WHERE storage_quota_bytes IS NOT NULL AND status = 'active'"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch tenants: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut warning_count = 0;

    for tenant in tenants {
        if let Some(quota) = tenant.storage_quota_bytes {
            // Calculate actual storage from files_metadata (not stale tenant.storage_used_bytes)
            let actual_storage: (i64,) = sqlx::query_as(
                "SELECT COALESCE(SUM(size_bytes), 0)::bigint FROM files_metadata WHERE tenant_id = $1 AND is_deleted = false AND is_directory = false"
            )
            .bind(tenant.id)
            .fetch_one(&state.pool)
            .await
            .unwrap_or((0,));

            let percentage = ((actual_storage.0 as f64 / quota as f64) * 100.0) as i32;

            // Only warn at 80%, 90%, and 100% thresholds
            if percentage >= 80 {
                // Check if we already sent a warning at this threshold recently
                let threshold = if percentage >= 100 { 100 } else if percentage >= 90 { 90 } else { 80 };
                
                let recent_warning: Option<(i64,)> = sqlx::query_as(
                    r#"
                    SELECT COUNT(*) FROM notifications 
                    WHERE notification_type = 'storage_warning' 
                      AND tenant_id = $1
                      AND (metadata->>'percentage_used')::int >= $2
                      AND created_at > NOW() - interval '24 hours'
                    "#
                )
                .bind(tenant.id)
                .bind(threshold)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten();

                if recent_warning.map(|(c,)| c).unwrap_or(0) == 0 {
                    // No recent warning at this threshold, send one
                    let _ = notification_service::notify_all_admins(
                        &state.pool,
                        &tenant,
                        notification_service::NotificationType::StorageWarning,
                        &if percentage >= 100 {
                            "Storage quota exceeded".to_string()
                        } else {
                            format!("Storage {}% full", percentage)
                        },
                        &if percentage >= 100 {
                            "Your storage quota has been exceeded. Please free up space or upgrade your plan.".to_string()
                        } else {
                            format!("Your organization has used {}% of the storage quota.", percentage)
                        },
                        Some(serde_json::json!({
                            "percentage_used": percentage,
                            "storage_used_bytes": actual_storage.0,
                            "storage_quota_bytes": quota
                        })),
                    ).await;
                    warning_count += 1;
                }
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "warning_count": warning_count,
        "message": format!("Sent {} storage warnings", warning_count)
    })))
}
