use crate::AppState;
use axum::{extract::State, http::StatusCode, response::Json, Extension};
use clovalink_auth::{require_admin, AuthUser};
use clovalink_core::cache::{keys as cache_keys, ttl as cache_ttl};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

/// Cached dashboard stats response
#[derive(Serialize, Deserialize, Clone)]
struct DashboardStatsCache {
    data: Value,
}

/// Get dashboard statistics
/// GET /api/dashboard/stats
/// Requires Admin or SuperAdmin role
pub async fn get_dashboard_stats(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // SECURITY: Dashboard is Admin/SuperAdmin only
    require_admin(&auth)?;

    // Check cache first - use role-specific cache key since SuperAdmin sees different data
    let cache_key = cache_keys::dashboard_stats(auth.tenant_id, &auth.role);
    if let Some(ref cache) = state.cache {
        if let Ok(cached) = cache.get::<DashboardStatsCache>(&cache_key).await {
            return Ok(Json(cached.data));
        }
    }

    // Get tenant storage distribution (for SuperAdmin, show all tenants; otherwise show current tenant)
    // Calculate actual storage from files_metadata instead of using stale tenants.storage_used_bytes
    let storage_distribution = state
        .store
        .dashboard()
        .storage_distribution(auth.tenant_id, auth.role == "SuperAdmin")
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch storage distribution: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Calculate total storage for percentage calculation
    let total_storage: i64 = storage_distribution
        .iter()
        .map(|(_, _, used, _)| used)
        .sum();

    let storage_data: Vec<Value> = storage_distribution
        .iter()
        .map(|(id, name, used, quota)| {
            // Calculate percentage against quota if set, otherwise relative to total
            let percentage = if let Some(q) = quota {
                if *q > 0 {
                    ((*used as f64 / *q as f64) * 100.0).round() as i32
                } else {
                    0
                }
            } else if total_storage > 0 {
                ((*used as f64 / total_storage as f64) * 100.0).round() as i32
            } else {
                0
            };
            json!({
                "id": id,
                "name": name,
                "used_bytes": used,
                "quota_bytes": quota,
                "used_formatted": format_bytes(*used),
                "percentage": percentage,
                "quota_formatted": quota.map(|q| format_bytes(q))
            })
        })
        .collect();

    // Get active file requests count and summaries
    let file_requests = state
        .store
        .dashboard()
        .active_requests(auth.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch file requests: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let file_request_data: Vec<Value> = file_requests
        .iter()
        .map(|(id, name, upload_count, expires_at)| {
            let now = chrono::Utc::now();
            let duration = *expires_at - now;
            let days_until_expiry = duration.num_days();
            let expiry_text = if days_until_expiry <= 0 {
                "Expired".to_string()
            } else if days_until_expiry == 1 {
                "Expires tomorrow".to_string()
            } else {
                format!("Expires in {} days", days_until_expiry)
            };

            json!({
                "id": id,
                "name": name,
                "upload_count": upload_count,
                "expires_at": expires_at,
                "expiry_text": expiry_text,
                "has_new_uploads": *upload_count > 0
            })
        })
        .collect();

    // Get total active file requests count
    let total_active_requests = state
        .store
        .dashboard()
        .active_request_count(auth.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count file requests: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get total files count
    let total_files = state
        .store
        .dashboard()
        .file_count(auth.tenant_id)
        .await
        .unwrap_or(0);

    // Get user count for current tenant
    let user_count = state
        .store
        .dashboard()
        .user_count(auth.tenant_id)
        .await
        .unwrap_or(0);

    // Get company count (for SuperAdmin)
    let company_count = if auth.role == "SuperAdmin" {
        state.store.dashboard().company_count().await.unwrap_or(0)
    } else {
        1
    };

    // Get total storage for current tenant (calculated from actual files)
    let tenant_storage = state
        .store
        .dashboard()
        .tenant_storage(auth.tenant_id)
        .await
        .unwrap_or(0);

    // Get storage quota for current tenant
    let tenant_quota = state
        .store
        .dashboard()
        .tenant_quota(auth.tenant_id)
        .await
        .unwrap_or(None);

    let response = json!({
        "storage_distribution": storage_data,
        "total_storage_bytes": total_storage,
        "total_storage_formatted": format_bytes(total_storage),
        "file_requests": file_request_data,
        "total_active_requests": total_active_requests,
        "stats": {
            "companies": company_count, "users": user_count, "files": total_files,
            "storage_used_bytes": tenant_storage, "storage_used_formatted": format_bytes(tenant_storage),
            "storage_quota_bytes": tenant_quota, "storage_quota_formatted": tenant_quota.map(format_bytes)
        }
    });

    // Cache the response for 60 seconds
    if let Some(ref cache) = state.cache {
        let cache_data = DashboardStatsCache {
            data: response.clone(),
        };
        let _ = cache
            .set(&cache_key, &cache_data, cache_ttl::DASHBOARD)
            .await;
    }

    Ok(Json(response))
}

/// Get file types distribution
/// GET /api/dashboard/file-types
/// Requires Admin or SuperAdmin role
pub async fn get_file_types(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // SECURITY: Dashboard is Admin/SuperAdmin only
    require_admin(&auth)?;
    // Get file count grouped by content type for the current tenant
    let file_types = state
        .store
        .dashboard()
        .file_types(auth.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch file types: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get total file count
    let total = state
        .store
        .dashboard()
        .non_directory_file_count(auth.tenant_id)
        .await
        .unwrap_or(0);

    // Map content types to friendly labels
    let file_type_data: Vec<Value> = file_types
        .iter()
        .map(|(content_type, count)| {
            let label = get_file_type_label(content_type);
            json!({
                "content_type": content_type,
                "label": label,
                "count": count
            })
        })
        .collect();

    Ok(Json(json!({
        "file_types": file_type_data,
        "total": total
    })))
}

/// Get friendly label for content type
fn get_file_type_label(content_type: &str) -> String {
    match content_type {
        "application/pdf" => "PDF".to_string(),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            "Documents".to_string()
        }
        "application/msword" => "Documents".to_string(),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            "Spreadsheets".to_string()
        }
        "application/vnd.ms-excel" => "Spreadsheets".to_string(),
        "text/plain" => "Text Files".to_string(),
        "text/csv" => "CSV".to_string(),
        "application/json" => "JSON".to_string(),
        "application/zip" => "Archives".to_string(),
        "application/x-rar-compressed" => "Archives".to_string(),
        ct if ct.starts_with("image/") => "Images".to_string(),
        ct if ct.starts_with("video/") => "Videos".to_string(),
        ct if ct.starts_with("audio/") => "Audio".to_string(),
        ct if ct.starts_with("text/") => "Text Files".to_string(),
        _ => "Other".to_string(),
    }
}

/// Format bytes into human-readable format
fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    const TB: i64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ==================== S3 Replication Admin Endpoints ====================

/// Get replication status
/// GET /api/admin/replication/status
/// Requires SuperAdmin role
pub async fn get_replication_status(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // SECURITY: SuperAdmin only
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let status = clovalink_core::replication::get_status(&state.store, &state.replication_config)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get replication status: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!(status)))
}

/// Query parameters for listing replication jobs
#[derive(Debug, Deserialize)]
pub struct ReplicationJobsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Get pending/failed replication jobs
/// GET /api/admin/replication/pending
/// Requires SuperAdmin role
pub async fn get_replication_jobs(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    axum::extract::Query(query): axum::extract::Query<ReplicationJobsQuery>,
) -> Result<Json<Value>, StatusCode> {
    // SECURITY: SuperAdmin only
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    let status_filter = query.status.as_deref();

    let jobs =
        clovalink_core::replication::get_pending_jobs(&state.store, status_filter, limit, offset)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get replication jobs: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    Ok(Json(json!({
        "jobs": jobs,
        "limit": limit,
        "offset": offset
    })))
}

/// Retry all failed replication jobs
/// POST /api/admin/replication/retry-failed
/// Requires SuperAdmin role
pub async fn retry_failed_jobs(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // SECURITY: SuperAdmin only
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let count = clovalink_core::replication::retry_failed_jobs(&state.store)
        .await
        .map_err(|e| {
            tracing::error!("Failed to retry failed jobs: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(target: "replication", "Reset {} failed jobs for retry", count);

    Ok(Json(json!({
        "message": "Failed jobs reset for retry",
        "jobs_reset": count
    })))
}
