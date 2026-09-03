use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::{DateTime, Utc};
use clovalink_auth::{require_admin, AuthUser};
use clovalink_core::notification_service::{Notification, NotificationPreference};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

// ==================== Tenant Notification Settings ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantNotificationSetting {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub enabled: bool,
    pub email_enforced: bool,
    pub in_app_enforced: bool,
    pub default_email: bool,
    pub default_in_app: bool,
    pub role: Option<String>, // NULL = all roles, specific value = role-specific
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TenantSettingsQuery {
    pub role: Option<String>, // Filter by role (NULL for global settings)
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantSettingsInput {
    pub settings: Vec<TenantSettingUpdate>,
    pub role: Option<String>, // Which role to update (NULL for global)
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

fn notification(m: clovalink_entity::entities::notifications::Model) -> Notification {
    Notification {
        id: m.id,
        user_id: m.user_id,
        tenant_id: m.tenant_id,
        notification_type: m.notification_type,
        title: m.title,
        message: m.message,
        metadata: m.metadata.unwrap_or_else(|| json!({})),
        is_read: m.is_read,
        email_sent: m.email_sent,
        created_at: m.created_at.into(),
    }
}
fn preference(
    m: clovalink_entity::entities::notification_preferences::Model,
) -> NotificationPreference {
    NotificationPreference {
        id: m.id,
        user_id: m.user_id,
        event_type: m.event_type,
        email_enabled: m.email_enabled,
        in_app_enabled: m.in_app_enabled,
        created_at: m.created_at.into(),
        updated_at: m.updated_at.into(),
    }
}
fn tenant_setting(
    m: clovalink_entity::entities::tenant_notification_settings::Model,
) -> TenantNotificationSetting {
    TenantNotificationSetting {
        id: m.id,
        tenant_id: m.tenant_id,
        event_type: m.event_type,
        enabled: m.enabled,
        email_enforced: m.email_enforced,
        in_app_enforced: m.in_app_enforced,
        default_email: m.default_email,
        default_in_app: m.default_in_app,
        role: m.role,
        created_at: m.created_at.into(),
        updated_at: m.updated_at.into(),
    }
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

    let (rows, total, unread_count) = state
        .store
        .notifications()
        .list(auth.user_id, unread_only, limit as u64, offset as u64)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let notifications = rows.into_iter().map(notification).collect();

    Ok(Json(NotificationListResponse {
        notifications,
        total,
        unread_count,
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
    let (_, _, count) = state
        .store
        .notifications()
        .list(auth.user_id, true, 1, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "unread_count": count })))
}

/// Mark a notification as read
/// PUT /api/notifications/:id/read
pub async fn mark_as_read(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    if !state
        .store
        .notifications()
        .mark_read(notification_id, auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
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
    let marked = state
        .store
        .notifications()
        .mark_all_read(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "success": true,
        "marked_count": marked
    })))
}

