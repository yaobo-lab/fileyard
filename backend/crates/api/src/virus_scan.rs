//! Virus Scanning API Handlers
//!
//! Admin endpoints for managing virus scanning settings, viewing results, and metrics.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use clovalink_auth::{require_admin, require_super_admin, AuthUser};
use clovalink_core::virus_scan::{
    self, ScanMetrics, TenantScanSettings,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

// =============================================================================
// Settings Endpoints
// =============================================================================

/// Get virus scan settings for current tenant
/// GET /api/admin/virus-scan/settings
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<TenantScanSettings>, StatusCode> {
    require_admin(&auth)?;

    let settings = virus_scan::get_tenant_settings(&state.pool, auth.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get virus scan settings: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(settings))
}

/// Update request body for virus scan settings
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub enabled: Option<bool>,
    pub file_types: Option<Vec<String>>,
    pub max_file_size_mb: Option<i32>,
    pub action_on_detect: Option<String>,
    pub notify_admin: Option<bool>,
    pub notify_uploader: Option<bool>,
    pub auto_suspend_uploader: Option<bool>,
    pub suspend_threshold: Option<i32>,
}

/// Update virus scan settings for current tenant
/// PUT /api/admin/virus-scan/settings
pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<TenantScanSettings>, StatusCode> {
    require_admin(&auth)?;

    // Validate action_on_detect if provided
    if let Some(ref action) = req.action_on_detect {
        if !["delete", "quarantine", "flag"].contains(&action.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let settings = virus_scan::update_tenant_settings(
        &state.pool,
        auth.tenant_id,
        req.enabled,
        req.file_types,
        req.max_file_size_mb,
        req.action_on_detect,
        req.notify_admin,
        req.notify_uploader,
        req.auto_suspend_uploader,
        req.suspend_threshold,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to update virus scan settings: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(settings))
}

// =============================================================================
// Metrics Endpoints
// =============================================================================

/// Get virus scan metrics (SuperAdmin only for global, Admin for tenant-specific)
/// GET /api/admin/virus-scan/metrics
pub async fn get_metrics(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<ScanMetrics>, StatusCode> {
    require_admin(&auth)?;

    // Get circuit breaker reference if available
    let cb_ref = state.clamav_circuit_breaker.as_ref().map(|cb| cb.as_ref());

    let metrics = virus_scan::get_metrics(&state.pool, &state.virus_scan_config, cb_ref)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get virus scan metrics: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(metrics))
}

// =============================================================================
// Scan Results Endpoints
// =============================================================================

/// Query params for scan history
#[derive(Debug, Deserialize)]
pub struct ScanHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub infected_only: Option<bool>,
}

/// Get scan history for current tenant
/// GET /api/admin/virus-scan/results
pub async fn get_scan_results(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<ScanHistoryQuery>,
) -> Result<Json<virus_scan::ScanHistoryResponse>, StatusCode> {
    require_admin(&auth)?;

    let limit = params.limit.unwrap_or(10).min(500) as i64;
    let offset = params.offset.unwrap_or(0) as i64;
    let infected_only = params.infected_only.unwrap_or(false);

    let results = virus_scan::get_scan_history(
        &state.pool,
        auth.tenant_id,
        limit,
        offset,
        infected_only,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get scan history: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(results))
}

// =============================================================================
// Quarantine Endpoints
// =============================================================================

/// Get quarantined files for current tenant
/// GET /api/admin/virus-scan/quarantine
pub async fn get_quarantined_files(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<ScanHistoryQuery>,
) -> Result<Json<virus_scan::QuarantineListResponse>, StatusCode> {
    require_admin(&auth)?;

    let limit = params.limit.unwrap_or(50).min(500) as i64;
    let offset = params.offset.unwrap_or(0) as i64;

    let results = virus_scan::get_quarantined_files(
        &state.pool,
        auth.tenant_id,
        limit,
        offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get quarantined files: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(results))
}

/// Delete a quarantined file permanently
/// DELETE /api/admin/virus-scan/quarantine/:id
pub async fn delete_quarantined_file(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Verify the quarantined file belongs to the tenant
    let exists: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM quarantined_files WHERE id = $1 AND tenant_id = $2 AND permanently_deleted_at IS NULL"
    )
    .bind(id)
    .bind(auth.tenant_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify quarantined file: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Mark as permanently deleted
    sqlx::query(
        "UPDATE quarantined_files SET permanently_deleted_at = NOW(), deleted_by = $2 WHERE id = $1"
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete quarantined file: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({ "message": "Quarantined file permanently deleted" })))
}

// =============================================================================
// Manual Scan Endpoints
// =============================================================================

/// Trigger a manual rescan of a file
/// POST /api/admin/virus-scan/rescan/:file_id
pub async fn rescan_file(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(file_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Check if virus scanning is enabled
    if !state.virus_scan_config.enabled {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Verify file exists and belongs to tenant
    let file_exists: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM files_metadata WHERE id = $1 AND tenant_id = $2 AND is_deleted = false"
    )
    .bind(file_id)
    .bind(auth.tenant_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to verify file: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if file_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Enqueue scan job with high priority
    let job_id = virus_scan::enqueue_scan(&state.pool, file_id, auth.tenant_id, 100)
        .await
        .map_err(|e| {
            tracing::error!("Failed to enqueue rescan: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Reset file scan status
    virus_scan::update_file_scan_status(&state.pool, file_id, "pending")
        .await
        .map_err(|e| {
            tracing::error!("Failed to update file scan status: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "message": "Rescan queued",
        "job_id": job_id
    })))
}

// =============================================================================
// Global Config (SuperAdmin only)
// =============================================================================

/// Get global virus scan configuration
/// GET /api/admin/virus-scan/config
pub async fn get_global_config(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    Ok(Json(json!({
        "enabled": state.virus_scan_config.enabled,
        "host": state.virus_scan_config.host,
        "port": state.virus_scan_config.port,
        "timeout_ms": state.virus_scan_config.timeout_ms,
        "workers": state.virus_scan_config.workers,
        "max_file_size_mb": state.virus_scan_config.max_file_size_mb,
    })))
}


