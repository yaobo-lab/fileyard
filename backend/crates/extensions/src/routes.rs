//! Extension API route handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension as AxumExtension,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use clovalink_auth::AuthUser;

use crate::manifest::{fetch_manifest, parse_manifest};
use crate::models::{
    Extension, ExtensionVersion, ExtensionInstallation, ExtensionWebhookLog,
    RegisterExtensionInput, InstallExtensionInput, ValidateManifestInput,
    UpdateExtensionSettingsInput, UpdateExtensionAccessInput, CreateAutomationJobInput,
    UISidebarItem, UIButton, UIComponent,
};
use crate::permissions::{grant_permissions, get_installation_permissions, revoke_all_permissions};
use crate::scheduler::{create_automation_job, get_automation_jobs};
use crate::webhook::{generate_hmac_secret, generate_ed25519_keypair};

/// Shared state for extension routes
#[derive(Clone)]
pub struct ExtensionState {
    pub pool: PgPool,
    pub redis_url: String,
    pub webhook_timeout_ms: u64,
}

/// Register a new extension
/// POST /api/extensions/register
pub async fn register_extension(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Json(input): Json<RegisterExtensionInput>,
) -> Result<Json<Value>, StatusCode> {
    // Fetch and validate manifest
    let manifest = fetch_manifest(&input.manifest_url)
        .await
        .map_err(|e| {
            tracing::error!("Manifest fetch error: {:?}", e);
            StatusCode::BAD_REQUEST
        })?;

    // Generate signing key based on algorithm
    let (public_key, signature_algorithm) = match input.signature_algorithm.as_deref() {
        Some("ed25519") => {
            let (_private, public) = generate_ed25519_keypair();
            // Note: private key should be returned to the extension developer
            // In production, this would be handled more securely
            (public, "ed25519".to_string())
        }
        _ => {
            let secret = generate_hmac_secret();
            (secret, "hmac_sha256".to_string())
        }
    };

    // Insert extension with allowed_tenant_ids for cross-company access control
    let allowed_tenants = input.allowed_tenant_ids.as_ref().map(|ids| ids.as_slice());
    
    let extension = sqlx::query_as!(
        Extension,
        r#"
        INSERT INTO extensions (tenant_id, name, slug, description, extension_type, 
                               manifest_url, webhook_url, public_key, signature_algorithm, allowed_tenant_ids)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, tenant_id, name, slug, description, extension_type,
                  manifest_url, webhook_url, public_key, signature_algorithm,
                  status, allowed_tenant_ids, created_at, updated_at
        "#,
        auth.tenant_id,
        manifest.name,
        manifest.slug,
        manifest.description,
        manifest.extension_type,
        input.manifest_url,
        manifest.webhook,
        public_key,
        signature_algorithm,
        allowed_tenants
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert extension: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Insert version
    let manifest_json = serde_json::to_value(&manifest).unwrap_or(json!({}));
    
    sqlx::query!(
        r#"
        INSERT INTO extension_versions (extension_id, version, manifest, is_current)
        VALUES ($1, $2, $3, true)
        "#,
        extension.id,
        manifest.version,
        manifest_json
    )
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert extension version: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Insert event triggers if file_processor
    if manifest.extension_type == "file_processor" {
        let filter_config = if let Some(fp) = &manifest.file_processor {
            serde_json::json!({
                "file_types": fp.file_types,
                "max_file_size_mb": fp.max_file_size_mb
            })
        } else {
            serde_json::json!({})
        };

        sqlx::query!(
            r#"
            INSERT INTO extension_event_triggers (extension_id, event_type, filter_config)
            VALUES ($1, 'file_uploaded', $2)
            "#,
            extension.id,
            filter_config
        )
        .execute(&state.pool)
        .await
        .ok();
    }

    Ok(Json(json!({
        "extension": extension,
        "signing_key": public_key, // Return the key for developer to use
        "message": "Extension registered successfully"
    })))
}

/// Install an extension for the current tenant
/// POST /api/extensions/install/:extension_id
pub async fn install_extension(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Path(extension_id): Path<Uuid>,
    Json(input): Json<InstallExtensionInput>,
) -> Result<Json<Value>, StatusCode> {
    // Verify extension exists and tenant has access
    let extension = sqlx::query_as!(
        Extension,
        r#"
        SELECT id, tenant_id, name, slug, description, extension_type,
               manifest_url, webhook_url, public_key, signature_algorithm,
               status, allowed_tenant_ids, created_at, updated_at
        FROM extensions
        WHERE id = $1 AND status = 'active'
        "#,
        extension_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Check if tenant has access to install this extension
    let has_access = extension.tenant_id == auth.tenant_id  // Owner always has access
        || extension.allowed_tenant_ids
            .as_ref()
            .map(|ids| ids.contains(&auth.tenant_id))
            .unwrap_or(false);  // If allowed_tenant_ids is None, only owner has access

    if !has_access {
        tracing::warn!(
            "Tenant {} attempted to install extension {} without access",
            auth.tenant_id, extension_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Get current version
    let version = sqlx::query_as!(
        ExtensionVersion,
        r#"
        SELECT id, extension_id, version, manifest, changelog, is_current, created_at
        FROM extension_versions
        WHERE extension_id = $1 AND is_current = true
        "#,
        extension_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Check if already installed
    let existing = sqlx::query!(
        "SELECT id FROM extension_installations WHERE extension_id = $1 AND tenant_id = $2",
        extension_id,
        auth.tenant_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if existing.is_some() {
        return Err(StatusCode::CONFLICT);
    }

    // Create installation
    let installation = sqlx::query_as!(
        ExtensionInstallation,
        r#"
        INSERT INTO extension_installations (extension_id, tenant_id, version_id, settings, installed_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, extension_id, tenant_id, version_id, enabled, settings, installed_by, installed_at
        "#,
        extension_id,
        auth.tenant_id,
        version.id,
        input.settings.unwrap_or(json!({})),
        auth.user_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create installation: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Grant requested permissions
    grant_permissions(&state.pool, installation.id, &input.permissions)
        .await
        .map_err(|e| {
            tracing::error!("Failed to grant permissions: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // If automation extension, create default job if specified
    if extension.extension_type == "automation" {
        if let Some(manifest) = version.manifest.as_object() {
            if let Some(automation) = manifest.get("automation") {
                if let Some(default_cron) = automation.get("default_cron").and_then(|v| v.as_str()) {
                    let _ = create_automation_job(
                        &state.pool,
                        extension_id,
                        auth.tenant_id,
                        &format!("{} - Default Job", extension.name),
                        default_cron,
                        json!({}),
                    )
                    .await;
                }
            }
        }
    }

    Ok(Json(json!({
        "installation": installation,
        "message": "Extension installed successfully"
    })))
}

/// List all extensions accessible to current tenant
/// GET /api/extensions/list
/// Returns extensions that are:
/// - Owned by the tenant
/// - OR have the tenant in their allowed_tenant_ids
pub async fn list_extensions(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // List extensions owned by tenant OR accessible via allowed_tenant_ids
    let extensions = sqlx::query_as!(
        Extension,
        r#"
        SELECT id, tenant_id, name, slug, description, extension_type,
               manifest_url, webhook_url, public_key, signature_algorithm,
               status, allowed_tenant_ids, created_at, updated_at
        FROM extensions
        WHERE status = 'active' AND (
            tenant_id = $1 
            OR $1 = ANY(allowed_tenant_ids)
        )
        ORDER BY created_at DESC
        "#,
        auth.tenant_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get current versions for each extension
    let mut result = Vec::new();
    for ext in extensions {
        let version = sqlx::query!(
            "SELECT version, manifest FROM extension_versions WHERE extension_id = $1 AND is_current = true",
            ext.id
        )
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

        let is_owner = ext.tenant_id == auth.tenant_id;

        result.push(json!({
            "id": ext.id,
            "tenant_id": ext.tenant_id,
            "name": ext.name,
            "slug": ext.slug,
            "description": ext.description,
            "type": ext.extension_type,
            "status": ext.status,
            "is_owner": is_owner,
            "allowed_tenant_ids": ext.allowed_tenant_ids,
            "current_version": version.as_ref().map(|v| &v.version),
            "manifest": version.as_ref().map(|v| v.manifest.clone()),
            "created_at": ext.created_at
        }));
    }

    Ok(Json(json!(result)))
}

/// List installed extensions for current tenant
/// GET /api/extensions/installed
pub async fn list_installed_extensions(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    let installations = sqlx::query!(
        r#"
        SELECT ei.id as installation_id, ei.enabled, ei.settings, ei.installed_at,
               e.id as extension_id, e.name, e.slug, e.description, e.extension_type,
               e.status as extension_status,
               ev.version, ev.manifest
        FROM extension_installations ei
        JOIN extensions e ON ei.extension_id = e.id
        JOIN extension_versions ev ON ei.version_id = ev.id
        WHERE ei.tenant_id = $1
        ORDER BY ei.installed_at DESC
        "#,
        auth.tenant_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for inst in installations {
        let permissions = get_installation_permissions(&state.pool, inst.installation_id)
            .await
            .unwrap_or_default();

        result.push(json!({
            "installation_id": inst.installation_id,
            "extension_id": inst.extension_id,
            "name": inst.name,
            "slug": inst.slug,
            "description": inst.description,
            "type": inst.extension_type,
            "version": inst.version,
            "enabled": inst.enabled,
            "settings": inst.settings,
            "permissions": permissions,
            "installed_at": inst.installed_at
        }));
    }

    Ok(Json(json!(result)))
}

/// Validate a manifest
/// POST /api/extensions/validate-manifest
pub async fn validate_manifest(
    Json(input): Json<ValidateManifestInput>,
) -> Result<Json<Value>, StatusCode> {
    let manifest = if let Some(url) = input.manifest_url {
        fetch_manifest(&url).await.map_err(|e| {
            tracing::error!("Manifest validation error: {:?}", e);
            StatusCode::BAD_REQUEST
        })?
    } else if let Some(json) = input.manifest {
        parse_manifest(&json).map_err(|e| {
            tracing::error!("Manifest parse error: {:?}", e);
            StatusCode::BAD_REQUEST
        })?
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    Ok(Json(json!({
        "valid": true,
        "manifest": manifest
    })))
}

/// Get UI extensions for frontend injection
/// GET /api/extensions/ui
pub async fn get_ui_extensions(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    let installations = sqlx::query!(
        r#"
        SELECT e.id as extension_id, ev.manifest
        FROM extension_installations ei
        JOIN extensions e ON ei.extension_id = e.id
        JOIN extension_versions ev ON ei.version_id = ev.id
        WHERE ei.tenant_id = $1
          AND ei.enabled = true
          AND e.status = 'active'
          AND e.extension_type = 'ui'
        "#,
        auth.tenant_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut sidebar: Vec<UISidebarItem> = Vec::new();
    let mut buttons: Vec<UIButton> = Vec::new();
    let mut components: Vec<UIComponent> = Vec::new();

    for inst in installations {
        if let Some(manifest) = inst.manifest.as_object() {
            if let Some(ui) = manifest.get("ui") {
                let _load_mode = ui.get("load_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("iframe")
                    .to_string();

                // Parse sidebar items
                if let Some(items) = ui.get("sidebar").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Ok(mut parsed) = serde_json::from_value::<UISidebarItem>(item.clone()) {
                            parsed.extension_id = inst.extension_id;
                            sidebar.push(parsed);
                        }
                    }
                }

                // Parse buttons
                if let Some(items) = ui.get("buttons").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Ok(mut parsed) = serde_json::from_value::<UIButton>(item.clone()) {
                            parsed.extension_id = inst.extension_id;
                            buttons.push(parsed);
                        }
                    }
                }

                // Parse components
                if let Some(items) = ui.get("components").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Ok(mut parsed) = serde_json::from_value::<UIComponent>(item.clone()) {
                            parsed.extension_id = inst.extension_id;
                            components.push(parsed);
                        }
                    }
                }
            }
        }
    }

    // Sort sidebar by order
    sidebar.sort_by_key(|s| s.order);

    Ok(Json(json!({
        "sidebar": sidebar,
        "buttons": buttons,
        "components": components
    })))
}

/// Update extension settings
/// PUT /api/extensions/:id/settings
pub async fn update_extension_settings(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Path(extension_id): Path<Uuid>,
    Json(input): Json<UpdateExtensionSettingsInput>,
) -> Result<Json<Value>, StatusCode> {
    // Find installation
    let installation = sqlx::query!(
        "SELECT id, settings FROM extension_installations WHERE extension_id = $1 AND tenant_id = $2",
        extension_id,
        auth.tenant_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Update enabled status if provided
    if let Some(enabled) = input.enabled {
        sqlx::query!(
            "UPDATE extension_installations SET enabled = $1 WHERE id = $2",
            enabled,
            installation.id
        )
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Update settings if provided
    if let Some(settings) = input.settings {
        // Merge with existing settings
        let mut current = installation.settings.as_object().cloned().unwrap_or_default();
        if let Some(new_settings) = settings.as_object() {
            for (k, v) in new_settings {
                current.insert(k.clone(), v.clone());
            }
        }

        sqlx::query!(
            "UPDATE extension_installations SET settings = $1 WHERE id = $2",
            serde_json::Value::Object(current),
            installation.id
        )
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(json!({ "message": "Settings updated" })))
}

/// Update which companies can access an extension
/// PUT /api/extensions/:id/access
/// Only the extension owner can update access
pub async fn update_extension_access(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Path(extension_id): Path<Uuid>,
    Json(input): Json<UpdateExtensionAccessInput>,
) -> Result<Json<Value>, StatusCode> {
    // Verify extension exists and user is the owner
    let extension = sqlx::query!(
        "SELECT tenant_id FROM extensions WHERE id = $1",
        extension_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Only the owner tenant can update access
    if extension.tenant_id != auth.tenant_id {
        tracing::warn!(
            "Tenant {} attempted to update access for extension {} owned by {}",
            auth.tenant_id, extension_id, extension.tenant_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Update allowed_tenant_ids
    if let Some(allowed_ids) = input.allowed_tenant_ids {
        sqlx::query!(
            "UPDATE extensions SET allowed_tenant_ids = $1 WHERE id = $2",
            &allowed_ids[..],
            extension_id
        )
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update extension access: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        Ok(Json(json!({ 
            "message": "Extension access updated",
            "allowed_tenant_ids": allowed_ids
        })))
    } else {
        Ok(Json(json!({ "message": "No changes made" })))
    }
}

/// Uninstall an extension
/// DELETE /api/extensions/:id
pub async fn uninstall_extension(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Path(extension_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Find installation
    let installation = sqlx::query!(
        "SELECT id FROM extension_installations WHERE extension_id = $1 AND tenant_id = $2",
        extension_id,
        auth.tenant_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Revoke all permissions
    let _ = revoke_all_permissions(&state.pool, installation.id).await;

    // Delete automation jobs
    sqlx::query!(
        "DELETE FROM automation_jobs WHERE extension_id = $1 AND tenant_id = $2",
        extension_id,
        auth.tenant_id
    )
    .execute(&state.pool)
    .await
    .ok();

    // Delete installation (cascades to permissions)
    sqlx::query!(
        "DELETE FROM extension_installations WHERE id = $1",
        installation.id
    )
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "message": "Extension uninstalled" })))
}

/// Trigger automation manually
/// POST /api/extensions/trigger/automation/:job_id
pub async fn trigger_automation(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Verify job exists and belongs to tenant
    let job = sqlx::query!(
        r#"
        SELECT aj.id, aj.extension_id, e.name as extension_name
        FROM automation_jobs aj
        JOIN extensions e ON aj.extension_id = e.id
        WHERE aj.id = $1 AND aj.tenant_id = $2
        "#,
        job_id,
        auth.tenant_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Create scheduler and trigger job
    let scheduler = crate::scheduler::Scheduler::new(
        state.pool.clone(),
        &state.redis_url,
        state.webhook_timeout_ms,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create scheduler: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    scheduler.trigger_job(job_id).await.map_err(|e| {
        tracing::error!("Failed to trigger job: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "message": "Automation triggered",
        "job_id": job_id,
        "extension": job.extension_name
    })))
}

/// Create a new automation job
/// POST /api/extensions/:extension_id/jobs
pub async fn create_job(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Path(extension_id): Path<Uuid>,
    Json(input): Json<CreateAutomationJobInput>,
) -> Result<Json<Value>, StatusCode> {
    // Verify extension is installed and is automation type
    let installation = sqlx::query!(
        r#"
        SELECT ei.id, e.extension_type
        FROM extension_installations ei
        JOIN extensions e ON ei.extension_id = e.id
        WHERE e.id = $1 AND ei.tenant_id = $2 AND ei.enabled = true
        "#,
        extension_id,
        auth.tenant_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    if installation.extension_type != "automation" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create job
    let job = create_automation_job(
        &state.pool,
        extension_id,
        auth.tenant_id,
        &input.name,
        &input.cron_expression,
        input.config.unwrap_or(json!({})),
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create job: {:?}", e);
        StatusCode::BAD_REQUEST
    })?;

    Ok(Json(json!({
        "job": job,
        "message": "Automation job created"
    })))
}

/// List automation jobs for an extension
/// GET /api/extensions/:extension_id/jobs
pub async fn list_jobs(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Path(extension_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let jobs = get_automation_jobs(&state.pool, extension_id, auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!(jobs)))
}

/// Get extension webhook logs
/// GET /api/extensions/:extension_id/logs
pub async fn get_webhook_logs(
    State(state): State<Arc<ExtensionState>>,
    AxumExtension(auth): AxumExtension<AuthUser>,
    Path(extension_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let logs = sqlx::query_as!(
        ExtensionWebhookLog,
        r#"
        SELECT id, extension_id, tenant_id, event_type, payload, request_headers,
               response_status, response_body, duration_ms, error_message, created_at
        FROM extension_webhook_logs
        WHERE extension_id = $1 AND tenant_id = $2
        ORDER BY created_at DESC
        LIMIT 100
        "#,
        extension_id,
        auth.tenant_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!(logs)))
}

