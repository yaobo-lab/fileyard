use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use clovalink_auth::{AuthUser, require_admin};
use clovalink_core::notification_service::{Notification, NotificationPreference};
use crate::AppState;

// ==================== Tenant Notification Settings ====================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantNotificationSetting {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub enabled: bool,
    pub email_enforced: bool,
    pub in_app_enforced: bool,
    pub default_email: bool,
    pub default_in_app: bool,
    pub role: Option<String>,  // NULL = all roles, specific value = role-specific
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TenantSettingsQuery {
    pub role: Option<String>,  // Filter by role (NULL for global settings)
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantSettingsInput {
    pub settings: Vec<TenantSettingUpdate>,
    pub role: Option<String>,  // Which role to update (NULL for global)
}

#[derive(Debug, Deserialize)]
pub struct TenantSettingUpdate {
    pub event_type: String,
    pub enabled: Option<bool>,
    pub email_enforced: Option<bool>,
    pub in_app_enforced: Option<bool>,
    pub default_email: Option<bool>,
    pub default_in_app: Option<bool>,
}

// ==================== Query Parameters ====================

#[derive(Debug, Deserialize)]
pub struct ListNotificationsQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub unread_only: Option<bool>,
}

// ==================== Request Bodies ====================

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesInput {
    pub preferences: Vec<PreferenceUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct PreferenceUpdate {
    pub event_type: String,
    pub email_enabled: Option<bool>,
    pub in_app_enabled: Option<bool>,
}

// ==================== Response Types ====================

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub notifications: Vec<Notification>,
    pub total: i64,
    pub unread_count: i64,
    pub page: i64,
    pub limit: i64,
}

// ==================== Handlers ====================

/// List notifications for the current user
/// GET /api/notifications
pub async fn list_notifications(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<ListNotificationsQuery>,
) -> Result<Json<NotificationListResponse>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = (page - 1) * limit;
    let unread_only = params.unread_only.unwrap_or(false);

    // Get notifications
    let notifications: Vec<Notification> = if unread_only {
        sqlx::query_as(
            r#"
            SELECT * FROM notifications 
            WHERE user_id = $1 AND is_read = false
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(auth.user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch notifications: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        sqlx::query_as(
            r#"
            SELECT * FROM notifications 
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(auth.user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch notifications: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    // Get total count
    let total: (i64,) = if unread_only {
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false")
            .bind(auth.user_id)
            .fetch_one(&state.pool)
            .await
            .unwrap_or((0,))
    } else {
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id = $1")
            .bind(auth.user_id)
            .fetch_one(&state.pool)
            .await
            .unwrap_or((0,))
    };

    // Get unread count
    let unread_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or((0,));

    Ok(Json(NotificationListResponse {
        notifications,
        total: total.0,
        unread_count: unread_count.0,
        page,
        limit,
    }))
}

/// Get unread notification count
/// GET /api/notifications/unread-count
pub async fn get_unread_count(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or((0,));

    Ok(Json(json!({ "unread_count": count.0 })))
}

/// Mark a notification as read
/// PUT /api/notifications/:id/read
pub async fn mark_as_read(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = true WHERE id = $1 AND user_id = $2"
    )
    .bind(notification_id)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to mark notification as read: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(json!({ "success": true })))
}

/// Mark all notifications as read
/// PUT /api/notifications/read-all
pub async fn mark_all_as_read(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = true WHERE user_id = $1 AND is_read = false"
    )
    .bind(auth.user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to mark all notifications as read: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({ 
        "success": true, 
        "marked_count": result.rows_affected() 
    })))
}

/// Delete a notification
/// DELETE /api/notifications/:id
pub async fn delete_notification(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query(
        "DELETE FROM notifications WHERE id = $1 AND user_id = $2"
    )
    .bind(notification_id)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete notification: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(json!({ "success": true })))
}

