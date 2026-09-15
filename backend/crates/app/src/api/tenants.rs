use crate::compliance::{can_modify_setting, get_tenant_compliance_mode, ComplianceRestrictions};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    Extension,
};
use crate::auth::{generate_token, require_super_admin, AuthUser};
use app_core::models::{CreateTenantInput, UpdateTenantInput};
use app_entity::{
    entities::tenants,
    repositories::{ListTenantsFilter, NewAuditLog},
};
use sea_orm::ActiveValue::Set;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct TenantFilters {
    pub status: Option<String>,
    pub plan: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List all tenants/companies
/// GET /api/tenants
/// SuperAdmin only - for managing companies
pub async fn list_tenants(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(filters): Query<TenantFilters>,
) -> Result<Json<Value>, StatusCode> {
    // SECURITY: Only SuperAdmin can list/manage all companies
    require_super_admin(&auth)?;

    let limit = filters.limit.unwrap_or(50).min(100) as u64;
    let offset = filters.offset.unwrap_or(0).max(0) as u64;

    let tenants = state
        .store
        .tenants()
        .list_filtered(ListTenantsFilter {
            status: filters.status,
            plan: filters.plan,
            search: filters.search,
            limit,
            offset,
        })
        .await
        .map_err(|e| {
            tracing::error!("Failed to list tenants: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut results = Vec::new();
    for tenant in tenants {
        let user_count = state
            .store
            .tenants()
            .count_users(tenant.id)
            .await
            .unwrap_or(0);

        // Calculate actual storage from files_metadata
        let actual_storage = state
            .store
            .files()
            .calculate_storage_used(tenant.id)
            .await
            .unwrap_or(0);

        let restrictions = ComplianceRestrictions::for_mode(&tenant.compliance_mode);

        results.push(json!({
            "id": tenant.id,
            "name": tenant.name,
            "domain": tenant.domain,
            "plan": tenant.plan,
            "status": tenant.status,
            "compliance_mode": tenant.compliance_mode,
            "retention_policy_days": tenant.retention_policy_days,
            "storage_used_bytes": actual_storage,
            "storage_quota_bytes": tenant.storage_quota_bytes,
            "max_upload_size_bytes": tenant.max_upload_size_bytes,
            "mfa_required": tenant.mfa_required,
            "session_timeout_minutes": tenant.session_timeout_minutes,
            "public_sharing_enabled": tenant.public_sharing_enabled,
            "data_export_enabled": tenant.data_export_enabled.unwrap_or(true),
            "approval_workflow_enabled": tenant.approval_workflow_enabled.unwrap_or(false),
            "backup_enabled": tenant.backup_enabled.unwrap_or(true),
            "auto_backup_enabled": tenant.auto_backup_enabled.unwrap_or(false),
            "auto_backup_cron": tenant.auto_backup_cron.as_deref().unwrap_or("0 2 * * 0"),
            "auto_backup_retention_count": tenant.auto_backup_retention_count.unwrap_or(5),
            "enable_totp": tenant.enable_totp,
            "auth_methods": tenant.auth_methods,
            "user_count": user_count,
            "created_at": tenant.created_at,
            "restrictions": restrictions,
            "smtp_host": tenant.smtp_host,
            "smtp_port": tenant.smtp_port,
            "smtp_username": tenant.smtp_username,
            "smtp_password": tenant.smtp_password,
            "smtp_from": tenant.smtp_from,
            "smtp_secure": tenant.smtp_secure,
        }));
    }

    Ok(Json(json!(results)))
}

/// List tenants accessible to the current user
/// GET /api/tenants/accessible
/// Returns the user's primary tenant plus any tenants from allowed_tenant_ids
/// SuperAdmins get ALL active tenants (god-mode access)
/// Available to all authenticated users
pub async fn accessible_tenants(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // Get the user
    let user = state
        .store
        .users()
        .user(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user for accessible tenants: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let primary_tenant_id = user.tenant_id;

    // SuperAdmins get ALL active tenants
    let tenants = if auth.role == "SuperAdmin" {
        state
            .store
            .tenants()
            .list_active()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch all tenants for SuperAdmin: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        let mut tenant_ids = vec![primary_tenant_id];
        if let Some(ref ids) = user.allowed_tenant_ids {
            for id in ids {
                if !tenant_ids.contains(id) {
                    tenant_ids.push(*id);
                }
            }
        }

        state
            .store
            .tenants()
            .list_by_ids(&tenant_ids, Some("active"))
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch accessible tenants: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    let mut results = Vec::new();
    for tenant in tenants {
        let restrictions = ComplianceRestrictions::for_mode(&tenant.compliance_mode);
        results.push(json!({
            "id": tenant.id,
            "name": tenant.name,
            "domain": tenant.domain,
            "plan": tenant.plan,
            "status": tenant.status,
            "compliance_mode": tenant.compliance_mode,
            "retention_policy_days": tenant.retention_policy_days,
            "data_export_enabled": tenant.data_export_enabled.unwrap_or(true),
            "approval_workflow_enabled": tenant.approval_workflow_enabled.unwrap_or(false),
            "backup_enabled": tenant.backup_enabled.unwrap_or(true),
            "auto_backup_enabled": tenant.auto_backup_enabled.unwrap_or(false),
            "auto_backup_cron": tenant.auto_backup_cron.as_deref().unwrap_or("0 2 * * 0"),
            "auto_backup_retention_count": tenant.auto_backup_retention_count.unwrap_or(5),
            "enable_totp": tenant.enable_totp,
            "auth_methods": tenant.auth_methods,
            "is_primary": tenant.id == primary_tenant_id,
            "restrictions": restrictions,
            "storage_used_bytes": 0,
            "storage_quota_bytes": tenant.storage_quota_bytes,
            "max_upload_size_bytes": tenant.max_upload_size_bytes,
            "smtp_host": tenant.smtp_host,
            "smtp_port": tenant.smtp_port,
            "smtp_username": tenant.smtp_username,
            "smtp_password": tenant.smtp_password,
            "smtp_from": tenant.smtp_from,
            "smtp_secure": tenant.smtp_secure,
        }));
    }

    Ok(Json(json!(results)))
}

/// Create new tenant
/// POST /api/tenants
/// Only SuperAdmin can create tenants
pub async fn create_tenant(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<CreateTenantInput>,
) -> Result<Json<Value>, StatusCode> {
    // Only SuperAdmin can create tenants
    require_super_admin(&auth)?;

    let tenant = state
        .store
        .tenants()
        .create_tenant(
            input.name,
            input.domain,
            input.plan,
            input.storage_quota_bytes,
            input.departments,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create tenant: {:?}", e);
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(json!({
        "id": tenant.id,
        "name": tenant.name,
        "domain": tenant.domain,
        "plan": tenant.plan,
        "status": tenant.status,
        "storage_quota_bytes": tenant.storage_quota_bytes,
        "created_at": tenant.created_at,
    })))
}

/// Update tenant
/// PUT /api/tenants/:id
/// Only SuperAdmin can update tenants
pub async fn update_tenant(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTenantInput>,
) -> Result<Json<Value>, StatusCode> {
    // Only SuperAdmin can update tenants
    require_super_admin(&auth)?;

    let tenant = state
        .store
        .tenants()
        .by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: tenants::ActiveModel = tenant.into();
    let mut has_updates = false;

    if let Some(name) = input.name {
        active.name = Set(name);
        has_updates = true;
    }
    if let Some(domain) = input.domain {
        active.domain = Set(domain);
        has_updates = true;
    }
    if let Some(plan) = input.plan {
        active.plan = Set(plan);
        has_updates = true;
    }
    if let Some(status) = input.status {
        active.status = Set(status);
        has_updates = true;
    }
    if let Some(compliance_mode) = input.compliance_mode {
        active.compliance_mode = Set(compliance_mode);
        has_updates = true;
    }
    if let Some(storage_quota_bytes) = input.storage_quota_bytes {
        active.storage_quota_bytes = Set(Some(storage_quota_bytes));
        has_updates = true;
    }
    if let Some(retention_policy_days) = input.retention_policy_days {
        active.retention_policy_days = Set(retention_policy_days);
        has_updates = true;
    }
    if let Some(max_upload_size_bytes) = input.max_upload_size_bytes {
        active.max_upload_size_bytes = Set(Some(max_upload_size_bytes));
        has_updates = true;
    }
    if let Some(smtp_host) = input.smtp_host {
        active.smtp_host = Set(Some(smtp_host));
        has_updates = true;
    }
    if let Some(smtp_port) = input.smtp_port {
        active.smtp_port = Set(Some(smtp_port));
        has_updates = true;
    }
    if let Some(smtp_username) = input.smtp_username {
        active.smtp_username = Set(Some(smtp_username));
        has_updates = true;
    }
    if let Some(smtp_password) = input.smtp_password {
        active.smtp_password = Set(Some(smtp_password));
        has_updates = true;
    }
    if let Some(smtp_from) = input.smtp_from {
        active.smtp_from = Set(Some(smtp_from));
        has_updates = true;
    }
    if let Some(smtp_secure) = input.smtp_secure {
        active.smtp_secure = Set(Some(smtp_secure));
        has_updates = true;
    }
    if let Some(enable_totp) = input.enable_totp {
        active.enable_totp = Set(Some(enable_totp));
        has_updates = true;
    }
    if let Some(enable_passkeys) = input.enable_passkeys {
        active.enable_passkeys = Set(Some(enable_passkeys));
        has_updates = true;
    }
    if let Some(data_export_enabled) = input.data_export_enabled {
        active.data_export_enabled = Set(Some(data_export_enabled));
        has_updates = true;
    }
    if let Some(approval_workflow_enabled) = input.approval_workflow_enabled {
        active.approval_workflow_enabled = Set(Some(approval_workflow_enabled));
        has_updates = true;
    }
    if let Some(backup_enabled) = input.backup_enabled {
        active.backup_enabled = Set(Some(backup_enabled));
        has_updates = true;
    }
    if let Some(auto_backup_enabled) = input.auto_backup_enabled {
        if auto_backup_enabled && !crate::settings_backup::is_master_key_configured() {
            return Err(StatusCode::BAD_REQUEST);
        }
        active.auto_backup_enabled = Set(Some(auto_backup_enabled));
        has_updates = true;
    }
    if let Some(auto_backup_cron) = input.auto_backup_cron {
        active.auto_backup_cron = Set(Some(auto_backup_cron));
        has_updates = true;
    }
    if let Some(auto_backup_retention_count) = input.auto_backup_retention_count {
        active.auto_backup_retention_count = Set(Some(auto_backup_retention_count));
        has_updates = true;
    }

    if !has_updates {
        return Err(StatusCode::BAD_REQUEST);
    }

    active.updated_at = Set(chrono::Utc::now().into());

    let tenant = state
        .store
        .tenants()
        .update(active)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update tenant: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Invalidate ALL caches after tenant update
    if let Some(ref cache) = state.cache {
        use app_core::cache::keys;
        let compliance_key = keys::compliance(id);
        let tenant_key = keys::tenant(id);
        let _ = cache.delete(&compliance_key).await;
        let _ = cache.delete(&tenant_key).await;
        let _ = cache.delete_pattern("clovalink:user:*").await;
        tracing::info!("Invalidated all caches for tenant {} after update", id);
    }

    Ok(Json(json!({
        "id": tenant.id,
        "name": tenant.name,
        "domain": tenant.domain,
        "plan": tenant.plan,
        "status": tenant.status,
        "compliance_mode": tenant.compliance_mode,
        "storage_quota_bytes": tenant.storage_quota_bytes,
        "retention_policy_days": tenant.retention_policy_days,
        "data_export_enabled": tenant.data_export_enabled.unwrap_or(true),
        "updated_at": tenant.updated_at,
    })))
}

/// Edit my company (for company owners/admins)
/// PUT /api/tenants/:id/edit
/// Owners and Admins can edit their own company
pub async fn edit_my_company(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<UpdateTenantInput>,
) -> Result<Json<Value>, StatusCode> {
    // Debug logging
    tracing::info!(
        "edit_my_company called: user_id={}, user_tenant_id={}, user_role={}, requested_tenant_id={}", 
        auth.user_id, auth.tenant_id, auth.role, id
    );

    // Check if user is Owner or Admin of this tenant
    if auth.tenant_id != id {
        tracing::warn!(
            "Forbidden: tenant_id mismatch. user_tenant={}, requested={}",
            auth.tenant_id,
            id
        );
        return Err(StatusCode::FORBIDDEN);
    }
    if auth.role != "Owner" && auth.role != "Admin" && auth.role != "SuperAdmin" {
        tracing::warn!("Forbidden: insufficient role. user_role={}", auth.role);
        return Err(StatusCode::FORBIDDEN);
    }

    tracing::info!(
        "Authorization passed for user {} (role={}) to edit tenant {}",
        auth.user_id,
        auth.role,
        id
    );

    // Get compliance mode to check restrictions
    let compliance_mode = get_tenant_compliance_mode(&state.store, id)
        .await
        .unwrap_or_else(|_| "Standard".to_string());
    let restrictions = ComplianceRestrictions::for_mode(&compliance_mode);

    // Check if trying to disable MFA when compliance requires it
    if let Some(enable_totp) = input.enable_totp {
        if !enable_totp && restrictions.mfa_locked {
            tracing::warn!("Cannot disable MFA in {} mode", compliance_mode);
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Check retention policy against minimum
    if let Some(retention_days) = input.retention_policy_days {
        if let Some(min_days) = restrictions.min_retention_days {
            if retention_days < min_days {
                tracing::warn!(
                    "Retention days {} below minimum {} for {} mode",
                    retention_days,
                    min_days,
                    compliance_mode
                );
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    // Handle auto_backup fields (SuperAdmin only, requires password confirmation)
    let has_backup_changes = input.auto_backup_enabled.is_some()
        || input.auto_backup_cron.is_some()
        || input.auto_backup_retention_count.is_some();
    if auth.role == "SuperAdmin" && has_backup_changes {
        crate::settings_backup::verify_password_confirmation(
            &state.store,
            auth.user_id,
            &headers,
        )
        .await?;
    }

    let tenant = state
        .store
        .tenants()
        .by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: tenants::ActiveModel = tenant.into();
    let mut has_updates = false;

    if let Some(retention_policy_days) = input.retention_policy_days {
        if can_modify_setting(&compliance_mode, "retention_policy_days")
            || auth.role == "SuperAdmin"
        {
            active.retention_policy_days = Set(retention_policy_days);
            has_updates = true;
        }
    }
    if let Some(smtp_host) = input.smtp_host {
        active.smtp_host = Set(Some(smtp_host));
        has_updates = true;
    }
    if let Some(smtp_port) = input.smtp_port {
        active.smtp_port = Set(Some(smtp_port));
        has_updates = true;
    }
    if let Some(smtp_username) = input.smtp_username {
        active.smtp_username = Set(Some(smtp_username));
        has_updates = true;
    }
    if let Some(smtp_password) = input.smtp_password {
        active.smtp_password = Set(Some(smtp_password));
        has_updates = true;
    }
    if let Some(smtp_from) = input.smtp_from {
        active.smtp_from = Set(Some(smtp_from));
        has_updates = true;
    }
    if let Some(smtp_secure) = input.smtp_secure {
        active.smtp_secure = Set(Some(smtp_secure));
        has_updates = true;
    }
    if let Some(enable_totp) = input.enable_totp {
        if can_modify_setting(&compliance_mode, "enable_totp") || enable_totp {
            active.enable_totp = Set(Some(enable_totp));
            has_updates = true;
        }
    }
    if let Some(mfa_required) = input.mfa_required {
        if can_modify_setting(&compliance_mode, "mfa_required") || auth.role == "SuperAdmin" {
            active.mfa_required = Set(Some(mfa_required));
            has_updates = true;
        }
    }
    if let Some(session_timeout) = input.session_timeout_minutes {
        if can_modify_setting(&compliance_mode, "session_timeout_minutes")
            || auth.role == "SuperAdmin"
        {
            active.session_timeout_minutes = Set(Some(session_timeout));
            has_updates = true;
        }
    }
    if let Some(public_sharing) = input.public_sharing_enabled {
        if can_modify_setting(&compliance_mode, "public_sharing_enabled")
            || auth.role == "SuperAdmin"
        {
            active.public_sharing_enabled = Set(Some(public_sharing));
            has_updates = true;
        }
    }
    if let Some(storage_quota_bytes) = input.storage_quota_bytes {
        active.storage_quota_bytes = Set(Some(storage_quota_bytes));
        has_updates = true;
    }
    if let Some(max_upload_size_bytes) = input.max_upload_size_bytes {
        active.max_upload_size_bytes = Set(Some(max_upload_size_bytes));
        has_updates = true;
    }
    if let Some(approval_workflow_enabled) = input.approval_workflow_enabled {
        active.approval_workflow_enabled = Set(Some(approval_workflow_enabled));
        has_updates = true;
    }
    if let Some(backup_enabled) = input.backup_enabled {
        active.backup_enabled = Set(Some(backup_enabled));
        has_updates = true;
    }
    if auth.role == "SuperAdmin" {
        if let Some(auto_backup_enabled) = input.auto_backup_enabled {
            if auto_backup_enabled && !crate::settings_backup::is_master_key_configured() {
                return Err(StatusCode::BAD_REQUEST);
            }
            active.auto_backup_enabled = Set(Some(auto_backup_enabled));
            has_updates = true;
        }
        if let Some(auto_backup_cron) = input.auto_backup_cron {
            active.auto_backup_cron = Set(Some(auto_backup_cron));
            has_updates = true;
        }
        if let Some(auto_backup_retention_count) = input.auto_backup_retention_count {
            active.auto_backup_retention_count = Set(Some(auto_backup_retention_count));
            has_updates = true;
        }
    }

    if !has_updates {
        return Err(StatusCode::BAD_REQUEST);
    }

    active.updated_at = Set(chrono::Utc::now().into());

    let tenant = state
        .store
        .tenants()
        .update(active)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update tenant: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Log the settings change
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: id,
            user_id: Some(auth.user_id),
            action: "tenant_settings_updated".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: Some(id),
            metadata: Some(json!({
                "compliance_mode": compliance_mode,
            })),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    // Invalidate ALL caches after tenant update
    if let Some(ref cache) = state.cache {
        use app_core::cache::keys;
        let compliance_key = keys::compliance(id);
        let tenant_key = keys::tenant(id);
        let _ = cache.delete(&compliance_key).await;
        let _ = cache.delete(&tenant_key).await;
        let _ = cache.delete_pattern("clovalink:user:*").await;
        tracing::info!("Invalidated all caches for tenant {} after edit", id);
    }

    Ok(Json(json!({
        "id": tenant.id,
        "name": tenant.name,
        "domain": tenant.domain,
        "plan": tenant.plan,
        "status": tenant.status,
        "compliance_mode": tenant.compliance_mode,
        "storage_quota_bytes": tenant.storage_quota_bytes,
        "max_upload_size_bytes": tenant.max_upload_size_bytes,
        "retention_policy_days": tenant.retention_policy_days,
        "smtp_host": tenant.smtp_host,
        "smtp_port": tenant.smtp_port,
        "smtp_username": tenant.smtp_username,
        "smtp_from": tenant.smtp_from,
        "smtp_secure": tenant.smtp_secure,
        "enable_totp": tenant.enable_totp,
        "mfa_required": tenant.mfa_required,
        "session_timeout_minutes": tenant.session_timeout_minutes,
        "public_sharing_enabled": tenant.public_sharing_enabled,
        "data_export_enabled": tenant.data_export_enabled.unwrap_or(true),
        "restrictions": restrictions,
    })))
}

/// Switch active tenant (generates new token for different tenant)
/// POST /api/tenants/switch/:tenant_id
/// User must have access to the target tenant
pub async fn switch_tenant(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!(
        "Switch tenant request: user={}, target_tenant={}, role={}",
        auth.user_id,
        tenant_id,
        auth.role
    );

    let user = state
        .store
        .users()
        .user(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch user for tenant switch: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify user has access to this tenant
    // Only SuperAdmin can switch to any tenant
    if auth.role.as_str() != "SuperAdmin" {
        let has_access = user.tenant_id == tenant_id
            || user
                .allowed_tenant_ids
                .as_ref()
                .map(|ids| ids.contains(&tenant_id))
                .unwrap_or(false);

        if !has_access {
            tracing::warn!(
                "User {} denied access to tenant {}",
                auth.user_id,
                tenant_id
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Get tenant info (must be active)
    let tenant = state
        .store
        .tenants()
        .by_id(tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch tenant {}: {:?}", tenant_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let tenant = match tenant {
        Some(t) if t.status == "active" => t,
        Some(t) => {
            tracing::warn!(
                "Attempted to switch to non-active tenant {} (status: {})",
                tenant_id,
                t.status
            );
            return Err(StatusCode::FORBIDDEN);
        }
        None => {
            tracing::warn!("Tenant {} not found", tenant_id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    let token = generate_token(auth.user_id, tenant_id, user.role.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Invalidate user cache so next /api/auth/me returns fresh tenant data
    if let Some(ref cache) = state.cache {
        use app_core::cache::keys;
        let user_key = keys::user(auth.user_id);
        let _ = cache.delete(&user_key).await;
        tracing::info!(
            "Invalidated user cache for {} after tenant switch to {}",
            auth.user_id,
            tenant_id
        );
    }

    let restrictions = ComplianceRestrictions::for_mode(&tenant.compliance_mode);

    Ok(Json(json!({
        "token": token,
        "tenant": {
            "id": tenant.id,
            "name": tenant.name,
            "domain": tenant.domain,
            "plan": tenant.plan,
            "compliance_mode": tenant.compliance_mode,
            "retention_policy_days": tenant.retention_policy_days,
            "mfa_required": tenant.mfa_required,
            "session_timeout_minutes": tenant.session_timeout_minutes,
            "public_sharing_enabled": tenant.public_sharing_enabled,
            "data_export_enabled": tenant.data_export_enabled.unwrap_or(true),
            "restrictions": restrictions,
        }
    })))
}

#[derive(Deserialize)]
pub struct TestSmtpInput {
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub secure: bool,
}

/// Test SMTP connection
/// POST /api/tenants/:id/smtp/test
pub async fn test_smtp(
    State(_state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(_id): Path<Uuid>,
    Json(input): Json<TestSmtpInput>,
) -> Result<Json<Value>, StatusCode> {
    // Only Admin/Owner can test SMTP
    if auth.role != "SuperAdmin" && auth.role != "Owner" && auth.role != "Admin" {
        return Err(StatusCode::FORBIDDEN);
    }

    app_core::mailer::test_smtp_connection(
        &input.host,
        input.port,
        &input.username,
        &input.password,
        input.secure,
    )
    .await
    .map_err(|e| {
        tracing::error!("SMTP Test Failed: {:?}", e);
        StatusCode::BAD_REQUEST
    })?;

    Ok(Json(json!({"success": true})))
}

#[derive(Deserialize)]
pub struct SuspendTenantInput {
    pub reason: Option<String>,
}

/// Suspend a tenant/company
/// POST /api/tenants/:id/suspend
/// SuperAdmin only
pub async fn suspend_tenant(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<SuspendTenantInput>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    // Cannot suspend your own tenant
    if id == auth.tenant_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check tenant exists
    let tenant = state
        .store
        .tenants()
        .by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if tenant.status == "suspended" {
        return Ok(Json(json!({
            "success": true,
            "message": "Tenant is already suspended"
        })));
    }

    // Update tenant status to suspended
    state
        .store
        .tenants()
        .suspend(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: auth.tenant_id,
            user_id: Some(auth.user_id),
            action: "suspend_tenant".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: Some(id),
            metadata: Some(json!({
                "tenant_name": tenant.name,
                "reason": input.reason
            })),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    // Invalidate caches
    if let Some(ref cache) = state.cache {
        use app_core::cache::keys;
        let tenant_key = keys::tenant(id);
        let _ = cache.delete(&tenant_key).await;
        let _ = cache.delete_pattern("clovalink:user:*").await;
    }

    tracing::info!(
        "SuperAdmin {} suspended tenant {} ({})",
        auth.user_id,
        id,
        tenant.name
    );

    Ok(Json(json!({
        "success": true,
        "message": "Tenant suspended successfully"
    })))
}

/// Unsuspend a tenant/company
/// POST /api/tenants/:id/unsuspend
/// SuperAdmin only
pub async fn unsuspend_tenant(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    // Check tenant exists
    let tenant = state
        .store
        .tenants()
        .by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if tenant.status != "suspended" {
        return Ok(Json(json!({
            "success": true,
            "message": "Tenant is not suspended"
        })));
    }

    // Update tenant status to active
    state
        .store
        .tenants()
        .unsuspend(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: auth.tenant_id,
            user_id: Some(auth.user_id),
            action: "unsuspend_tenant".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: Some(id),
            metadata: Some(json!({
                "tenant_name": tenant.name
            })),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    // Invalidate caches
    if let Some(ref cache) = state.cache {
        use app_core::cache::keys;
        let tenant_key = keys::tenant(id);
        let _ = cache.delete(&tenant_key).await;
        let _ = cache.delete_pattern("clovalink:user:*").await;
    }

    tracing::info!(
        "SuperAdmin {} unsuspended tenant {} ({})",
        auth.user_id,
        id,
        tenant.name
    );

    Ok(Json(json!({
        "success": true,
        "message": "Tenant unsuspended successfully"
    })))
}

/// Permanently delete a tenant/company
/// DELETE /api/tenants/:id
/// SuperAdmin only - DANGER: This permanently deletes ALL data
pub async fn delete_tenant(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    // Cannot delete your own tenant
    if id == auth.tenant_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check tenant exists
    let tenant = state
        .store
        .tenants()
        .by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Delete in cascade order (foreign key constraints handled inside repository)
    let (users_deleted, files_deleted) = state
        .store
        .tenants()
        .delete_cascade(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to cascade delete tenant {}: {:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Audit log (in SuperAdmin's tenant)
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: auth.tenant_id,
            user_id: Some(auth.user_id),
            action: "delete_tenant".to_string(),
            resource_type: "tenant".to_string(),
            resource_id: Some(id),
            metadata: Some(json!({
                "tenant_name": tenant.name,
                "tenant_domain": tenant.domain,
                "users_deleted": users_deleted,
                "files_deleted": files_deleted,
            })),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    // Invalidate caches
    if let Some(ref cache) = state.cache {
        use app_core::cache::keys;
        let tenant_key = keys::tenant(id);
        let _ = cache.delete(&tenant_key).await;
        let _ = cache.delete_pattern("clovalink:user:*").await;
    }

    tracing::warn!(
        "SuperAdmin {} PERMANENTLY DELETED tenant {} ({}) - {} users, {} files removed",
        auth.user_id,
        id,
        tenant.name,
        users_deleted,
        files_deleted
    );

    Ok(Json(json!({
        "success": true,
        "message": "Tenant permanently deleted",
        "deleted": {
            "tenant_name": tenant.name,
            "users": users_deleted,
            "files": files_deleted
        }
    })))
}