/// Delete a notification
/// DELETE /api/notifications/:id
pub async fn delete_notification(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    if !state
        .store
        .notifications()
        .delete(notification_id, auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
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
    let mut preferences = state
        .store
        .notifications()
        .preferences(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // If no preferences exist, create defaults
    if preferences.is_empty() {
        let event_types = [
            "file_upload",
            "request_expiring",
            "user_action",
            "compliance_alert",
            "storage_warning",
            "file_shared",
        ];

        state
            .store
            .notifications()
            .ensure_default_preferences(auth.user_id, &event_types)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        preferences = state
            .store
            .notifications()
            .preferences(auth.user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(preferences.into_iter().map(preference).collect()))
}

/// Update notification preferences
/// PUT /api/notifications/preferences
pub async fn update_preferences(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<UpdatePreferencesInput>,
) -> Result<Json<Vec<NotificationPreference>>, StatusCode> {
    for pref in input.preferences {
        state
            .store
            .notifications()
            .update_preference(
                auth.user_id,
                clovalink_entity::repositories::PreferencePatch {
                    event_type: pref.event_type,
                    email_enabled: pref.email_enabled,
                    in_app_enabled: pref.in_app_enabled,
                },
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Return updated preferences
    get_preferences(State(state), Extension(auth)).await
}

/// Get notification preference labels for UI
/// GET /api/notifications/preference-labels
pub async fn get_preference_labels() -> Result<Json<Value>, StatusCode> {
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
    let mut settings = state
        .store
        .notifications()
        .tenant_settings(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // If no global settings exist, create defaults
    let has_global = settings.iter().any(|s| s.role.is_none());
    if !has_global {
        state
            .store
            .notifications()
            .ensure_global_settings(tenant_id, &event_types)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Re-fetch all settings
    settings = state
        .store
        .notifications()
        .tenant_settings(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let all_settings: Vec<TenantNotificationSetting> =
        settings.into_iter().map(tenant_setting).collect();

    // Group settings by role
    let global_settings: Vec<&TenantNotificationSetting> =
        all_settings.iter().filter(|s| s.role.is_none()).collect();

    let role_settings: std::collections::HashMap<String, Vec<&TenantNotificationSetting>> =
        all_settings.iter().filter(|s| s.role.is_some()).fold(
            std::collections::HashMap::new(),
            |mut acc, s| {
                if let Some(ref role) = s.role {
                    acc.entry(role.clone()).or_insert_with(Vec::new).push(s);
                }
                acc
            },
        );

    // If a specific role is requested, return just those settings with inheritance info
    if let Some(ref role) = query.role {
        let role_specific = role_settings.get(role).cloned().unwrap_or_default();

        // Build effective settings for this role (merging with global)
        let effective: Vec<Value> = event_types
            .iter()
            .map(|et| {
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
            })
            .collect();

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
        state
            .store
            .notifications()
            .update_tenant_setting(
                tenant_id,
                target_role.clone(),
                clovalink_entity::repositories::TenantNotificationPatch {
                    event_type: setting.event_type,
                    enabled: setting.enabled,
                    email_enforced: setting.email_enforced,
                    in_app_enforced: setting.in_app_enforced,
                    default_email: setting.default_email,
                    default_in_app: setting.default_in_app,
                },
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Return updated settings for the same role
    let query = TenantSettingsQuery { role: target_role };
    get_tenant_notification_settings(State(state), Extension(auth), Path(tenant_id), Query(query))
        .await
}

/// Get user preferences with company settings overlay
/// GET /api/notifications/preferences-with-company
/// Returns effective settings for the user based on their role
pub async fn get_preferences_with_company_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // Get user preferences
    let user_prefs: Vec<NotificationPreference> = state
        .store
        .notifications()
        .preferences(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(preference)
        .collect();

    // SuperAdmins are exempt from company-level notification controls
    if auth.role == "SuperAdmin" {
        return Ok(Json(json!({
            "preferences": user_prefs,
            "company_settings": {},
            "is_exempt": true
        })));
    }

    // Get all company settings (global + role-specific)
    let all_settings: Vec<TenantNotificationSetting> = state
        .store
        .notifications()
        .tenant_settings(auth.tenant_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(tenant_setting)
        .collect();

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
        let role_setting = all_settings
            .iter()
            .find(|s| s.event_type == event_type && s.role.as_deref() == Some(&auth.role));

        // Fall back to global setting
        let global_setting = all_settings
            .iter()
            .find(|s| s.event_type == event_type && s.role.is_none());

        let effective = role_setting.or(global_setting);

        if let Some(s) = effective {
            company_settings_map.insert(
                event_type.to_string(),
                json!({
                    "enabled": s.enabled,
                    "email_enforced": s.email_enforced,
                    "in_app_enforced": s.in_app_enforced,
                    "default_email": s.default_email,
                    "default_in_app": s.default_in_app,
                    "role_specific": role_setting.is_some()
                }),
            );
        }
    }

    Ok(Json(json!({
        "preferences": user_prefs,
        "company_settings": company_settings_map,
        "user_role": auth.role,
        "is_exempt": false
    })))
}