/// Get notification preferences
/// GET /api/notifications/preferences
pub async fn get_preferences(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Vec<NotificationPreference>>, StatusCode> {
    // Get existing preferences
    let preferences: Vec<NotificationPreference> = sqlx::query_as(
        "SELECT * FROM notification_preferences WHERE user_id = $1 ORDER BY event_type"
    )
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch preferences: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // If no preferences exist, create defaults
    if preferences.is_empty() {
        let event_types = vec![
            "file_upload",
            "request_expiring",
            "user_action",
            "compliance_alert",
            "storage_warning",
            "file_shared",
        ];

        for event_type in event_types {
            let _ = sqlx::query(
                r#"
                INSERT INTO notification_preferences (user_id, event_type, email_enabled, in_app_enabled)
                VALUES ($1, $2, true, true)
                ON CONFLICT (user_id, event_type) DO NOTHING
                "#
            )
            .bind(auth.user_id)
            .bind(event_type)
            .execute(&state.pool)
            .await;
        }

        // Fetch again
        let preferences: Vec<NotificationPreference> = sqlx::query_as(
            "SELECT * FROM notification_preferences WHERE user_id = $1 ORDER BY event_type"
        )
        .bind(auth.user_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch preferences: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        return Ok(Json(preferences));
    }

    Ok(Json(preferences))
}

/// Update notification preferences
/// PUT /api/notifications/preferences
pub async fn update_preferences(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<UpdatePreferencesInput>,
) -> Result<Json<Vec<NotificationPreference>>, StatusCode> {
    for pref in input.preferences {
        // Build dynamic update query
        let mut updates = Vec::new();
        let mut bind_index = 3; // user_id is $1, event_type is $2
        
        if pref.email_enabled.is_some() {
            updates.push(format!("email_enabled = ${}", bind_index));
            bind_index += 1;
        }
        if pref.in_app_enabled.is_some() {
            updates.push(format!("in_app_enabled = ${}", bind_index));
        }

        if updates.is_empty() {
            continue;
        }

        updates.push("updated_at = NOW()".to_string());
        let update_clause = updates.join(", ");

        // We need to use a dynamic query here
        let query = format!(
            r#"
            INSERT INTO notification_preferences (user_id, event_type, email_enabled, in_app_enabled)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, event_type) 
            DO UPDATE SET {}
            "#,
            update_clause
        );

        sqlx::query(&query)
            .bind(auth.user_id)
            .bind(&pref.event_type)
            .bind(pref.email_enabled.unwrap_or(true))
            .bind(pref.in_app_enabled.unwrap_or(true))
            .execute(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update preference: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Return updated preferences
    get_preferences(State(state), Extension(auth)).await
}

/// Get notification preference labels for UI
/// GET /api/notifications/preference-labels
pub async fn get_preference_labels(
) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!([
        {
            "event_type": "file_upload",
            "label": "File Uploads",
            "description": "Notifications when files are uploaded to your file requests"
        },
        {
            "event_type": "request_expiring",
            "label": "Expiring Requests",
            "description": "Reminders when your file requests are about to expire"
        },
        {
            "event_type": "user_action",
            "label": "User Actions",
            "description": "Notifications about new users and role changes (Admin only)"
        },
        {
            "event_type": "compliance_alert",
            "label": "Compliance Alerts",
            "description": "Important compliance-related notifications (Admin only)"
        },
        {
            "event_type": "storage_warning",
            "label": "Storage Warnings",
            "description": "Alerts when storage quota is running low (Admin only)"
        },
        {
            "event_type": "file_shared",
            "label": "File Sharing",
            "description": "Notifications when files are shared with you"
        }
    ])))
}

// ==================== Tenant Notification Settings Handlers ====================

