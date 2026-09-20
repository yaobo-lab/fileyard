//! Security API - Endpoints for managing security alerts
//!
//! - GET /api/security/alerts - List alerts (filtered by tenant for Admins)
//! - GET /api/security/alerts/stats - Summary counts by severity/type
//! - POST /api/security/alerts/{id}/resolve - Mark alert as resolved
//! - POST /api/security/alerts/{id}/dismiss - Dismiss false positive

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use crate::auth::AuthUser;
use app_entity::{AlertQueryFilter, AlertStatsResult};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct AlertsQuery {
    pub severity: Option<String>,
    pub alert_type: Option<String>,
    pub resolved: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List security alerts
/// GET /api/security/alerts
/// SuperAdmin sees all, Admin sees only their tenant
pub async fn list_alerts(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<AlertsQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Only Admin and SuperAdmin can view security alerts
    if !matches!(auth.role.as_str(), "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = params.limit.unwrap_or(50).clamp(1, 100) as u64;
    let offset = params.offset.unwrap_or(0).max(0) as u64;
    let tenant_id = if auth.role == "SuperAdmin" {
        None
    } else {
        Some(auth.tenant_id)
    };

    let (alerts, total) = state
        .store
        .security()
        .list_enriched_alerts(AlertQueryFilter {
            tenant_id,
            severity: params.severity,
            alert_type: params.alert_type,
            resolved: params.resolved,
            limit,
            offset,
        })
        .await
        .map_err(|e| {
            log::error!("Failed to fetch alerts: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "alerts": alerts,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}

/// Get alert statistics
/// GET /api/security/alerts/stats
pub async fn get_alert_stats(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<AlertStatsResult>, StatusCode> {
    // Only Admin and SuperAdmin can view security stats
    if !matches!(auth.role.as_str(), "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    let tenant_id = if auth.role == "SuperAdmin" {
        None
    } else {
        Some(auth.tenant_id)
    };

    let stats = state
        .store
        .security()
        .get_stats(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(stats))
}

/// Resolve an alert
/// POST /api/security/alerts/{id}/resolve
pub async fn resolve_alert(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Only Admin and SuperAdmin can resolve alerts
    if !matches!(auth.role.as_str(), "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify access to this alert
    let alert = state
        .store
        .security()
        .get_tenant_id(alert_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Non-SuperAdmins can only resolve their own tenant's alerts
    if auth.role != "SuperAdmin" {
        if let Some(tenant_id) = alert {
            if tenant_id != auth.tenant_id {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // Update the alert
    state
        .store
        .security()
        .resolve(alert_id, auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            "security_alert_resolved",
            "security_alert",
            Some(alert_id),
            None,
            auth.ip_address.clone(),
        )
        .await;

    Ok(Json(
        json!({ "success": true, "message": "Alert resolved" }),
    ))
}

/// Dismiss an alert (mark as false positive)
/// POST /api/security/alerts/{id}/dismiss
pub async fn dismiss_alert(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Only Admin and SuperAdmin can dismiss alerts
    if !matches!(auth.role.as_str(), "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify access to this alert
    let alert = state
        .store
        .security()
        .get_tenant_id(alert_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Non-SuperAdmins can only dismiss their own tenant's alerts
    if auth.role != "SuperAdmin" {
        if let Some(tenant_id) = alert {
            if tenant_id != auth.tenant_id {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // Delete the alert (dismissed = removed)
    state
        .store
        .security()
        .dismiss(alert_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            "security_alert_dismissed",
            "security_alert",
            Some(alert_id),
            None,
            auth.ip_address.clone(),
        )
        .await;

    Ok(Json(
        json!({ "success": true, "message": "Alert dismissed" }),
    ))
}

/// Bulk action on alerts
/// POST /api/security/alerts/bulk
#[derive(Debug, Deserialize)]
pub struct BulkAlertAction {
    pub ids: Vec<Uuid>,
    pub action: String, // "resolve" or "dismiss"
}

pub async fn bulk_alert_action(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<BulkAlertAction>,
) -> Result<Json<Value>, StatusCode> {
    // Only Admin and SuperAdmin can perform bulk actions
    if !matches!(auth.role.as_str(), "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    if payload.ids.is_empty() {
        return Ok(Json(json!({ "success": true, "affected": 0 })));
    }

    let is_superadmin = auth.role == "SuperAdmin";

    // For non-SuperAdmins, verify all alerts belong to their tenant
    if !is_superadmin {
        let invalid_count = state
            .store
            .security()
            .count_invalid_tenant_alerts(&payload.ids, auth.tenant_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if invalid_count > 0 {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let tenant_id = if is_superadmin {
        None
    } else {
        Some(auth.tenant_id)
    };

    let affected = match payload.action.as_str() {
        "resolve" => state
            .store
            .security()
            .bulk_resolve(&payload.ids, auth.user_id, tenant_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        "dismiss" => state
            .store
            .security()
            .bulk_dismiss(&payload.ids, tenant_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Audit log
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            format!("security_alert_bulk_{}", payload.action),
            "security_alert",
            None,
            Some(json!({ "ids": payload.ids, "count": affected })),
            auth.ip_address.clone(),
        )
        .await;

    Ok(Json(json!({
        "success": true,
        "affected": affected,
        "message": format!("{} alerts {}", affected, if payload.action == "resolve" { "resolved" } else { "dismissed" })
    })))
}

/// Get count of unresolved critical/high alerts (for badge)
/// GET /api/security/alerts/badge
pub async fn get_alert_badge(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // Only Admin and SuperAdmin
    if !matches!(auth.role.as_str(), "Admin" | "SuperAdmin") {
        return Ok(Json(json!({ "count": 0 })));
    }

    let tenant_id = if auth.role == "SuperAdmin" {
        None
    } else {
        Some(auth.tenant_id)
    };

    let count = state
        .store
        .security()
        .unresolved_badge_count(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "count": count })))
}
