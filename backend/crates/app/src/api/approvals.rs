use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::{DateTime, Utc};
use crate::auth::{require_admin, AuthUser};
use app_core::models::Tenant;
use app_core::notification_service;
use app_entity::{
    entities::approval_policies,
    repositories::NewAuditLog,
    DataStore,
};
use sea_orm::ActiveValue::Set;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

// ==================== Models ====================

pub type ApprovalPolicy = approval_policies::Model;

#[derive(Debug, Clone, Serialize)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub file_id: Uuid,
    pub policy_id: Option<Uuid>,
    pub requested_by: Uuid,
    pub status: String,
    pub step: i32,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalListQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub department_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePolicyInput {
    pub name: String,
    pub scope: String,
    pub scope_value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyInput {
    pub name: Option<String>,
    pub scope: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub scope_value: Option<Option<String>>, // None = not sent, Some(None) = explicitly null, Some(Some(v)) = value
    pub is_active: Option<bool>,
}

fn deserialize_nullable_string<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

#[derive(Debug, Deserialize)]
pub struct RejectInput {
    pub reason: String,
}

// ==================== Policy Matching ====================

/// Context for matching approval policies against an uploaded file
pub struct FileUploadContext {
    pub department_id: Option<Uuid>,
    pub is_company_folder: bool,
    pub file_name: String,
    pub file_size: i64,
    pub visibility: String,
    pub uploader_role: String,
}

/// Find a matching active approval policy for a file upload.
/// Priority: specific scopes (department, file_type, file_size, role, private, company_folder) > all
pub async fn find_matching_policy(
    store: &DataStore,
    tenant_id: Uuid,
    ctx: &FileUploadContext,
) -> Option<ApprovalPolicy> {
    // Fetch all active policies for this tenant in one query
    let policies = store
        .approvals()
        .list_active_policies(tenant_id)
        .await
        .unwrap_or_default();

    // Check specific scopes first, then catch-all
    for policy in &policies {
        match policy.scope.as_str() {
            "department" => {
                if let (Some(dept_id), Some(ref sv)) = (ctx.department_id, &policy.scope_value) {
                    if dept_id.to_string() == *sv {
                        return Some(policy.clone());
                    }
                }
            }
            "company_folder" => {
                if ctx.is_company_folder {
                    return Some(policy.clone());
                }
            }
            "file_type" => {
                if let Some(ref sv) = policy.scope_value {
                    let file_ext = ctx
                        .file_name
                        .rsplit('.')
                        .next()
                        .unwrap_or("")
                        .to_lowercase();
                    if sv
                        .split(',')
                        .any(|ext| ext.trim().eq_ignore_ascii_case(&file_ext))
                    {
                        return Some(policy.clone());
                    }
                }
            }
            "file_size" => {
                if let Some(ref sv) = policy.scope_value {
                    if let Ok(threshold) = sv.parse::<i64>() {
                        if ctx.file_size >= threshold {
                            return Some(policy.clone());
                        }
                    }
                }
            }
            "role" => {
                if let Some(ref sv) = policy.scope_value {
                    if ctx.uploader_role == *sv {
                        return Some(policy.clone());
                    }
                }
            }
            "private_files" => {
                if ctx.visibility == "private" {
                    return Some(policy.clone());
                }
            }
            _ => {} // Skip unknown scopes, check "all" last
        }
    }

    // Check catch-all last
    for policy in &policies {
        if policy.scope == "all" {
            return Some(policy.clone());
        }
    }

    None
}

/// Simple helper to find any active approval policy (e.g. for resubmit)
pub async fn find_matching_policy_simple(
    store: &DataStore,
    tenant_id: Uuid,
    _department_id: Option<Uuid>,
    _is_company_folder: bool,
) -> Option<ApprovalPolicy> {
    store
        .approvals()
        .list_active_policies(tenant_id)
        .await
        .ok()
        .and_then(|list| list.into_iter().next())
}

// ==================== Approval Handlers ====================

/// GET /api/approvals/:company_id/pending
/// List files pending approval (requires approvals.view)
pub async fn list_pending(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<Uuid>,
    Query(query): Query<ApprovalListQuery>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    // Manager+ have approvals.view by default
    if !matches!(auth.role.as_str(), "Manager" | "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(50).min(100).max(1) as u64;
    let offset = (query.page.unwrap_or(0).max(0) * (limit as i64)) as u64;

    let rows = state
        .store
        .approvals()
        .list_pending(company_id, limit, offset, query.department_id)
        .await
        .map_err(|e| {
            log::error!("Failed to list pending approvals: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({ "approvals": rows })))
}

/// GET /api/approvals/:company_id/history
/// List completed approval requests (requires approvals.view)
pub async fn list_history(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<Uuid>,
    Query(query): Query<ApprovalListQuery>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    if !matches!(auth.role.as_str(), "Manager" | "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(50).min(100).max(1) as u64;
    let offset = (query.page.unwrap_or(0).max(0) * (limit as i64)) as u64;

    let history = state
        .store
        .approvals()
        .list_history(company_id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "history": history })))
}

/// GET /api/approvals/:company_id/my-pending
/// List current user's files that are pending approval
pub async fn list_my_pending(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let items = state
        .store
        .approvals()
        .list_my_pending(company_id, auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "items": items })))
}

