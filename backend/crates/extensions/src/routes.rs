//! Extension API route handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension as AxumExtension,
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use clovalink_auth::AuthUser;

use crate::manifest::{fetch_manifest, parse_manifest};
use crate::models::{
    CreateAutomationJobInput, InstallExtensionInput, RegisterExtensionInput, UIButton, UIComponent,
    UISidebarItem, UpdateExtensionAccessInput, UpdateExtensionSettingsInput, ValidateManifestInput,
};
use crate::permissions::{get_installation_permissions, grant_permissions};
use crate::scheduler::{create_automation_job, get_automation_jobs};
use crate::webhook::{generate_ed25519_keypair, generate_hmac_secret};

/// Shared state for extension routes
#[derive(Clone)]
pub struct ExtensionState {
    pub store: clovalink_entity::DataStore,
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
    let manifest = fetch_manifest(&input.manifest_url).await.map_err(|e| {
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

    let manifest_json = serde_json::to_value(&manifest).unwrap_or(json!({}));
    let trigger_filter = if manifest.extension_type == "file_processor" {
        Some(if let Some(fp) = &manifest.file_processor {
            serde_json::json!({
                "file_types": fp.file_types,
                "max_file_size_mb": fp.max_file_size_mb
            })
        } else {
            serde_json::json!({})
        })
    } else {
        None
    };
    let extension = state
        .store
        .extension_runtime()
        .register(clovalink_entity::repositories::NewExtension {
            tenant_id: auth.tenant_id,
            name: manifest.name.clone(),
            slug: manifest.slug.clone(),
            description: manifest.description.clone(),
            extension_type: manifest.extension_type.clone(),
            manifest_url: input.manifest_url,
            webhook_url: manifest.webhook.clone(),
            public_key: public_key.clone(),
            signature_algorithm,
            allowed_tenant_ids: input.allowed_tenant_ids,
            version: manifest.version.clone(),
            manifest: manifest_json,
            trigger_filter,
        })
        .await
        .map_err(|e| {
            tracing::error!("Failed to register extension: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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
    let repository = state.store.extension_runtime();
    let extension = repository
        .extension(extension_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .filter(|extension| extension.status == "active")
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if tenant has access to install this extension
    let has_access = extension.tenant_id == auth.tenant_id  // Owner always has access
        || extension.allowed_tenant_ids
            .as_ref()
            .map(|ids| ids.contains(&auth.tenant_id))
            .unwrap_or(false); // If allowed_tenant_ids is None, only owner has access

    if !has_access {
        tracing::warn!(
            "Tenant {} attempted to install extension {} without access",
            auth.tenant_id,
            extension_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Get current version
    let version = repository
        .current_version(extension_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Create installation
    let installation = repository
        .create_installation(
            extension_id,
            auth.tenant_id,
            version.id,
            input.settings.unwrap_or(json!({})),
            auth.user_id,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create installation: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::CONFLICT)?;

    // Grant requested permissions
    grant_permissions(&state.store, installation.id, &input.permissions)
        .await
        .map_err(|e| {
            tracing::error!("Failed to grant permissions: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // If automation extension, create default job if specified
    if extension.extension_type == "automation" {
        if let Some(manifest) = version.manifest.as_object() {
            if let Some(automation) = manifest.get("automation") {
                if let Some(default_cron) = automation.get("default_cron").and_then(|v| v.as_str())
                {
                    let _ = create_automation_job(
                        &state.store,
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
    let extensions = state
        .store
        .extension_runtime()
        .accessible_extensions(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get current versions for each extension
    let mut result = Vec::new();
    for (ext, version) in extensions {
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
    let installations = state
        .store
        .extension_runtime()
        .installed_extensions(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    for inst in installations {
        let permissions = get_installation_permissions(&state.store, inst.installation.id)
            .await
            .unwrap_or_default();

        result.push(json!({
            "installation_id": inst.installation.id,
            "extension_id": inst.extension.id,
            "name": inst.extension.name,
            "slug": inst.extension.slug,
            "description": inst.extension.description,
            "type": inst.extension.extension_type,
            "version": inst.version.version,
            "enabled": inst.installation.enabled,
            "settings": inst.installation.settings,
            "permissions": permissions,
            "installed_at": inst.installation.installed_at
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
    let installations = state
        .store
        .extension_runtime()
        .active_ui_manifests(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut sidebar: Vec<UISidebarItem> = Vec::new();
    let mut buttons: Vec<UIButton> = Vec::new();
    let mut components: Vec<UIComponent> = Vec::new();

    for (extension_id, manifest_value) in installations {
        if let Some(manifest) = manifest_value.as_object() {
            if let Some(ui) = manifest.get("ui") {
                let _load_mode = ui
                    .get("load_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("iframe")
                    .to_string();

                // Parse sidebar items
                if let Some(items) = ui.get("sidebar").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Ok(mut parsed) =
                            serde_json::from_value::<UISidebarItem>(item.clone())
                        {
                            parsed.extension_id = extension_id;
                            sidebar.push(parsed);
                        }
                    }
                }

                // Parse buttons
                if let Some(items) = ui.get("buttons").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Ok(mut parsed) = serde_json::from_value::<UIButton>(item.clone()) {
                            parsed.extension_id = extension_id;
                            buttons.push(parsed);
                        }
                    }
                }

                // Parse components
                if let Some(items) = ui.get("components").and_then(|v| v.as_array()) {
                    for item in items {
                        if let Ok(mut parsed) = serde_json::from_value::<UIComponent>(item.clone())
                        {
                            parsed.extension_id = extension_id;
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
    let updated = state
        .store
        .extension_runtime()
        .update_installation(extension_id, auth.tenant_id, input.enabled, input.settings)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !updated {
        return Err(StatusCode::NOT_FOUND);
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
    if let Some(allowed_ids) = input.allowed_tenant_ids {
        match state
            .store
            .extension_runtime()
            .update_access(extension_id, auth.tenant_id, allowed_ids.clone())
            .await
            .map_err(|e| {
                tracing::error!("Failed to update extension access: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })? {
            None => return Err(StatusCode::NOT_FOUND),
            Some(false) => return Err(StatusCode::FORBIDDEN),
            Some(true) => {}
        }

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
    let deleted = state
        .store
        .extension_runtime()
        .uninstall(extension_id, auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

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
    let extension_name = state
        .store
        .extension_runtime()
        .tenant_job_extension_name(job_id, auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Create scheduler and trigger job
    let scheduler = crate::scheduler::Scheduler::new(
        state.store.clone(),
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
        "extension": extension_name
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
    if !state
        .store
        .extension_runtime()
        .is_enabled_automation(extension_id, auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create job
    let job = create_automation_job(
        &state.store,
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
    let jobs = get_automation_jobs(&state.store, extension_id, auth.tenant_id)
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
    let logs = state
        .store
        .extension_runtime()
        .webhook_logs(extension_id, auth.tenant_id, 100)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!(logs)))
}
