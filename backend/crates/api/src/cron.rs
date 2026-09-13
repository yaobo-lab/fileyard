use crate::AppState;
use axum::{extract::State, http::StatusCode, response::Json, Extension};
use chrono::Utc;
use clovalink_auth::{require_admin, AuthUser};
use clovalink_core::models::Tenant;
use clovalink_core::notification_service;
use serde_json::{json, Value};
use std::sync::Arc;

/// Manually trigger cleanup of expired files
/// POST /api/cron/cleanup
/// Requires Admin role
pub async fn cleanup_expired_files(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // 1. Get all tenants and their retention policies
    let tenants = state
        .store
        .tenants()
        .list_retention_policies()
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch tenants: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut deleted_count = 0;

    for (tenant_id, retention_days) in tenants {
        // Skip tenants with infinite retention (0 = never auto-delete from trash)
        if retention_days == 0 {
            tracing::debug!("Skipping tenant {} - infinite retention policy", tenant_id);
            continue;
        }

        // Calculate cutoff date
        let cutoff_date = Utc::now() - chrono::Duration::days(retention_days as i64);

        // 2. Find expired files for this tenant
        let expired_files = state
            .store
            .files()
            .list_expired(tenant_id, cutoff_date)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch expired files: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        for (file_name, storage_path) in expired_files {
            // 3. Delete from storage
            if let Err(e) = state.storage.delete(&storage_path).await {
                tracing::error!(
                    "Failed to delete file from storage: {:?}, error: {:?}",
                    storage_path,
                    e
                );
            }

            // 4. Delete from database
            match state.store.files().delete_by_name(tenant_id, &file_name).await {
                Ok(rows) => {
                    deleted_count += rows;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to delete file metadata: {:?}, error: {:?}",
                        file_name,
                        e
                    );
                }
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
    let three_days = now + chrono::Duration::days(3);
    let one_day_ago = now - chrono::Duration::days(1);

    // Find requests expiring within 3 days
    let requests = state
        .store
        .file_requests()
        .list_expiring(now, three_days)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch expiring requests: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut notification_count = 0;

    for req in requests {
        // Check if recently notified within 1 day
        if let Ok(true) = state
            .store
            .notifications()
            .has_recent_request_expiring(req.id, one_day_ago)
            .await
        {
            continue;
        }

        let duration = req.expires_at.with_timezone(&Utc) - now;
        let days_until = duration.num_days() as i32;

        if let Ok(Some(user)) = state.store.users().user(req.created_by).await {
            if let Ok(Some(tenant_model)) = state.store.tenants().by_id(req.tenant_id).await {
                let tenant = Tenant::from(tenant_model);
                let _ = notification_service::notify_expiring_request(
                    &state.store,
                    &tenant,
                    req.created_by,
                    &user.email,
                    &user.role,
                    &req.name,
                    req.id,
                    days_until,
                )
                .await;
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
    let tenants = state
        .store
        .tenants()
        .list_with_storage_quota()
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch tenants: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut warning_count = 0;
    let one_day_ago = Utc::now() - chrono::Duration::hours(24);

    for tenant_model in tenants {
        let tenant = Tenant::from(tenant_model);
        if let Some(quota) = tenant.storage_quota_bytes {
            // Calculate actual storage from files_metadata
            let actual_storage = state
                .store
                .files()
                .calculate_actual_storage(tenant.id)
                .await
                .unwrap_or(0);

            let percentage = ((actual_storage as f64 / quota as f64) * 100.0) as i32;

            // Only warn at 80%, 90%, and 100% thresholds
            if percentage >= 80 {
                let threshold = if percentage >= 100 {
                    100
                } else if percentage >= 90 {
                    90
                } else {
                    80
                };

                let already_warned = state
                    .store
                    .notifications()
                    .has_recent_storage_warning(tenant.id, threshold, one_day_ago)
                    .await
                    .unwrap_or(false);

                if !already_warned {
                    let _ = notification_service::notify_all_admins(
                        &state.store,
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
                            "storage_used_bytes": actual_storage,
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