/// GET /api/approvals/:company_id/stats
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    if !matches!(auth.role.as_str(), "Manager" | "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    let (pending, approved, rejected) = state
        .store
        .approvals()
        .get_stats(company_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "pending": pending,
        "approved": approved,
        "rejected": rejected,
    })))
}

/// POST /api/approvals/:company_id/:request_id/approve
pub async fn approve_file(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, request_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    if !matches!(auth.role.as_str(), "Manager" | "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    // Atomic update — only succeeds if status is currently 'pending'
    let result = state
        .store
        .approvals()
        .approve_request(company_id, request_id, auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (file_id, requested_by) = result.ok_or(StatusCode::NOT_FOUND)?;

    // Update file status
    state
        .store
        .approvals()
        .update_file_approval_status(file_id, "approved")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: company_id,
            user_id: Some(auth.user_id),
            action: "file_approved".to_string(),
            resource_type: "file".to_string(),
            resource_id: Some(file_id),
            metadata: Some(json!({"approval_request_id": request_id})),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    // Notify uploader
    if let Ok(Some(tenant_model)) = state.store.tenants().by_id(company_id).await {
        let tenant: Tenant = tenant_model.into();
        if let Ok(Some((file_name, email, role))) = state
            .store
            .approvals()
            .get_file_and_uploader(file_id, requested_by)
            .await
        {
            let _ = notification_service::create_notification(
                &state.store,
                &tenant,
                requested_by,
                &role,
                notification_service::NotificationType::ApprovalDecision,
                "File Approved",
                &format!("Your file \"{}\" has been approved and is now accessible.", file_name),
                Some(json!({"file_id": file_id, "status": "approved"})),
                Some(&email),
            )
            .await;
        }
    }

    Ok(Json(json!({ "status": "approved", "file_id": file_id })))
}

/// POST /api/approvals/:company_id/:request_id/reject
pub async fn reject_file(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, request_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<RejectInput>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    if !matches!(auth.role.as_str(), "Manager" | "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    if input.reason.trim().is_empty() || input.reason.len() > 2000 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Atomic update
    let result = state
        .store
        .approvals()
        .reject_request(company_id, request_id, auth.user_id, input.reason.trim())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (file_id, requested_by) = result.ok_or(StatusCode::NOT_FOUND)?;

    // Update file status
    state
        .store
        .approvals()
        .update_file_approval_status(file_id, "rejected")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: company_id,
            user_id: Some(auth.user_id),
            action: "file_rejected".to_string(),
            resource_type: "file".to_string(),
            resource_id: Some(file_id),
            metadata: Some(json!({"approval_request_id": request_id, "reason": input.reason.trim()})),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    // Notify uploader
    if let Ok(Some(tenant_model)) = state.store.tenants().by_id(company_id).await {
        let tenant: Tenant = tenant_model.into();
        if let Ok(Some((file_name, email, role))) = state
            .store
            .approvals()
            .get_file_and_uploader(file_id, requested_by)
            .await
        {
            let _ = notification_service::create_notification(
                &state.store,
                &tenant,
                requested_by,
                &role,
                notification_service::NotificationType::ApprovalDecision,
                "File Rejected",
                &format!("Your file \"{}\" was rejected. Reason: {}", file_name, input.reason.trim()),
                Some(json!({"file_id": file_id, "status": "rejected", "reason": input.reason.trim()})),
                Some(&email),
            )
            .await;
        }
    }

    Ok(Json(json!({ "status": "rejected", "file_id": file_id })))
}

/// POST /api/approvals/:company_id/:file_id/send
/// Manually send an existing approved file for approval review
pub async fn send_for_approval(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify the file exists and belongs to this tenant
    let file = state
        .store
        .approvals()
        .get_file_info_for_approval(company_id, file_id, true)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (owner_id, approval_status, department_id, is_company_folder) =
        file.ok_or(StatusCode::NOT_FOUND)?;

    // Only the file owner or an admin can send for approval
    if owner_id != auth.user_id && !matches!(auth.role.as_str(), "Admin" | "SuperAdmin") {
        return Err(StatusCode::FORBIDDEN);
    }

    // File must currently be approved (not already pending or rejected)
    if approval_status != "approved" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Find matching policy
    let policy =
        find_matching_policy_simple(&state.store, company_id, department_id, is_company_folder)
            .await;
    let policy_id = policy.map(|p| p.id);

    // Set file to pending
    state
        .store
        .approvals()
        .update_file_approval_status(file_id, "pending")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create approval request
    let new_request_id = state
        .store
        .approvals()
        .create_request(company_id, file_id, policy_id, auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: company_id,
            user_id: Some(auth.user_id),
            action: "file_sent_for_approval".to_string(),
            resource_type: "file".to_string(),
            resource_id: Some(file_id),
            metadata: Some(json!({"approval_request_id": new_request_id})),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    // Notify approvers
    if let Ok(Some(tenant_model)) = state.store.tenants().by_id(company_id).await {
        let tenant: Tenant = tenant_model.into();
        if let Ok(Some(file_name)) = state.store.approvals().get_file_name(file_id).await {
            let _ = notification_service::notify_all_admins(
                &state.store,
                &tenant,
                notification_service::NotificationType::ApprovalRequired,
                "File Sent for Approval",
                &format!("\"{}\" has been manually sent for approval.", file_name),
                Some(
                    json!({"file_id": file_id, "file_name": &file_name, "sender_id": auth.user_id}),
                ),
            )
            .await;
        }
    }

    Ok(Json(
        json!({ "status": "pending", "approval_request_id": new_request_id }),
    ))
}

/// POST /api/approvals/:company_id/:file_id/resubmit
/// Owner resubmits a rejected file for approval
pub async fn resubmit(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, file_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Verify the file exists, belongs to this tenant, and the user owns it
    let file = state
        .store
        .approvals()
        .get_file_info_for_approval(company_id, file_id, false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (owner_id, approval_status, department_id, is_company_folder) =
        file.ok_or(StatusCode::NOT_FOUND)?;

    if owner_id != auth.user_id {
        return Err(StatusCode::FORBIDDEN);
    }
    if approval_status != "rejected" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Find matching policy
    let policy =
        find_matching_policy_simple(&state.store, company_id, department_id, is_company_folder)
            .await;
    let policy_id = policy.map(|p| p.id);

    // Reset file status to pending
    state
        .store
        .approvals()
        .update_file_approval_status(file_id, "pending")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create new approval request
    let new_request_id = state
        .store
        .approvals()
        .create_request(company_id, file_id, policy_id, auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: company_id,
            user_id: Some(auth.user_id),
            action: "file_resubmitted".to_string(),
            resource_type: "file".to_string(),
            resource_id: Some(file_id),
            metadata: Some(json!({"approval_request_id": new_request_id})),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    Ok(Json(
        json!({ "status": "pending", "approval_request_id": new_request_id }),
    ))
}

// ==================== Policy CRUD ====================

/// GET /api/approvals/:company_id/policies
pub async fn list_policies(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    require_admin(&auth)?;

    let policies = state
        .store
        .approvals()
        .list_policies(company_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "policies": policies })))
}

/// POST /api/approvals/:company_id/policies
pub async fn create_policy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(company_id): Path<Uuid>,
    Json(input): Json<CreatePolicyInput>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    require_admin(&auth)?;

    // Validate scope
    if !matches!(
        input.scope.as_str(),
        "all"
            | "department"
            | "company_folder"
            | "file_type"
            | "file_size"
            | "role"
            | "private_files"
    ) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if matches!(
        input.scope.as_str(),
        "department" | "file_type" | "file_size" | "role"
    ) && input.scope_value.is_none()
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if input.name.trim().is_empty() || input.name.len() > 255 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let policy = state
        .store
        .approvals()
        .create_policy(
            company_id,
            input.name.trim().to_string(),
            input.scope,
            input.scope_value,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to create approval policy: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: company_id,
            user_id: Some(auth.user_id),
            action: "approval_policy_created".to_string(),
            resource_type: "approval_policy".to_string(),
            resource_id: Some(policy.id),
            metadata: Some(json!({"name": &policy.name, "scope": &policy.scope})),
            ip_address: auth.ip_address.clone(),
        })
        .await;

    Ok(Json(json!({ "policy": policy })))
}

/// PUT /api/approvals/:company_id/policies/:id
pub async fn update_policy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, policy_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdatePolicyInput>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    require_admin(&auth)?;

    if let Some(ref scope) = input.scope {
        if !matches!(
            scope.as_str(),
            "all"
                | "department"
                | "company_folder"
                | "file_type"
                | "file_size"
                | "role"
                | "private_files"
        ) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let policy = state
        .store
        .approvals()
        .by_policy_id(company_id, policy_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: approval_policies::ActiveModel = policy.into();
    let mut has_updates = false;

    if let Some(name) = input.name {
        active.name = Set(name);
        has_updates = true;
    }
    if let Some(scope) = input.scope {
        active.scope = Set(scope);
        has_updates = true;
    }
    if let Some(scope_value) = input.scope_value {
        active.scope_value = Set(scope_value);
        has_updates = true;
    }
    if let Some(is_active) = input.is_active {
        active.is_active = Set(is_active);
        has_updates = true;
    }

    if !has_updates {
        return Err(StatusCode::BAD_REQUEST);
    }

    active.updated_at = Set(chrono::Utc::now().into());

    let policy = state
        .store
        .approvals()
        .update_policy(active)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "policy": policy })))
}

/// DELETE /api/approvals/:company_id/policies/:id
pub async fn delete_policy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((company_id, policy_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, StatusCode> {
    if auth.tenant_id != company_id {
        return Err(StatusCode::FORBIDDEN);
    }
    require_admin(&auth)?;

    let deleted = state
        .store
        .approvals()
        .delete_policy(company_id, policy_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    // Audit log
    let _ = state
        .store
        .audit()
        .log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id: company_id,
            user_id: Some(auth.user_id),
            action: "approval_policy_deleted".to_string(),
            resource_type: "approval_policy".to_string(),
            resource_id: Some(policy_id),
            metadata: None,
            ip_address: auth.ip_address.clone(),
        })
        .await;

    Ok(Json(json!({ "deleted": true })))
}
