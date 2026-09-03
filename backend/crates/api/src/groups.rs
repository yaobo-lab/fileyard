//! File Groups API handlers
//!
//! Allows users to create and manage file groups - manual collections of related files.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use clovalink_auth::AuthUser;

/// Maximum number of files allowed per group
const MAX_FILES_PER_GROUP: i64 = 20;

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if a group is inside a company folder
async fn is_group_in_company_folder(
    store: &clovalink_entity::DataStore,
    tenant_id: Uuid,
    group_id: Uuid,
) -> clovalink_entity::DataResult<bool> {
    store.groups().in_company_folder(tenant_id, group_id).await
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Serialize)]
pub struct FileGroup {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub department_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub parent_path: Option<String>,
    pub visibility: String,
    pub owner_id: Option<Uuid>,
    pub is_locked: Option<bool>,
    pub locked_by: Option<Uuid>,
    pub locked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub lock_requires_role: Option<String>,
}
impl From<clovalink_entity::entities::file_groups::Model> for FileGroup {
    fn from(v: clovalink_entity::entities::file_groups::Model) -> Self {
        Self {
            id: v.id,
            tenant_id: v.tenant_id,
            department_id: v.department_id,
            name: v.name,
            description: v.description,
            color: v.color,
            icon: v.icon,
            created_by: v.created_by,
            created_at: v.created_at.with_timezone(&chrono::Utc),
            updated_at: v.updated_at.with_timezone(&chrono::Utc),
            parent_path: v.parent_path,
            visibility: v.visibility,
            owner_id: v.owner_id,
            is_locked: v.is_locked,
            locked_by: v.locked_by,
            locked_at: v.locked_at.map(|x| x.with_timezone(&chrono::Utc)),
            lock_requires_role: v.lock_requires_role,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileGroupWithCount {
    #[serde(flatten)]
    pub group: FileGroup,
    pub file_count: i64,
    pub total_size: i64, // Total size in bytes of all files in the group
    pub owner_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupInput {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub department_id: Option<String>,
    pub visibility: Option<String>, // 'department' (default) or 'private'
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListGroupsParams {
    pub department_id: Option<String>,
    pub parent_path: Option<String>, // Filter by folder path (empty string = root)
    pub visibility: Option<String>,  // 'department' or 'private' filter
}

#[derive(Debug, Deserialize)]
pub struct AddToGroupInput {
    pub group_id: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// List all groups for the tenant (filtered by department if applicable)
/// GET /api/groups/{company_id}
pub async fn list_groups(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<String>,
    Query(params): Query<ListGroupsParams>,
) -> Result<Json<Vec<FileGroupWithCount>>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Parse department filter from query params (for admin filtering)
    let dept_filter: Option<Uuid> = params.department_id.as_ref().and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Uuid::parse_str(s).ok()
        }
    });

    // Parse parent_path filter (empty string means root, None means all)
    let path_filter = params.parent_path.as_deref();

    // Parse visibility filter
    let visibility_filter = params.visibility.as_deref();

    // Get user's department and allowed departments for non-admin filtering
    let user_dept_info = state
        .store
        .users()
        .departments(auth.user_id)
        .await
        .unwrap_or(None);

    let user_department_id = user_dept_info.as_ref().and_then(|u| u.0);
    let user_allowed_depts: Vec<Uuid> = user_dept_info
        .as_ref()
        .and_then(|u| u.1.clone())
        .unwrap_or_default();

    // Query groups with file counts, total size, owner names, and locking info
    // Filter by parent_path to only show groups in the current folder
    // Filter by visibility: department groups visible to all, private only to owner (or admins)
    // For non-admins: only show groups in their department(s), NOT groups with NULL department
    let groups = state
        .store
        .groups()
        .list(
            tenant_id,
            dept_filter,
            path_filter,
            visibility_filter,
            auth.user_id,
            &auth.role,
            user_department_id,
            &user_allowed_depts,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list groups: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let result: Vec<FileGroupWithCount> = groups
        .into_iter()
        .map(|row| FileGroupWithCount {
            group: FileGroup {
                id: row.id,
                tenant_id: row.tenant_id,
                department_id: row.department_id,
                name: row.name,
                description: row.description,
                color: row.color,
                icon: row.icon,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
                parent_path: row.parent_path,
                visibility: row.visibility,
                owner_id: row.owner_id,
                is_locked: row.is_locked,
                locked_by: row.locked_by,
                locked_at: row.locked_at,
                lock_requires_role: row.lock_requires_role,
            },
            file_count: row.file_count,
            total_size: row.total_size,
            owner_name: row.owner_name,
        })
        .collect();

    Ok(Json(result))
}

/// Create a new file group
/// POST /api/groups/{company_id}
pub async fn create_group(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<String>,
    Json(input): Json<CreateGroupInput>,
) -> Result<Json<FileGroup>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate name
    let name = input.name.trim();
    if name.is_empty() || name.len() > 255 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Parse department_id
    let department_id: Option<Uuid> = input.department_id.as_ref().and_then(|s| {
        if s.is_empty() {
            None
        } else {
            Uuid::parse_str(s).ok()
        }
    });

    // Parse and validate visibility (default to 'department')
    let visibility = input.visibility.as_deref().unwrap_or("department");
    let visibility = if visibility == "private" {
        "private"
    } else {
        "department"
    };

    // Validate color format if provided
    if let Some(ref color) = input.color {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Create the group with visibility and owner_id
    let group: FileGroup = state
        .store
        .groups()
        .create(clovalink_entity::repositories::NewGroup {
            tenant_id,
            department_id,
            name: name.into(),
            description: input.description,
            color: input.color,
            icon: input.icon.unwrap_or_else(|| "folder-kanban".into()),
            user_id: auth.user_id,
            visibility: visibility.into(),
        })
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                tracing::warn!("Duplicate group name: {}", name);
                StatusCode::CONFLICT
            } else {
                tracing::error!("Failed to create group: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?
        .into();

    // Audit log
    let _ = state
        .store
        .system()
        .audit_resource(
            tenant_id,
            auth.user_id,
            "group_created",
            "file_group",
            group.id,
            json!({"name":name,"visibility":visibility}),
            auth.ip_address.as_deref(),
        )
        .await;

    tracing::info!(user_id = %auth.user_id, group_id = %group.id, "File group created");

    Ok(Json(group))
}

/// Update a file group
/// PUT /api/groups/{company_id}/{group_id}
pub async fn update_group(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, group_id)): Path<(String, String)>,
    Json(input): Json<UpdateGroupInput>,
) -> Result<Json<FileGroup>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let group_uuid = Uuid::parse_str(&group_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if group is inside company folder - only admins can modify
    if is_group_in_company_folder(&state.store, tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        if auth.role != "SuperAdmin" && auth.role != "Admin" {
            tracing::warn!(
                "Security: Non-admin user {} attempted to rename group in company folder",
                auth.user_id
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Check group exists and belongs to tenant
    let existing = state
        .store
        .groups()
        .get(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if existing.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Validate color format if provided
    if let Some(ref color) = input.color {
        if !color.starts_with('#') || color.len() != 7 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Build update query dynamically
    let group: FileGroup = state
        .store
        .groups()
        .update(
            tenant_id,
            group_uuid,
            clovalink_entity::repositories::GroupPatch {
                name: input.name,
                description: input.description,
                color: input.color,
                icon: input.icon,
            },
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                StatusCode::CONFLICT
            } else {
                tracing::error!("Failed to update group: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?
        .ok_or(StatusCode::NOT_FOUND)?
        .into();

    tracing::info!(user_id = %auth.user_id, group_id = %group_uuid, "File group updated");

    Ok(Json(group))
}

/// Delete a file group (files are unlinked, not deleted)
/// DELETE /api/groups/{company_id}/{group_id}
pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, group_id)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let group_uuid = Uuid::parse_str(&group_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if group is inside company folder - only admins can delete
    if is_group_in_company_folder(&state.store, tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        if auth.role != "SuperAdmin" && auth.role != "Admin" {
            tracing::warn!(
                "Security: Non-admin user {} attempted to delete group in company folder",
                auth.user_id
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Get group info for audit log
    let group_name = state
        .store
        .groups()
        .get(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(group) = group_name else {
        return Err(StatusCode::NOT_FOUND);
    };

    // Delete the group (files will have group_id set to NULL due to ON DELETE SET NULL)
    let name = group.name;
    state
        .store
        .groups()
        .delete(tenant_id, group_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete group: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Audit log
    let _ = state
        .store
        .system()
        .audit_resource(
            tenant_id,
            auth.user_id,
            "group_deleted",
            "file_group",
            group_uuid,
            json!({"name":name}),
            auth.ip_address.as_deref(),
        )
        .await;

    tracing::info!(user_id = %auth.user_id, group_id = %group_uuid, "File group deleted");

    Ok(Json(
        json!({ "success": true, "message": "Group deleted. Files have been unlinked." }),
    ))
}

/// Add a file to a group
/// POST /api/files/{company_id}/{file_id}/group
pub async fn add_file_to_group(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, file_id)): Path<(String, String)>,
    Json(input): Json<AddToGroupInput>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let file_uuid = Uuid::parse_str(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let group_uuid = Uuid::parse_str(&input.group_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify file exists and belongs to tenant
    let file_exists = state
        .store
        .groups()
        .active_file(tenant_id, file_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(file) = file_exists else {
        return Err(StatusCode::NOT_FOUND);
    };
    if file.is_directory {
        return Err(StatusCode::NOT_FOUND);
    }
    let file_name = file.name;

    // Verify group exists and belongs to tenant
    let group_exists = state
        .store
        .groups()
        .get(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(group) = group_exists else {
        return Err(StatusCode::NOT_FOUND);
    };
    let group_name = group.name;

    // Check if group already has max files
    let current_count = state
        .store
        .groups()
        .file_count(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if current_count as i64 >= MAX_FILES_PER_GROUP {
        tracing::warn!(
            "Group {} has reached max file limit of {}",
            group_uuid,
            MAX_FILES_PER_GROUP
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update file's group_id
    state
        .store
        .groups()
        .set_file_group(tenant_id, file_uuid, Some(group_uuid))
        .await
        .map_err(|e| {
            tracing::error!("Failed to add file to group: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        user_id = %auth.user_id,
        file_id = %file_uuid,
        group_id = %group_uuid,
        "File added to group"
    );

    // Invalidate file cache since grouped files are now hidden from main list
    if let Some(ref cache) = state.cache {
        let pattern = format!("clovalink:files:{}:*", tenant_id);
        if let Err(e) = cache.delete_pattern(&pattern).await {
            tracing::warn!("Failed to invalidate file cache: {}", e);
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": format!("'{}' added to group '{}'", file_name, group_name)
    })))
}

/// Remove a file from its group
/// DELETE /api/files/{company_id}/{file_id}/group
pub async fn remove_file_from_group(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, file_id)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let file_uuid = Uuid::parse_str(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Get file info including parent_path, name, visibility to check for duplicates
    let file_info = state
        .store
        .groups()
        .active_file(tenant_id, file_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch file info: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let Some(file) = file_info.filter(|f| f.group_id.is_some()) else {
        return Err(StatusCode::NOT_FOUND);
    };
    let file_name = file.name;
    let parent_path = file.parent_path;
    let visibility = file.visibility;

    // Check for duplicate filename at the target location (files NOT in a group)
    let duplicate = state
        .store
        .groups()
        .duplicate_ungrouped(
            tenant_id,
            file_uuid,
            &file_name,
            parent_path.as_deref(),
            &visibility,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to check for duplicate: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if duplicate {
        // Generate suggested name: "file (1).ext"
        let name_without_ext = std::path::Path::new(&file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_name);
        let extension = std::path::Path::new(&file_name)
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let suggested_name = format!("{} (1){}", name_without_ext, extension);

        tracing::warn!(
            file_id = %file_uuid,
            file_name = %file_name,
            "Cannot remove file from group - duplicate name exists at target location"
        );

        return Ok(Json(json!({
            "error": format!("A file named \"{}\" already exists in this location. Rename the file first or remove the conflicting file.", file_name),
            "duplicate": true,
            "conflicting_name": file_name,
            "suggested_name": suggested_name
        })));
    }

    // Remove file from group
    let removed = state
        .store
        .groups()
        .set_file_group(tenant_id, file_uuid, None)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove file from group: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !removed {
        return Err(StatusCode::NOT_FOUND);
    }

    tracing::info!(user_id = %auth.user_id, file_id = %file_uuid, "File removed from group");

    // Invalidate file cache since file is now visible in main list again
    if let Some(ref cache) = state.cache {
        let pattern = format!("clovalink:files:{}:*", tenant_id);
        if let Err(e) = cache.delete_pattern(&pattern).await {
            tracing::warn!("Failed to invalidate file cache: {}", e);
        }
    }

    Ok(Json(
        json!({ "success": true, "message": "File removed from group" }),
    ))
}

/// Get files in a specific group
/// GET /api/groups/{company_id}/{group_id}/files
pub async fn get_group_files(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, group_id)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let group_uuid = Uuid::parse_str(&group_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Get group info
    let group: Option<FileGroup> = state
        .store
        .groups()
        .get(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Into::into);

    let Some(group) = group else {
        return Err(StatusCode::NOT_FOUND);
    };

    // Check visibility access: private groups only visible to owner or admins
    if group.visibility == "private"
        && group.owner_id != Some(auth.user_id)
        && auth.role != "SuperAdmin"
        && auth.role != "Admin"
    {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if user can access locked group
    if group.is_locked.unwrap_or(false) {
        if !can_access_locked_group(
            auth.user_id,
            &auth.role,
            true,
            group.locked_by,
            group.created_by,
            group.lock_requires_role.as_deref(),
        ) {
            tracing::warn!(
                "Access denied: user {} (role: {}) attempted to access locked group {} (requires: {:?})",
                auth.user_id, auth.role, group_uuid, group.lock_requires_role
            );
            return Ok(Json(json!({
                "error": "Group is locked - access denied",
                "is_locked": true,
                "lock_requires_role": group.lock_requires_role
            })));
        }
    }

    // Get files in this group
    tracing::info!(
        "Fetching files for group {} in tenant {}",
        group_uuid,
        tenant_id
    );

    let files = state
        .store
        .groups()
        .files(tenant_id, group_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get group files: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Found {} files in group {}", files.len(), group_uuid);

    let files_json: Vec<Value> = files
        .into_iter()
        .map(|f| {
            json!({
                "id":f.id,"name":f.name,"size_bytes":f.size_bytes,"content_type":f.content_type,
                "parent_path":f.parent_path,"owner_id":f.owner_id,"created_at":f.created_at,
                "type": "file"
            })
        })
        .collect();

    Ok(Json(json!({
        "group": group,
        "files": files_json
    })))
}

#[derive(Debug, Deserialize)]
pub struct MoveGroupInput {
    pub target_folder_id: Option<String>,
    pub target_path: Option<String>,
    pub target_visibility: Option<String>, // 'department' or 'private'
}

/// Move a group to a folder (updates the group's parent_path, not the files)
/// PUT /api/groups/{company_id}/{group_id}/move
pub async fn move_group_to_folder(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, group_id)): Path<(String, String)>,
    Json(input): Json<MoveGroupInput>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let group_uuid = Uuid::parse_str(&group_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if group is inside company folder - only admins can move
    if is_group_in_company_folder(&state.store, tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        if auth.role != "SuperAdmin" && auth.role != "Admin" {
            tracing::warn!(
                "Security: Non-admin user {} attempted to move group in company folder",
                auth.user_id
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Get group info including current visibility
    let group = state
        .store
        .groups()
        .get(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(group) = group else {
        return Err(StatusCode::NOT_FOUND);
    };
    let group_name = group.name;
    let old_path = group.parent_path;
    let current_visibility = group.visibility;

    // Check if trying to change visibility - groups are locked to their original visibility
    if let Some(ref target_vis) = input.target_visibility {
        if target_vis != &current_visibility {
            tracing::warn!(
                group_id = %group_uuid,
                current = %current_visibility,
                target = %target_vis,
                "Attempted to move group across visibility boundary"
            );
            return Ok(Json(json!({
                "error": "Groups cannot be moved between department and private files. They are locked to their original visibility.",
                "visibility_locked": true
            })));
        }
    }

    // Determine target path
    let target_path = if let Some(folder_id) = &input.target_folder_id {
        let folder_uuid = Uuid::parse_str(folder_id).map_err(|_| StatusCode::BAD_REQUEST)?;

        // Get folder's path
        let folder = state
            .store
            .groups()
            .folder(tenant_id, folder_uuid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let Some((folder_name, parent_path)) = folder else {
            return Err(StatusCode::NOT_FOUND);
        };

        // Construct full path to folder
        match parent_path {
            Some(pp) if !pp.is_empty() => format!("{}/{}", pp, folder_name),
            _ => folder_name,
        }
    } else if let Some(path) = &input.target_path {
        path.clone()
    } else {
        // Move to root
        String::new()
    };

    // Update the GROUP's parent_path only (visibility is locked)
    state
        .store
        .groups()
        .move_group(
            tenant_id,
            group_uuid,
            if target_path.is_empty() {
                None
            } else {
                Some(target_path.clone())
            },
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to move group: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Audit log
    let _ = state
        .store
        .system()
        .audit_resource(
            tenant_id,
            auth.user_id,
            "group_moved",
            "file_group",
            group_uuid,
            json!({"name":group_name,"from":old_path,"to":target_path}),
            auth.ip_address.as_deref(),
        )
        .await;

    tracing::info!(
        user_id = %auth.user_id,
        group_id = %group_uuid,
        target_path = %target_path,
        "Group moved to folder"
    );

    // Invalidate file cache
    if let Some(ref cache) = state.cache {
        let pattern = format!("clovalink:files:{}:*", tenant_id);
        if let Err(e) = cache.delete_pattern(&pattern).await {
            tracing::warn!("Failed to invalidate file cache: {}", e);
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": format!("Moved group '{}' to '{}'", group_name, if target_path.is_empty() { "root" } else { &target_path })
    })))
}

// ============================================================================
// Group Locking
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LockGroupInput {
    pub password: Option<String>, // Optional password for additional security
    pub required_role: Option<String>, // Optional role requirement (Admin, Manager, Employee)
}

#[derive(Debug, Deserialize)]
pub struct UnlockGroupInput {
    pub password: Option<String>, // Password if the group is password-locked
}

/// Lock a group (prevents access to files within)
/// POST /api/groups/{company_id}/{group_id}/lock
pub async fn lock_group(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, group_id)): Path<(String, String)>,
    Json(input): Json<LockGroupInput>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let group_uuid = Uuid::parse_str(&group_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if group is inside company folder - only admins can lock
    if is_group_in_company_folder(&state.store, tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        if auth.role != "SuperAdmin" && auth.role != "Admin" {
            tracing::warn!(
                "Security: Non-admin user {} attempted to lock group in company folder",
                auth.user_id
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Check if user has lock permission (Manager, Admin, SuperAdmin)
    let has_lock_permission = ["SuperAdmin", "Admin", "Manager"].contains(&auth.role.as_str());
    if !has_lock_permission {
        // Check for custom role with files.lock permission
        let custom_role_has_perm = state
            .store
            .groups()
            .custom_permission(tenant_id, &auth.role, "files.lock")
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if !custom_role_has_perm {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Get current group status
    let group = state
        .store
        .groups()
        .get(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let group = group.ok_or(StatusCode::NOT_FOUND)?;
    let group_name = group.name;
    let is_locked = group.is_locked.unwrap_or(false);
    let locked_by = group.locked_by;

    if is_locked {
        return Ok(Json(json!({
            "error": "Group is already locked",
            "locked_by": locked_by
        })));
    }

    // Process optional password and role requirement
    let password_hash: Option<String> = if let Some(ref pwd) = input.password {
        if !pwd.is_empty() {
            use argon2::password_hash::rand_core::OsRng;
            use argon2::{password_hash::SaltString, PasswordHasher};
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = crate::password::get_argon2();
            Some(
                argon2
                    .hash_password(pwd.as_bytes(), &salt)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .to_string(),
            )
        } else {
            None
        }
    } else {
        None
    };

    let required_role = input.required_role.clone();

    // Lock the group
    state
        .store
        .groups()
        .lock(
            tenant_id,
            group_uuid,
            auth.user_id,
            password_hash.clone(),
            required_role.clone(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .system()
        .audit_resource(
            tenant_id,
            auth.user_id,
            "group_locked",
            "file_group",
            group_uuid,
            json!({
                "name": group_name,
                "has_password": password_hash.is_some(),
                "requires_role": required_role
            }),
            auth.ip_address.as_deref(),
        )
        .await;

    tracing::info!(
        user_id = %auth.user_id,
        group_id = %group_uuid,
        "Group locked"
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("Group '{}' has been locked", group_name)
    })))
}

/// Unlock a group
/// POST /api/groups/{company_id}/{group_id}/unlock
pub async fn unlock_group(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, group_id)): Path<(String, String)>,
    Json(input): Json<UnlockGroupInput>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let group_uuid = Uuid::parse_str(&group_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if group is inside company folder - only admins can unlock
    if is_group_in_company_folder(&state.store, tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        if auth.role != "SuperAdmin" && auth.role != "Admin" {
            tracing::warn!(
                "Security: Non-admin user {} attempted to unlock group in company folder",
                auth.user_id
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Get current group status including lock details
    let group = state
        .store
        .groups()
        .get(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let group = group.ok_or(StatusCode::NOT_FOUND)?;
    let group_name = group.name;
    let is_locked = group.is_locked.unwrap_or(false);
    let locked_by = group.locked_by;
    let password_hash = group.lock_password_hash;
    let required_role = group.lock_requires_role;
    let owner_id = group.created_by;

    if !is_locked {
        return Ok(Json(json!({ "message": "Group is not locked" })));
    }

    // Role hierarchy for permission checking
    let role_hierarchy = |role: &str| -> i32 {
        match role {
            "SuperAdmin" => 100,
            "Admin" => 80,
            "Manager" => 60,
            "Employee" => 40,
            _ => 20, // Custom roles
        }
    };

    // Check if user can unlock based on role requirement
    let mut can_unlock = false;

    // Group owner can always unlock
    if owner_id == auth.user_id {
        can_unlock = true;
    }
    // User who locked it can always unlock
    else if locked_by == Some(auth.user_id) {
        can_unlock = true;
    }
    // SuperAdmin can always unlock
    else if auth.role == "SuperAdmin" {
        can_unlock = true;
    }
    // Check role requirement
    else if let Some(ref req_role) = required_role {
        let user_level = role_hierarchy(&auth.role);
        let required_level = role_hierarchy(req_role);

        if user_level >= required_level {
            can_unlock = true;
        } else {
            // Check if user has custom role with files.unlock permission
            let custom_role_has_perm = state
                .store
                .groups()
                .custom_permission(tenant_id, &auth.role, "files.unlock")
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            if custom_role_has_perm {
                can_unlock = true;
            }
        }
    } else {
        // No role requirement - Admin or above can unlock
        if ["SuperAdmin", "Admin"].contains(&auth.role.as_str()) {
            can_unlock = true;
        }
    }

    if !can_unlock {
        return Ok(Json(json!({
            "error": "Insufficient permissions",
            "required_role": required_role,
            "has_password": password_hash.is_some()
        })));
    }

    // Check password if required
    if let Some(ref pwd_hash) = password_hash {
        let provided_password = input.password.as_deref().unwrap_or("");

        use argon2::{Argon2, PasswordHash, PasswordVerifier};
        let parsed_hash =
            PasswordHash::new(pwd_hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if Argon2::default()
            .verify_password(provided_password.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Ok(Json(json!({
                "error": "Invalid password",
                "has_password": true
            })));
        }
    }

    // Unlock the group
    state
        .store
        .groups()
        .unlock(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .system()
        .audit_resource(
            tenant_id,
            auth.user_id,
            "group_unlocked",
            "file_group",
            group_uuid,
            json!({"name":group_name}),
            auth.ip_address.as_deref(),
        )
        .await;

    tracing::info!(
        user_id = %auth.user_id,
        group_id = %group_uuid,
        "Group unlocked"
    );

    Ok(Json(json!({
        "success": true,
        "message": format!("Group '{}' has been unlocked", group_name)
    })))
}

/// Check if user can access a locked group
pub fn can_access_locked_group(
    user_id: Uuid,
    user_role: &str,
    is_locked: bool,
    locked_by: Option<Uuid>,
    owner_id: Uuid,
    lock_requires_role: Option<&str>,
) -> bool {
    if !is_locked {
        return true;
    }

    // Owner can always access
    if owner_id == user_id {
        return true;
    }

    // User who locked it can always access
    if locked_by == Some(user_id) {
        return true;
    }

    // SuperAdmin can always access
    if user_role == "SuperAdmin" {
        return true;
    }

    // Check role requirement
    if let Some(req_role) = lock_requires_role {
        let role_level = |role: &str| -> i32 {
            match role {
                "SuperAdmin" => 100,
                "Admin" => 80,
                "Manager" => 60,
                "Employee" => 40,
                _ => 20,
            }
        };

        let user_level = role_level(user_role);
        let required_level = role_level(req_role);

        return user_level >= required_level;
    }

    false
}

/// Toggle star status for a group
/// POST /api/groups/{company_id}/{group_id}/star
pub async fn toggle_group_star(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, group_id)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let group_uuid = Uuid::parse_str(&group_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify group exists
    let group_exists = state
        .store
        .groups()
        .get(tenant_id, group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if group_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Get current user prefs from S3
    let user_prefs_key = format!(".clovalink/{}/user_prefs/{}.json", tenant_id, auth.user_id);
    let mut prefs: Value = match state.storage.download(&user_prefs_key).await {
        Ok(data) => serde_json::from_slice(&data).unwrap_or(json!({ "starred": [] })),
        Err(_) => json!({ "starred": [] }),
    };

    let group_id_str = group_uuid.to_string();

    // Modify starred array in place
    {
        let starred_files = prefs["starred"].as_array_mut().unwrap();
        if let Some(pos) = starred_files
            .iter()
            .position(|x| x.as_str() == Some(&group_id_str))
        {
            starred_files.remove(pos); // Unstar
        } else {
            starred_files.push(json!(group_id_str)); // Star
        }
    }

    let data = serde_json::to_vec(&prefs).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .storage
        .upload(&user_prefs_key, data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "starred": prefs["starred"] })))
}
