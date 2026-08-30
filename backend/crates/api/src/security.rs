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
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::AppState;
use clovalink_auth::AuthUser;

#[derive(Debug, Serialize, FromRow)]
pub struct SecurityAlert {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: Option<String>,
    pub metadata: Value,
    pub ip_address: Option<String>,
    pub resolved: bool,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AlertsQuery {
    pub severity: Option<String>,
    pub alert_type: Option<String>,
    pub resolved: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AlertStats {
    pub total: i64,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub unresolved: i64,
    pub by_type: Vec<TypeCount>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TypeCount {
    pub alert_type: String,
    pub count: i64,
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

    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    // Build query based on role
    let is_superadmin = auth.role == "SuperAdmin";

    let alerts: Vec<SecurityAlert> = if is_superadmin {
        // SuperAdmin sees all alerts
        sqlx::query_as::<_, SecurityAlert>(
            r#"
            SELECT 
                sa.id, sa.tenant_id, sa.user_id, sa.alert_type, sa.severity,
                sa.title, sa.description, sa.metadata, 
                sa.ip_address::text as ip_address,
                sa.resolved, sa.resolved_by, sa.resolved_at, sa.created_at
            FROM security_alerts sa
            WHERE ($1::text IS NULL OR sa.severity = $1)
            AND ($2::text IS NULL OR sa.alert_type = $2)
            AND ($3::boolean IS NULL OR sa.resolved = $3)
            ORDER BY 
                CASE sa.severity 
                    WHEN 'critical' THEN 1 
                    WHEN 'high' THEN 2 
                    WHEN 'medium' THEN 3 
                    ELSE 4 
                END,
                sa.created_at DESC
            LIMIT $4 OFFSET $5
            "#
        )
        .bind(&params.severity)
        .bind(&params.alert_type)
        .bind(params.resolved)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch alerts: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        // Admin sees only their tenant's alerts
        sqlx::query_as::<_, SecurityAlert>(
            r#"
            SELECT 
                sa.id, sa.tenant_id, sa.user_id, sa.alert_type, sa.severity,
                sa.title, sa.description, sa.metadata, 
                sa.ip_address::text as ip_address,
                sa.resolved, sa.resolved_by, sa.resolved_at, sa.created_at
            FROM security_alerts sa
            WHERE sa.tenant_id = $1
            AND ($2::text IS NULL OR sa.severity = $2)
            AND ($3::text IS NULL OR sa.alert_type = $3)
            AND ($4::boolean IS NULL OR sa.resolved = $4)
            ORDER BY 
                CASE sa.severity 
                    WHEN 'critical' THEN 1 
                    WHEN 'high' THEN 2 
                    WHEN 'medium' THEN 3 
                    ELSE 4 
                END,
                sa.created_at DESC
            LIMIT $5 OFFSET $6
            "#
        )
        .bind(auth.tenant_id)
        .bind(&params.severity)
        .bind(&params.alert_type)
        .bind(params.resolved)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch alerts: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    // Get total count for pagination
    let total: (i64,) = if is_superadmin {
        sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM security_alerts
            WHERE ($1::text IS NULL OR severity = $1)
            AND ($2::text IS NULL OR alert_type = $2)
            AND ($3::boolean IS NULL OR resolved = $3)
            "#
        )
        .bind(&params.severity)
        .bind(&params.alert_type)
        .bind(params.resolved)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM security_alerts
            WHERE tenant_id = $1
            AND ($2::text IS NULL OR severity = $2)
            AND ($3::text IS NULL OR alert_type = $3)
            AND ($4::boolean IS NULL OR resolved = $4)
            "#
        )
        .bind(auth.tenant_id)
        .bind(&params.severity)
        .bind(&params.alert_type)
        .bind(params.resolved)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Enrich alerts with user email and tenant name
    let mut enriched_alerts = Vec::new();
    for alert in alerts {
        let user_email: Option<String> = if let Some(uid) = alert.user_id {
            sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                .bind(uid)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let tenant_name: Option<String> = if let Some(tid) = alert.tenant_id {
            sqlx::query_scalar("SELECT name FROM tenants WHERE id = $1")
                .bind(tid)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let resolved_by_email: Option<String> = if let Some(rid) = alert.resolved_by {
            sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                .bind(rid)
                .fetch_optional(&state.pool)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        enriched_alerts.push(json!({
            "id": alert.id,
            "tenant_id": alert.tenant_id,
            "tenant_name": tenant_name,
            "user_id": alert.user_id,
            "user_email": user_email,
            "alert_type": alert.alert_type,
            "severity": alert.severity,
            "title": alert.title,
            "description": alert.description,
            "metadata": alert.metadata,
            "ip_address": alert.ip_address,
            "resolved": alert.resolved,
            "resolved_by": alert.resolved_by,
            "resolved_by_email": resolved_by_email,
            "resolved_at": alert.resolved_at,
            "created_at": alert.created_at,
        }));
    }

    Ok(Json(json!({
        "alerts": enriched_alerts,
        "total": total.0,
        "limit": limit,
        "offset": offset
    })))
}

/// Get alert statistics
/// GET /api/security/alerts/stats
pub async fn get_alert_stats(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<AlertStats>, StatusCode> {
    // Only Admin and SuperAdmin can view security stats
    if !matches!(auth.role.as_str(), "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    let is_superadmin = auth.role == "SuperAdmin";

    // Get counts by severity
    let (total, critical, high, medium, low, unresolved): (i64, i64, i64, i64, i64, i64) = if is_superadmin {
        sqlx::query_as(
            r#"
            SELECT 
                COUNT(*),
                COUNT(*) FILTER (WHERE severity = 'critical'),
                COUNT(*) FILTER (WHERE severity = 'high'),
                COUNT(*) FILTER (WHERE severity = 'medium'),
                COUNT(*) FILTER (WHERE severity = 'low'),
                COUNT(*) FILTER (WHERE resolved = false)
            FROM security_alerts
            "#
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query_as(
            r#"
            SELECT 
                COUNT(*),
                COUNT(*) FILTER (WHERE severity = 'critical'),
                COUNT(*) FILTER (WHERE severity = 'high'),
                COUNT(*) FILTER (WHERE severity = 'medium'),
                COUNT(*) FILTER (WHERE severity = 'low'),
                COUNT(*) FILTER (WHERE resolved = false)
            FROM security_alerts
            WHERE tenant_id = $1
            "#
        )
        .bind(auth.tenant_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Get counts by type
    let by_type: Vec<TypeCount> = if is_superadmin {
        sqlx::query_as(
            r#"
            SELECT alert_type, COUNT(*) as count
            FROM security_alerts
            WHERE resolved = false
            GROUP BY alert_type
            ORDER BY count DESC
            "#
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query_as(
            r#"
            SELECT alert_type, COUNT(*) as count
            FROM security_alerts
            WHERE tenant_id = $1 AND resolved = false
            GROUP BY alert_type
            ORDER BY count DESC
            "#
        )
        .bind(auth.tenant_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    Ok(Json(AlertStats {
        total,
        critical,
        high,
        medium,
        low,
        unresolved,
        by_type,
    }))
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
    let alert: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT tenant_id FROM security_alerts WHERE id = $1"
    )
    .bind(alert_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let alert = alert.ok_or(StatusCode::NOT_FOUND)?;

    // Non-SuperAdmins can only resolve their own tenant's alerts
    if auth.role != "SuperAdmin" {
        if let Some(tenant_id) = alert.0 {
            if tenant_id != auth.tenant_id {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // Update the alert
    sqlx::query(
        r#"
        UPDATE security_alerts 
        SET resolved = true, resolved_by = $1, resolved_at = NOW()
        WHERE id = $2
        "#
    )
    .bind(auth.user_id)
    .bind(alert_id)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, resource_id, ip_address)
        VALUES ($1, $2, 'security_alert_resolved', 'security_alert', $3, $4::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(alert_id.to_string())
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await
    .ok();

    Ok(Json(json!({ "success": true, "message": "Alert resolved" })))
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
    let alert: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT tenant_id FROM security_alerts WHERE id = $1"
    )
    .bind(alert_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let alert = alert.ok_or(StatusCode::NOT_FOUND)?;

    // Non-SuperAdmins can only dismiss their own tenant's alerts
    if auth.role != "SuperAdmin" {
        if let Some(tenant_id) = alert.0 {
            if tenant_id != auth.tenant_id {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // Delete the alert (dismissed = removed)
    sqlx::query("DELETE FROM security_alerts WHERE id = $1")
        .bind(alert_id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, resource_id, ip_address)
        VALUES ($1, $2, 'security_alert_dismissed', 'security_alert', $3, $4::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(alert_id.to_string())
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await
    .ok();

    Ok(Json(json!({ "success": true, "message": "Alert dismissed" })))
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
        let invalid_count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM security_alerts 
            WHERE id = ANY($1) AND (tenant_id IS NULL OR tenant_id != $2)
            "#
        )
        .bind(&payload.ids)
        .bind(auth.tenant_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if invalid_count.0 > 0 {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let affected = match payload.action.as_str() {
        "resolve" => {
            let result = if is_superadmin {
                sqlx::query(
                    r#"
                    UPDATE security_alerts 
                    SET resolved = true, resolved_by = $1, resolved_at = NOW()
                    WHERE id = ANY($2)
                    "#
                )
                .bind(auth.user_id)
                .bind(&payload.ids)
                .execute(&state.pool)
                .await
            } else {
                sqlx::query(
                    r#"
                    UPDATE security_alerts 
                    SET resolved = true, resolved_by = $1, resolved_at = NOW()
                    WHERE id = ANY($2) AND tenant_id = $3
                    "#
                )
                .bind(auth.user_id)
                .bind(&payload.ids)
                .bind(auth.tenant_id)
                .execute(&state.pool)
                .await
            };
            result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.rows_affected()
        }
        "dismiss" => {
            let result = if is_superadmin {
                sqlx::query("DELETE FROM security_alerts WHERE id = ANY($1)")
                    .bind(&payload.ids)
                    .execute(&state.pool)
                    .await
            } else {
                sqlx::query("DELETE FROM security_alerts WHERE id = ANY($1) AND tenant_id = $2")
                    .bind(&payload.ids)
                    .bind(auth.tenant_id)
                    .execute(&state.pool)
                    .await
            };
            result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.rows_affected()
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Audit log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, metadata, ip_address)
        VALUES ($1, $2, $3, 'security_alert', $4, $5::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(format!("security_alert_bulk_{}", payload.action))
    .bind(json!({ "ids": payload.ids, "count": affected }))
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await
    .ok();

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

    let count: (i64,) = if auth.role == "SuperAdmin" {
        sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM security_alerts
            WHERE resolved = false AND severity IN ('critical', 'high')
            "#
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM security_alerts
            WHERE tenant_id = $1 AND resolved = false AND severity IN ('critical', 'high')
            "#
        )
        .bind(auth.tenant_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    Ok(Json(json!({ "count": count.0 })))
}

