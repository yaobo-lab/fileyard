//! User-Specific File Sharing Handlers
//!
//! Provides endpoints for:
//! - Listing shareable users (respecting tenant/department boundaries)
//! - Listing files shared with the current user
//! - Filtering share recipients by access control rules

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::auth::middleware::AuthUser;

// ==================== Models ====================

#[allow(dead_code)]
pub type ShareableUser = app_entity::ShareableUserRow;
#[allow(dead_code)]
pub type SharedFile = app_entity::SharedFileRow;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ShareableUsersQuery {
    pub search: Option<String>,
    pub department_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SharedWithMeQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

// ==================== Access Control Rules ====================

/// Gets the list of department IDs a user can share with.
/// - Admin/SuperAdmin: all departments in tenant
/// - Others: their own department + any explicitly accessible departments
async fn get_accessible_department_ids(
    store: &app_entity::DataStore,
    user_id: Uuid,
    tenant_id: Uuid,
    role: &str,
) -> Result<Vec<Uuid>, StatusCode> {
    if role == "Admin" || role == "SuperAdmin" {
        let depts = store
            .departments()
            .list(tenant_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(depts.into_iter().map(|d| d.id).collect());
    }

    let user = store
        .users()
        .user(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut dept_ids = Vec::new();
    if let Some(d) = user.and_then(|u| u.department_id) {
        dept_ids.push(d);
    }
    Ok(dept_ids)
}

// ==================== Handlers ====================

/// List users available for sharing (respects tenant/department boundaries)
/// GET /api/users/{company_id}/shareable
pub async fn list_shareable_users(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<String>,
    Query(query): Query<ShareableUsersQuery>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let accessible_depts = if auth.role == "Admin" || auth.role == "SuperAdmin" {
        None
    } else {
        Some(get_accessible_department_ids(&state.store, auth.user_id, tenant_id, &auth.role).await?)
    };

    let users = state
        .store
        .shares()
        .list_shareable_users(
            tenant_id,
            auth.user_id,
            accessible_depts.as_deref(),
            query.search.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch shareable users: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "total": users.len(),
        "users": users
    })))
}

/// List files shared with the current user
/// GET /api/shared-with-me
pub async fn list_shared_with_me(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<SharedWithMeQuery>,
) -> Result<Json<Value>, StatusCode> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let (shared_files, total) = state
        .store
        .shares()
        .list_shared_with_me(auth.user_id, auth.tenant_id, per_page as u64, offset as u64)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch shared files: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(json!({
        "files": shared_files,
        "total": total,
        "page": page,
        "per_page": per_page,
        "total_pages": total_pages
    })))
}

// ==================== Copy to My Files ====================

#[derive(Debug, Deserialize)]
pub struct CopyToMyFilesInput {
    pub file_id: Uuid,
    pub share_token: String,
}

/// Copy a shared file to the user's private files
/// POST /api/shared-with-me/copy
pub async fn copy_to_my_files(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<CopyToMyFilesInput>,
) -> Result<Json<Value>, StatusCode> {
    // Verify the share exists and is valid for this user
    let share = state
        .store
        .shares()
        .get_user_share(&input.share_token, auth.user_id, auth.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch share: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if share has expired
    if let Some(exp) = share.expires_at {
        if exp < chrono::Utc::now().fixed_offset() {
            return Err(StatusCode::GONE);
        }
    }

    // Verify file_id matches
    if share.file_id != input.file_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get original file metadata
    let original = state
        .store
        .files()
        .by_tenant_id(share.tenant_id, share.file_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch file metadata: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .filter(|f| !f.is_deleted)
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_name = original.name.clone();

    // Get user's department for the new file
    let user_dept = state
        .store
        .users()
        .user(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .and_then(|u| u.department_id);

    // Generate new storage path for the copy
    let new_file_id = Uuid::new_v4();
    let new_ulid = ulid::Ulid::new().to_string();
    let extension = std::path::Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    let new_storage_path = format!(
        "{}/{}/{}{}",
        share.tenant_id, auth.user_id, new_file_id, extension
    );

    // Download the original file
    let file_data = state.storage.download(&original.storage_path).await.map_err(|e| {
        tracing::error!("Failed to download original file: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Upload to the new location
    state
        .storage
        .upload(&new_storage_path, file_data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload copied file: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Create new file metadata entry
    let new_file = state
        .store
        .files()
        .create(
            new_file_id,
            auth.tenant_id,
            user_dept,
            file_name.clone(),
            new_storage_path,
            original.size_bytes,
            original.content_type,
            auth.user_id,
            None,
            "private".to_string(),
            new_ulid,
            original.content_hash,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create file metadata: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Log the action
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            "file_copied_from_share",
            "file",
            Some(new_file_id),
            Some(serde_json::json!({
                "original_file_id": share.file_id,
                "file_name": file_name,
                "share_token": input.share_token,
            })),
            auth.ip_address.clone(),
        )
        .await;

    tracing::info!(
        user_id = %auth.user_id,
        original_file = %share.file_id,
        new_file = %new_file_id,
        "File copied from share to private files"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "file": {
            "id": new_file.id,
            "name": new_file.name,
            "created_at": new_file.created_at,
        },
        "message": format!("\"{}\" has been saved to your files", file_name)
    })))
}

/// Validate that a user can share with another user
/// Returns true if sharing is allowed
pub async fn can_share_with_user(
    store: &app_entity::DataStore,
    sharer_id: Uuid,
    sharer_tenant_id: Uuid,
    sharer_role: &str,
    recipient_id: Uuid,
) -> Result<bool, StatusCode> {
    // Get recipient's tenant and department
    let recipient = store
        .users()
        .user(recipient_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let recipient = match recipient {
        Some(r) if r.status == "active" => r,
        _ => return Ok(false),
    };

    // CRITICAL: Must be same tenant
    if recipient.tenant_id != sharer_tenant_id {
        tracing::warn!(
            sharer_id = %sharer_id,
            recipient_id = %recipient_id,
            "Cross-tenant share attempt blocked"
        );
        return Ok(false);
    }

    // Admins can share with anyone in their tenant
    if sharer_role == "Admin" || sharer_role == "SuperAdmin" {
        return Ok(true);
    }

    // For regular users, check department access
    let accessible_depts =
        get_accessible_department_ids(store, sharer_id, sharer_tenant_id, sharer_role).await?;

    // Check if recipient is in an accessible department
    if let Some(dept_id) = recipient.department_id {
        Ok(accessible_depts.contains(&dept_id))
    } else {
        // User has no department - only allow if sharer also has no department
        let sharer = store
            .users()
            .user(sharer_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(sharer.and_then(|u| u.department_id).is_none())
    }
}