/// Get tenant notification settings
/// GET /api/tenants/:id/notification-settings?role=Admin
pub async fn get_tenant_notification_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(tenant_id): Path<Uuid>,
    Query(query): Query<TenantSettingsQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Check permissions - must be admin of the tenant
    require_admin(&auth)?;
    
    // Verify user has access to this tenant
    if auth.tenant_id != tenant_id && auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let event_types = vec![
        "file_upload",
        "request_expiring",
        "user_action",
        "compliance_alert",
        "storage_warning",
        "file_shared",
    ];

    // Get all settings for this tenant (global + role-specific)
    let all_settings: Vec<TenantNotificationSetting> = sqlx::query_as(
        "SELECT * FROM tenant_notification_settings WHERE tenant_id = $1 ORDER BY role NULLS FIRST, event_type"
    )
    .bind(tenant_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch tenant notification settings: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // If no global settings exist, create defaults
    let has_global = all_settings.iter().any(|s| s.role.is_none());
    if !has_global {
        for event_type in &event_types {
            let _ = sqlx::query(
                r#"
                INSERT INTO tenant_notification_settings 
                    (tenant_id, event_type, enabled, email_enforced, in_app_enforced, default_email, default_in_app, role)
                VALUES ($1, $2, true, false, false, true, true, NULL)
                ON CONFLICT (tenant_id, event_type, role) DO NOTHING
                "#
            )
            .bind(tenant_id)
            .bind(*event_type)
            .execute(&state.pool)
            .await;
        }
    }

    // Re-fetch all settings
    let all_settings: Vec<TenantNotificationSetting> = sqlx::query_as(
        "SELECT * FROM tenant_notification_settings WHERE tenant_id = $1 ORDER BY role NULLS FIRST, event_type"
    )
    .bind(tenant_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch tenant notification settings: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Group settings by role
    let global_settings: Vec<&TenantNotificationSetting> = all_settings.iter()
        .filter(|s| s.role.is_none())
        .collect();
    
    let role_settings: std::collections::HashMap<String, Vec<&TenantNotificationSetting>> = all_settings.iter()
        .filter(|s| s.role.is_some())
        .fold(std::collections::HashMap::new(), |mut acc, s| {
            if let Some(ref role) = s.role {
                acc.entry(role.clone()).or_insert_with(Vec::new).push(s);
            }
            acc
        });

    // If a specific role is requested, return just those settings with inheritance info
    if let Some(ref role) = query.role {
        let role_specific = role_settings.get(role).cloned().unwrap_or_default();
        
        // Build effective settings for this role (merging with global)
        let effective: Vec<Value> = event_types.iter().map(|et| {
            let global = global_settings.iter().find(|s| s.event_type == *et);
            let specific = role_specific.iter().find(|s| s.event_type == *et);
            
            let (setting, inherited) = match (specific, global) {
                (Some(s), _) => (Some(*s), false),
                (None, Some(g)) => (Some(*g), true),
                (None, None) => (None, true),
            };
            
            if let Some(s) = setting {
                json!({
                    "id": s.id,
                    "event_type": s.event_type,
                    "enabled": s.enabled,
                    "email_enforced": s.email_enforced,
                    "in_app_enforced": s.in_app_enforced,
                    "default_email": s.default_email,
                    "default_in_app": s.default_in_app,
                    "role": role,
                    "inherited": inherited
                })
            } else {
                json!({
                    "event_type": et,
                    "enabled": true,
                    "email_enforced": false,
                    "in_app_enforced": false,
                    "default_email": true,
                    "default_in_app": true,
                    "role": role,
                    "inherited": true
                })
            }
        }).collect();
        
        return Ok(Json(json!({
            "role": role,
            "settings": effective
        })));
    }

    // Return all settings grouped by role
    Ok(Json(json!({
        "global": global_settings,
        "by_role": role_settings,
        "available_roles": ["Admin", "Manager", "Employee"]
    })))
}

/// Update tenant notification settings
/// PUT /api/tenants/:id/notification-settings
pub async fn update_tenant_notification_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<UpdateTenantSettingsInput>,
) -> Result<Json<Value>, StatusCode> {
    // Check permissions - must be admin of the tenant
    require_admin(&auth)?;
    
    // Verify user has access to this tenant
    if auth.tenant_id != tenant_id && auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Role from input (None = global settings)
    let target_role = input.role.clone();

    for setting in input.settings {
        // Use different query based on whether role is NULL or not
        if target_role.is_some() {
            sqlx::query(
                r#"
                INSERT INTO tenant_notification_settings 
                    (tenant_id, event_type, enabled, email_enforced, in_app_enforced, default_email, default_in_app, role)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (tenant_id, event_type, role) 
                DO UPDATE SET 
                    enabled = COALESCE($3, tenant_notification_settings.enabled),
                    email_enforced = COALESCE($4, tenant_notification_settings.email_enforced),
                    in_app_enforced = COALESCE($5, tenant_notification_settings.in_app_enforced),
                    default_email = COALESCE($6, tenant_notification_settings.default_email),
                    default_in_app = COALESCE($7, tenant_notification_settings.default_in_app),
                    updated_at = NOW()
                "#
            )
            .bind(tenant_id)
            .bind(&setting.event_type)
            .bind(setting.enabled.unwrap_or(true))
            .bind(setting.email_enforced.unwrap_or(false))
            .bind(setting.in_app_enforced.unwrap_or(false))
            .bind(setting.default_email.unwrap_or(true))
            .bind(setting.default_in_app.unwrap_or(true))
            .bind(&target_role)
            .execute(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update tenant notification setting: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        } else {
            // For NULL role, we need special handling for the unique constraint
            sqlx::query(
                r#"
                INSERT INTO tenant_notification_settings 
                    (tenant_id, event_type, enabled, email_enforced, in_app_enforced, default_email, default_in_app, role)
                VALUES ($1, $2, $3, $4, $5, $6, $7, NULL)
                ON CONFLICT (tenant_id, event_type, role) WHERE role IS NULL
                DO UPDATE SET 
                    enabled = COALESCE($3, tenant_notification_settings.enabled),
                    email_enforced = COALESCE($4, tenant_notification_settings.email_enforced),
                    in_app_enforced = COALESCE($5, tenant_notification_settings.in_app_enforced),
                    default_email = COALESCE($6, tenant_notification_settings.default_email),
                    default_in_app = COALESCE($7, tenant_notification_settings.default_in_app),
                    updated_at = NOW()
                "#
            )
            .bind(tenant_id)
            .bind(&setting.event_type)
            .bind(setting.enabled.unwrap_or(true))
            .bind(setting.email_enforced.unwrap_or(false))
            .bind(setting.in_app_enforced.unwrap_or(false))
            .bind(setting.default_email.unwrap_or(true))
            .bind(setting.default_in_app.unwrap_or(true))
            .execute(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update tenant notification setting: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
    }

    // Return updated settings for the same role
    let query = TenantSettingsQuery { role: target_role };
    get_tenant_notification_settings(State(state), Extension(auth), Path(tenant_id), Query(query)).await
}

/// Get user preferences with company settings overlay
/// GET /api/notifications/preferences-with-company
/// Returns effective settings for the user based on their role
pub async fn get_preferences_with_company_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // Get user preferences
    let user_prefs: Vec<NotificationPreference> = sqlx::query_as(
        "SELECT * FROM notification_preferences WHERE user_id = $1 ORDER BY event_type"
    )
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch user preferences: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // SuperAdmins are exempt from company-level notification controls
    if auth.role == "SuperAdmin" {
        return Ok(Json(json!({
            "preferences": user_prefs,
            "company_settings": {},
            "is_exempt": true
        })));
    }

    // Get all company settings (global + role-specific)
    let all_settings: Vec<TenantNotificationSetting> = sqlx::query_as(
        "SELECT * FROM tenant_notification_settings WHERE tenant_id = $1 ORDER BY role NULLS FIRST, event_type"
    )
    .bind(auth.tenant_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    // Get role-specific settings first, then fall back to global
    let event_types = vec![
        "file_upload",
        "request_expiring", 
        "user_action",
        "compliance_alert",
        "storage_warning",
        "file_shared",
    ];

    // Build effective company settings map for the user's role
    let mut company_settings_map = serde_json::Map::new();
    
    for event_type in event_types {
        // Find role-specific setting first
        let role_setting = all_settings.iter()
            .find(|s| s.event_type == event_type && s.role.as_deref() == Some(&auth.role));
        
        // Fall back to global setting
        let global_setting = all_settings.iter()
            .find(|s| s.event_type == event_type && s.role.is_none());
        
        let effective = role_setting.or(global_setting);
        
        if let Some(s) = effective {
            company_settings_map.insert(event_type.to_string(), json!({
                "enabled": s.enabled,
                "email_enforced": s.email_enforced,
                "in_app_enforced": s.in_app_enforced,
                "default_email": s.default_email,
                "default_in_app": s.default_in_app,
                "role_specific": role_setting.is_some()
            }));
        }
    }

    Ok(Json(json!({
        "preferences": user_prefs,
        "company_settings": company_settings_map,
        "user_role": auth.role,
        "is_exempt": false
    })))
}
