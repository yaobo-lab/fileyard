//! File Comments API Handlers
//!
//! Provides CRUD operations for file comments with proper access control.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::handlers::can_access_file;
use crate::AppState;
use clovalink_auth::middleware::AuthUser;

// ==================== Models ====================

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct FileComment {
    pub id: Uuid,
    pub file_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub is_edited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CommentWithUser {
    pub id: Uuid,
    pub file_id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub user_avatar: Option<String>,
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub is_edited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub replies: Vec<CommentWithUser>,
    pub can_edit: bool,
    pub can_delete: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateCommentInput {
    pub content: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UpdateCommentInput {
    pub content: String,
}

// ==================== Handlers ====================

/// List all comments for a file
/// GET /api/files/{company_id}/{file_id}/comments
pub async fn list_comments(
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

    // Check if user can access this file
    if !can_access_file(
        &state.pool,
        file_uuid,
        tenant_id,
        auth.user_id,
        &auth.role,
        "read",
    )
    .await?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    // Fetch all comments with user info
    let comments = state
        .store
        .comments()
        .list(tenant_id, file_uuid)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch comments: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Build threaded structure
    let mut top_level: Vec<CommentWithUser> = Vec::new();
    let mut replies_map: std::collections::HashMap<Uuid, Vec<CommentWithUser>> =
        std::collections::HashMap::new();

    for row in comments {
        let c = row.comment;
        let id = c.id;
        let file_id = c.file_id;
        let user_id = c.user_id;
        let user_name = row.user_name;
        let user_avatar = row.user_avatar;
        let content = c.content;
        let parent_id = c.parent_id;
        let is_edited = c.is_edited;
        let created_at = c.created_at.with_timezone(&Utc);
        let updated_at = c.updated_at.with_timezone(&Utc);
        let comment = CommentWithUser {
            id,
            file_id,
            user_id,
            user_name,
            user_avatar,
            content,
            parent_id,
            is_edited,
            created_at,
            updated_at,
            replies: Vec::new(),
            can_edit: user_id == auth.user_id,
            can_delete: user_id == auth.user_id
                || auth.role == "Admin"
                || auth.role == "SuperAdmin",
        };

        if let Some(pid) = parent_id {
            replies_map.entry(pid).or_default().push(comment);
        } else {
            top_level.push(comment);
        }
    }

    // Attach replies to parent comments
    for comment in &mut top_level {
        if let Some(replies) = replies_map.remove(&comment.id) {
            comment.replies = replies;
        }
    }

    Ok(Json(json!({
        "comments": top_level,
        "total": top_level.len()
    })))
}

/// Add a comment to a file
/// POST /api/files/{company_id}/{file_id}/comments
pub async fn create_comment(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, file_id)): Path<(String, String)>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let file_uuid = Uuid::parse_str(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse input
    let content = input["content"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let parent_id = input["parent_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());

    if content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if user can access this file
    if !can_access_file(
        &state.pool,
        file_uuid,
        tenant_id,
        auth.user_id,
        &auth.role,
        "read",
    )
    .await?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify parent comment exists if provided
    if let Some(pid) = parent_id {
        let parent_exists = state
            .store
            .comments()
            .exists(pid, file_uuid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if !parent_exists {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Get file info for notification
    let file_info = state
        .store
        .comments()
        .file_info(file_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (file_name, file_owner_id) = file_info.ok_or(StatusCode::NOT_FOUND)?;
    let file_owner_id = file_owner_id.ok_or(StatusCode::NOT_FOUND)?;

    // Create the comment
    let comment_id = state
        .store
        .comments()
        .create(tenant_id, file_uuid, auth.user_id, content, parent_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create comment: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Audit log
    let _ = state
        .store
        .system()
        .audit(
            tenant_id,
            auth.user_id,
            "comment_added",
            "file",
            json!({
                "comment_id": comment_id,
                "file_name": file_name,
                "is_reply": parent_id.is_some()
            }),
            auth.ip_address.as_deref(),
        )
        .await;

    // Send Discord notification to file owner (if not commenting on own file)
    if file_owner_id != auth.user_id {
        let pool_clone = state.pool.clone();
        let commenter_name = auth
            .email
            .split('@')
            .next()
            .unwrap_or("Someone")
            .to_string();
        let content_preview = if content.len() > 100 {
            format!("{}...", &content[..100])
        } else {
            content.to_string()
        };

        tokio::spawn(async move {
            crate::discord::notify_comment(
                &pool_clone,
                tenant_id,
                file_owner_id,
                &file_name,
                &commenter_name,
                &content_preview,
            )
            .await;
        });
    }

    // Get user info for response
    let user_name = state
        .store
        .comments()
        .user_name(auth.user_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Unknown".into());

    Ok(Json(json!({
        "id": comment_id,
        "file_id": file_uuid,
        "user_id": auth.user_id,
        "user_name": user_name,
        "content": content,
        "parent_id": parent_id,
        "is_edited": false,
        "created_at": Utc::now(),
        "can_edit": true,
        "can_delete": true
    })))
}

/// Update a comment
/// PUT /api/files/{company_id}/{file_id}/comments/{comment_id}
pub async fn update_comment(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, file_id, comment_id)): Path<(String, String, String)>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let file_uuid = Uuid::parse_str(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let comment_uuid = Uuid::parse_str(&comment_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let content = input["content"].as_str().ok_or(StatusCode::BAD_REQUEST)?;

    if content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if comment exists and user owns it
    let comment = state
        .store
        .comments()
        .owner(tenant_id, file_uuid, comment_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let owner_id = comment.ok_or(StatusCode::NOT_FOUND)?;

    // Only comment owner can edit
    if owner_id != auth.user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Update the comment
    state
        .store
        .comments()
        .update(comment_uuid, content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "success": true,
        "id": comment_uuid,
        "content": content,
        "is_edited": true
    })))
}

/// Delete a comment
/// DELETE /api/files/{company_id}/{file_id}/comments/{comment_id}
pub async fn delete_comment(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, file_id, comment_id)): Path<(String, String, String)>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = Uuid::parse_str(&company_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let file_uuid = Uuid::parse_str(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let comment_uuid = Uuid::parse_str(&comment_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify tenant access
    if auth.role != "SuperAdmin" && auth.tenant_id != tenant_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if comment exists and get owner
    let comment = state
        .store
        .comments()
        .owner(tenant_id, file_uuid, comment_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let owner_id = comment.ok_or(StatusCode::NOT_FOUND)?;

    // Only comment owner or admins can delete
    if owner_id != auth.user_id && auth.role != "Admin" && auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Delete the comment (cascades to replies)
    state
        .store
        .comments()
        .delete(comment_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "success": true })))
}

/// Get comment count for a file
/// GET /api/files/{company_id}/{file_id}/comments/count
pub async fn get_comment_count(
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

    let count = state
        .store
        .comments()
        .count(tenant_id, file_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "count": count })))
}
