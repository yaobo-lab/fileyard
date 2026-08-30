use axum::{
    extract::{State, Multipart},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::AppState;
use clovalink_auth::AuthUser;
use clovalink_core::cache::{keys as cache_keys, ttl as cache_ttl};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalSetting {
    pub key: String,
    pub value: Value,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGlobalSettingsInput {
    pub settings: Vec<SettingUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct SettingUpdate {
    pub key: String,
    pub value: Value,
}

/// Cached global settings
#[derive(Serialize, Deserialize, Clone)]
struct GlobalSettingsCache {
    data: Value,
}

/// Get all global settings
/// GET /api/global-settings
/// Public read (any authenticated user can read global settings)
pub async fn get_global_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    // Check cache first
    let cache_key = cache_keys::global_settings();
    if let Some(ref cache) = state.cache {
        if let Ok(cached) = cache.get::<GlobalSettingsCache>(&cache_key).await {
            return Ok(Json(cached.data));
        }
    }
    
    let settings: Vec<(String, Value, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        "SELECT key, value, updated_at FROM global_settings ORDER BY key"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch global settings: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert to a more usable format
    let mut result = serde_json::Map::new();
    for (key, value, _) in settings {
        result.insert(key, value);
    }

    let response = json!(result);
    
    // Cache for 10 minutes
    if let Some(ref cache) = state.cache {
        let cache_data = GlobalSettingsCache { data: response.clone() };
        let _ = cache.set(&cache_key, &cache_data, cache_ttl::GLOBAL_SETTINGS).await;
    }

    Ok(Json(response))
}

/// Update global settings
/// PUT /api/global-settings
/// Requires SuperAdmin role
pub async fn update_global_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<UpdateGlobalSettingsInput>,
) -> Result<Json<Value>, StatusCode> {
    // Only SuperAdmin can update global settings
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Valid setting keys
    let valid_keys = [
        "date_format",
        "time_format", 
        "timezone",
        "footer_attribution",
        "footer_disclaimer",
        "app_name",
        "logo_url",
        // Page content
        "tos_content",
        "privacy_content",
        "help_content",
        // System
        "maintenance_mode",
        "maintenance_message",
        // Version & Updates
        "github_repo",
    ];

    // Update each setting
    for setting in &input.settings {
        if !valid_keys.contains(&setting.key.as_str()) {
            tracing::warn!("Attempted to update invalid global setting key: {}", setting.key);
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO global_settings (key, value, updated_at, updated_by)
            VALUES ($1, $2, NOW(), $3)
            ON CONFLICT (key) DO UPDATE 
            SET value = $2, updated_at = NOW(), updated_by = $3
            "#
        )
        .bind(&setting.key)
        .bind(&setting.value)
        .bind(auth.user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update global setting '{}': {:?}", setting.key, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    // Create audit log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, metadata, ip_address)
        VALUES ($1, $2, 'update_global_settings', 'global_settings', $3, $4::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(json!({
        "updated_keys": input.settings.iter().map(|s| &s.key).collect::<Vec<_>>(),
    }))
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await
    .ok(); // Don't fail if audit log fails

    // Invalidate global settings cache
    if let Some(ref cache) = state.cache {
        let cache_key = cache_keys::global_settings();
        let _ = cache.delete(&cache_key).await;
    }

    // Return updated settings
    get_global_settings(State(state)).await
}

/// Upload logo
/// POST /api/global-settings/logo
/// Requires SuperAdmin role
pub async fn upload_logo(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    mut multipart: Multipart,
) -> Result<Json<Value>, StatusCode> {
    // Only SuperAdmin can upload logo
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to get multipart field: {:?}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "logo" || name == "file" {
            let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
            
            // Validate it's an image (SVG, PNG, or other image types)
            let is_valid = content_type.starts_with("image/") || content_type == "image/svg+xml";
            if !is_valid {
                tracing::warn!("Invalid logo content type: {}", content_type);
                return Err(StatusCode::BAD_REQUEST);
            }

            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            
            // Limit size to 2MB
            if data.len() > 2 * 1024 * 1024 {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }

            // Generate filename
            let extension = match content_type.as_str() {
                "image/svg+xml" => "svg",
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => "png",
            };
            let filename = format!("branding/logo.{}", extension);

            // Upload to storage
            state.storage.upload(&filename, data.to_vec()).await
                .map_err(|e| {
                    tracing::error!("Failed to upload logo: {:?}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Get the public URL
            let logo_url = format!("/uploads/{}", filename);

            // Update global settings
            sqlx::query(
                r#"
                INSERT INTO global_settings (key, value, updated_at, updated_by)
                VALUES ('logo_url', $1, NOW(), $2)
                ON CONFLICT (key) DO UPDATE 
                SET value = $1, updated_at = NOW(), updated_by = $2
                "#
            )
            .bind(json!(logo_url))
            .bind(auth.user_id)
            .execute(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to save logo URL: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // Create audit log
            sqlx::query(
                r#"
                INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, metadata, ip_address)
                VALUES ($1, $2, 'upload_logo', 'global_settings', $3, $4::inet)
                "#
            )
            .bind(auth.tenant_id)
            .bind(auth.user_id)
            .bind(json!({ "logo_url": logo_url }))
            .bind(&auth.ip_address)
            .execute(&state.pool)
            .await
            .ok();

            return Ok(Json(json!({
                "success": true,
                "logo_url": logo_url
            })));
        }
    }

    Err(StatusCode::BAD_REQUEST)
}

/// Delete logo (revert to default)
/// DELETE /api/global-settings/logo
/// Requires SuperAdmin role
pub async fn delete_logo(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // Only SuperAdmin can delete logo
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Remove logo_url from settings
    sqlx::query("DELETE FROM global_settings WHERE key = 'logo_url'")
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete logo setting: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create audit log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, metadata, ip_address)
        VALUES ($1, $2, 'delete_logo', 'global_settings', '{}', $3::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await
    .ok();

    Ok(Json(json!({
        "success": true,
        "message": "Logo removed, using default"
    })))
}

/// Upload favicon
/// POST /api/global-settings/favicon
/// Requires SuperAdmin role
pub async fn upload_favicon(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    mut multipart: Multipart,
) -> Result<Json<Value>, StatusCode> {
    // Only SuperAdmin can upload favicon
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to get multipart field: {:?}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "favicon" || name == "file" {
            let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
            
            // Validate it's an image or ICO file
            let is_valid = content_type.starts_with("image/") 
                || content_type == "image/svg+xml"
                || content_type == "image/x-icon"
                || content_type == "image/vnd.microsoft.icon";
            if !is_valid {
                tracing::warn!("Invalid favicon content type: {}", content_type);
                return Err(StatusCode::BAD_REQUEST);
            }

            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            
            // Limit size to 1MB for favicon
            if data.len() > 1024 * 1024 {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }

            // Generate filename
            let extension = match content_type.as_str() {
                "image/svg+xml" => "svg",
                "image/png" => "png",
                "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => "png",
            };
            let filename = format!("branding/favicon.{}", extension);

            // Upload to storage
            state.storage.upload(&filename, data.to_vec()).await
                .map_err(|e| {
                    tracing::error!("Failed to upload favicon: {:?}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Get the public URL
            let favicon_url = format!("/uploads/{}", filename);

            // Update global settings
            sqlx::query(
                r#"
                INSERT INTO global_settings (key, value, updated_at, updated_by)
                VALUES ('favicon_url', $1, NOW(), $2)
                ON CONFLICT (key) DO UPDATE 
                SET value = $1, updated_at = NOW(), updated_by = $2
                "#
            )
            .bind(json!(favicon_url))
            .bind(auth.user_id)
            .execute(&state.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to save favicon URL: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // Create audit log
            sqlx::query(
                r#"
                INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, metadata, ip_address)
                VALUES ($1, $2, 'upload_favicon', 'global_settings', $3, $4::inet)
                "#
            )
            .bind(auth.tenant_id)
            .bind(auth.user_id)
            .bind(json!({ "favicon_url": favicon_url }))
            .bind(&auth.ip_address)
            .execute(&state.pool)
            .await
            .ok();

            return Ok(Json(json!({
                "success": true,
                "favicon_url": favicon_url
            })));
        }
    }

    Err(StatusCode::BAD_REQUEST)
}

/// Delete favicon (revert to default)
/// DELETE /api/global-settings/favicon
/// Requires SuperAdmin role
pub async fn delete_favicon(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // Only SuperAdmin can delete favicon
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Remove favicon_url from settings
    sqlx::query("DELETE FROM global_settings WHERE key = 'favicon_url'")
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete favicon setting: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create audit log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, metadata, ip_address)
        VALUES ($1, $2, 'delete_favicon', 'global_settings', '{}', $3::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await
    .ok();

    Ok(Json(json!({
        "success": true,
        "message": "Favicon removed, using default"
    })))
}

