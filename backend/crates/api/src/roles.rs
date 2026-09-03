use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::{DateTime,Utc};
use clovalink_auth::AuthUser;
use clovalink_core::models::{
    get_base_permissions, CreateRoleInput, UpdateRoleInput, UpdateRolePermissionsInput,
    ALL_PERMISSIONS,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

// ==================== Query Parameters ====================

#[derive(Debug, Deserialize)]
pub struct ListRolesParams {
    pub include_global: Option<bool>, // Include global roles (default true)
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleParams {
    pub is_global: Option<bool>, // Create as global role (SuperAdmin only)
}

// ==================== Response Types ====================

#[derive(Debug, Serialize)]
pub struct RoleWithPermissionsResponse {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub base_role: String,
    pub is_system: bool,
    pub permissions: Vec<PermissionResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PermissionResponse {
    pub permission: String,
    pub granted: bool,
    pub inherited: bool, // true if from base role, false if custom
}

// ==================== Handlers ====================

/// List all roles for the tenant (includes global roles)
/// GET /api/roles
pub async fn list_roles(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<ListRolesParams>,
) -> Result<Json<Value>, StatusCode> {
    let include_global = params.include_global.unwrap_or(true);

    let roles=state.store.roles().list(auth.tenant_id,include_global).await;

    match roles {
        Ok(roles) => Ok(Json(json!(roles))),
        Err(e) => {
            tracing::error!("Failed to list roles: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create a new custom role for the tenant (or global if SuperAdmin)
/// POST /api/roles
pub async fn create_role(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<CreateRoleParams>,
    Json(input): Json<CreateRoleInput>,
) -> Result<Json<Value>, StatusCode> {
    // Validate base_role
    let valid_base_roles = ["Employee", "Manager", "Admin", "SuperAdmin"];
    if !valid_base_roles.contains(&input.base_role.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check permission (must be Admin or SuperAdmin)
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Determine if this should be a global role
    let is_global = params.is_global.unwrap_or(false);

    // Only SuperAdmin can create global roles
    if is_global && auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Set tenant_id to NULL for global roles, otherwise use auth.tenant_id
    let tenant_id: Option<Uuid> = if is_global {
        None
    } else {
        Some(auth.tenant_id)
    };

    // Create role
    let role = state.store.roles().create(tenant_id,input.name.clone(),input.description.clone(),input.base_role.clone(),input.permissions.clone().unwrap_or_default()).await;
    if let Err(e) = &role {
        tracing::error!("Failed to create role: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let role=role.unwrap(); let role_id=role.id;

    // Log audit event
    let _=state.store.system().audit_resource(auth.tenant_id,auth.user_id,"role_created","role",role_id,json!({"role_name":input.name}),auth.ip_address.as_deref()).await;

    // Fetch and return the created role
    Ok(Json(json!(role)))
}

/// Get a specific role with its permissions
/// GET /api/roles/:id
pub async fn get_role(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(role_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Fetch the role
    let role=state.store.roles().accessible(role_id,auth.tenant_id)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch role: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let role = match role {
        Some(r) => r,
        None => return Err(StatusCode::NOT_FOUND),
    };

    // Get permissions for this role
    let permissions = get_role_permissions(&state.store, role_id, &role.base_role).await?;

    let response = RoleWithPermissionsResponse {
        id: role.id,
        tenant_id: role.tenant_id,
        name: role.name,
        description: role.description,
        base_role: role.base_role,
        is_system: role.is_system,
        permissions,
        created_at: role.created_at,
        updated_at: role.updated_at,
    };

    Ok(Json(json!(response)))
}

/// Update a custom role (or system role if SuperAdmin)
/// PUT /api/roles/:id
pub async fn update_role(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(role_id): Path<Uuid>,
    Json(input): Json<UpdateRoleInput>,
) -> Result<Json<Value>, StatusCode> {
    // Check permission
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Fetch role - SuperAdmin can access any role, others only their tenant's roles
    let role = if auth.role == "SuperAdmin" {state.store.roles().by_id(role_id).await} else {state.store.roles().tenant_role(role_id,auth.tenant_id).await}
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // SuperAdmin can edit system roles, others cannot
    let role = match role {
        Some(r) if !r.is_system => r,
        Some(r) if r.is_system && auth.role == "SuperAdmin" => r, // SuperAdmin can edit system roles
        Some(_) => return Err(StatusCode::FORBIDDEN), // Non-SuperAdmin cannot edit system roles
        None => return Err(StatusCode::NOT_FOUND),
    };

    if input.name.is_none()&&input.description.is_none()&&input.base_role.is_none() {
        return Ok(Json(json!(role)));
    }
    let updated_role = state.store.roles().update(role_id,clovalink_entity::repositories::RolePatch{name:input.name,description:input.description,base_role:input.base_role}).await.map_err(|e| {
        tracing::error!("Failed to update role: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?.ok_or(StatusCode::NOT_FOUND)?;

    // Log audit event
    let _=state.store.system().audit_resource(auth.tenant_id,auth.user_id,"role_updated","role",role_id,json!({"role_name":updated_role.name}),auth.ip_address.as_deref()).await;

    Ok(Json(json!(updated_role)))
}

/// Delete a custom role
/// DELETE /api/roles/:id
pub async fn delete_role(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(role_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Check permission
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if role exists and is deletable (not system role, belongs to tenant)
    let role=state.store.roles().tenant_role(role_id,auth.tenant_id)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match role {
        Some(r) if r.is_system => return Err(StatusCode::FORBIDDEN),
        None => return Err(StatusCode::NOT_FOUND),
        _ => {}
    }

    // Check if any users are using this role
    let user_count=state.store.roles().user_count(role_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if user_count > 0 {
        return Err(StatusCode::CONFLICT); // Role is in use
    }

    // Delete the role (cascade will delete permissions)
    state.store.roles().delete(role_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete role: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Log audit event
    let _=state.store.system().audit_resource(auth.tenant_id,auth.user_id,"role_deleted","role",role_id,json!({}),auth.ip_address.as_deref()).await;

    Ok(StatusCode::NO_CONTENT)
}

/// Get permissions for a role
/// GET /api/roles/:id/permissions
pub async fn get_role_permissions_handler(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(role_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Fetch the role
    let role=state.store.roles().accessible(role_id,auth.tenant_id)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let role = match role {
        Some(r) => r,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let permissions = get_role_permissions(&state.store, role_id, &role.base_role).await?;

    Ok(Json(json!({
        "role_id": role_id,
        "base_role": role.base_role,
        "permissions": permissions,
        "all_permissions": ALL_PERMISSIONS,
    })))
}

/// Update permissions for a role (SuperAdmin can edit any role including system roles)
/// PUT /api/roles/:id/permissions
pub async fn update_role_permissions(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(role_id): Path<Uuid>,
    Json(input): Json<UpdateRolePermissionsInput>,
) -> Result<Json<Value>, StatusCode> {
    // Check permission
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Fetch role - SuperAdmin can access any role, others only their tenant's roles
    let role=if auth.role=="SuperAdmin"{state.store.roles().by_id(role_id).await}else{state.store.roles().tenant_role(role_id,auth.tenant_id).await}
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // SuperAdmin can edit system roles, others cannot
    let role = match role {
        Some(r) if !r.is_system => r,
        Some(r) if r.is_system && auth.role == "SuperAdmin" => r, // SuperAdmin can edit system roles
        Some(_) => return Err(StatusCode::FORBIDDEN), // Non-SuperAdmin cannot edit system roles
        None => return Err(StatusCode::NOT_FOUND),
    };

    // Update permissions
    for perm in &input.permissions {
        // Validate permission exists
        if !ALL_PERMISSIONS.contains(&perm.permission.as_str()) {
            continue;
        }

        // Upsert permission
        state.store.roles().upsert_permissions(role_id,&[(perm.permission.clone(),perm.granted)]).await.map_err(|e|{tracing::error!("Failed to update permission: {:?}",e);StatusCode::INTERNAL_SERVER_ERROR})?;
    }

    // Log audit event
    let _=state.store.system().audit_resource(auth.tenant_id,auth.user_id,"role_permissions_updated","role",role_id,json!({"role_name":role.name}),auth.ip_address.as_deref()).await;

    // Invalidate cache for all users with this role
    // This ensures they get fresh permissions on next /api/auth/me call
    if let Some(ref cache) = state.cache {
        // Find all users with this role and invalidate their cache
        let users_with_role=state.store.roles().user_ids_by_role(&role.name).await.unwrap_or_default();

        for user_id in users_with_role {
            let cache_key = clovalink_core::cache::keys::user(user_id);
            if let Err(e) = cache.delete(&cache_key).await {
                tracing::warn!("Failed to invalidate cache for user {}: {}", user_id, e);
            }
        }

        tracing::info!("Invalidated cache for users with role '{}'", role.name);
    }

    // Return updated permissions
    let permissions = get_role_permissions(&state.store, role_id, &role.base_role).await?;

    Ok(Json(json!({
        "role_id": role_id,
        "permissions": permissions,
    })))
}

// ==================== Helper Functions ====================

async fn get_role_permissions(
    store: &clovalink_entity::DataStore,
    role_id: Uuid,
    base_role: &str,
) -> Result<Vec<PermissionResponse>, StatusCode> {
    // SuperAdmin always has all permissions
    if base_role == "SuperAdmin" {
        return Ok(ALL_PERMISSIONS
            .iter()
            .map(|perm| PermissionResponse {
                permission: perm.to_string(),
                granted: true,
                inherited: true, // SuperAdmin permissions are inherent
            })
            .collect());
    }

    // Get base permissions for the role level
    let base_perms: Vec<&str> = get_base_permissions(base_role);

    // Get custom permissions for this specific role
    let custom_perms=store.roles().permissions(role_id)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch role permissions: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Build permission list
    let mut permissions: Vec<PermissionResponse> = vec![];

    // Add all permissions with their status
    for perm in ALL_PERMISSIONS {
        let is_base = base_perms.contains(perm);
        let custom = custom_perms.iter().find(|p| p.0 == *perm);

        let (granted, inherited) = match custom {
            Some(c) => (c.1, false), // Custom override
            None => (is_base, is_base),    // Use base permission
        };

        permissions.push(PermissionResponse {
            permission: perm.to_string(),
            granted,
            inherited,
        });
    }

    Ok(permissions)
}
