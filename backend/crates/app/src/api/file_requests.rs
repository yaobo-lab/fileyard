use crate::compliance::{
    check_compliance_action, get_tenant_compliance_mode, ComplianceAction, ComplianceRestrictions,
};
use crate::AppState;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::{Duration, Utc};
use crate::auth::AuthUser;
use app_core::models::{CreateFileRequestInput, Tenant};
use app_core::notification_service;
use app_core::security_service;
use app_entity::ListFileRequestsFilter;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FileRequestFilters {
    pub status: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub visibility: Option<String>, // 'department' (default) or 'private'
    pub department_id: Option<String>, // Optional department filter (for admins)
}

/// Create a new file request
/// POST /api/file-requests
pub async fn create_file_request(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    body: String,
) -> Result<Json<Value>, StatusCode> {
    log::debug!("Received file request body: {}", body);

    let input: CreateFileRequestInput = serde_json::from_str(&body).map_err(|e| {
        log::error!("Failed to parse file request JSON: {:?}", e);
        log::error!("Raw body was: {}", body);
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    // Check compliance restrictions for public sharing
    let compliance_mode = get_tenant_compliance_mode(&state.store, auth.tenant_id)
        .await
        .unwrap_or_else(|_| "Standard".to_string());
    let restrictions = ComplianceRestrictions::for_mode(&compliance_mode);

    // Block public sharing if compliance mode restricts it
    if restrictions.public_sharing_blocked {
        return Err(StatusCode::FORBIDDEN);
    }

    // Also check using the compliance action checker for more detailed error handling
    if let Err(violation) =
        check_compliance_action(&state.store, auth.tenant_id, ComplianceAction::PublicShare).await
    {
        log::warn!("Compliance violation: {:?}", violation);
        return Err(violation.to_status_code());
    }

    let token = nanoid::nanoid!(16);
    let expires_at = Utc::now() + Duration::days(input.expires_in_days);

    // Validate and set visibility (default to 'department')
    let visibility = input.visibility.as_deref().unwrap_or("department");
    let visibility = if visibility == "private" {
        "private"
    } else {
        "department"
    };

    let request = state
        .store
        .file_requests()
        .create(
            auth.tenant_id,
            input.department_id,
            input.name.clone(),
            input.destination_path.clone(),
            token,
            auth.user_id,
            expires_at,
            input.max_uploads,
            visibility.to_string(),
        )
        .await
        .map_err(|e| {
            log::error!("Failed to create file request: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Log creation of file request
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            "file_request_created",
            "file_request",
            Some(request.id),
            Some(json!({
                "name": input.name,
                "destination_path": input.destination_path,
                "expires_at": expires_at,
                "visibility": visibility,
            })),
            auth.ip_address.clone(),
        )
        .await;

    let base_url = types::config::get_config().web.base_url.clone();

    Ok(Json(json!({
        "id": request.id,
        "name": request.name,
        "destination": request.destination_path,
        "token": request.token,
        "link": format!("{}/upload/{}", base_url, request.token),
        "expires_at": request.expires_at,
        "status": request.status,
        "upload_count": request.upload_count,
        "max_uploads": request.max_uploads,
        "visibility": request.visibility,
        "created_at": request.created_at,
    })))
}

/// List file requests with optional filters
/// GET /api/file-requests
pub async fn list_file_requests(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(filters): Query<FileRequestFilters>,
) -> Result<Json<Value>, StatusCode> {
    let limit = filters.limit.unwrap_or(50).clamp(1, 100) as u64;
    let offset = filters.offset.unwrap_or(0).max(0) as u64;

    // Get user's department and role from database
    let user = state
        .store
        .users()
        .user(auth.user_id)
        .await
        .unwrap_or(None);

    let user_department_id = user.as_ref().and_then(|u| u.department_id);
    let role = user
        .as_ref()
        .map(|u| u.role.clone())
        .unwrap_or_else(|| auth.role.clone());

    let is_admin = role == "SuperAdmin" || role == "Admin";
    let view_mode = filters.visibility.as_deref().unwrap_or("department").to_string();

    let department_id_filter = if is_admin {
        filters.department_id.as_deref().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Uuid::parse_str(s).ok()
            }
        })
    } else {
        None
    };

    let created_after = filters.created_after.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s).ok()
    });
    let created_before = filters.created_before.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s).ok()
    });

    let requests = state
        .store
        .file_requests()
        .list_filtered(ListFileRequestsFilter {
            tenant_id: auth.tenant_id,
            visibility: view_mode,
            user_id: auth.user_id,
            is_admin,
            user_department_id,
            department_id_filter,
            status: filters.status,
            created_after,
            created_before,
            limit,
            offset,
        })
        .await
        .map_err(|e| {
            log::error!("Failed to list file requests: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let base_url = types::config::get_config().web.base_url.clone();

    let results: Vec<Value> = requests
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "name": r.name,
                "destination": r.destination_path,
                "token": r.token,
                "link": format!("{}/upload/{}", base_url, r.token),
                "expires_at": r.expires_at,
                "status": r.status,
                "upload_count": r.upload_count,
                "max_uploads": r.max_uploads,
                "visibility": r.visibility,
                "created_at": r.created_at,
            })
        })
        .collect();

    Ok(Json(json!(results)))
}

/// Get single file request
/// GET /api/file-requests/:id
pub async fn get_file_request(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let request = state
        .store
        .file_requests()
        .by_tenant_and_id(auth.tenant_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Security check: enforce visibility rules
    let visibility = &request.visibility;

    if visibility == "private" {
        // Private requests: only the creator can access
        if request.created_by != auth.user_id {
            log::warn!(
                "User {} attempted to access private file request {} owned by {}",
                auth.user_id,
                request.id,
                request.created_by
            );
            return Err(StatusCode::FORBIDDEN);
        }
    } else {
        // Department visibility: check department membership or admin role
        if auth.role != "SuperAdmin" && auth.role != "Admin" {
            // Get user's department
            let user = state
                .store
                .users()
                .user(auth.user_id)
                .await
                .unwrap_or(None);

            let user_department_id = user.and_then(|u| u.department_id);

            // If request has a department, user must be in that department
            if let Some(req_dept_id) = request.department_id {
                if user_department_id != Some(req_dept_id) {
                    log::warn!(
                        "User {} (dept {:?}) attempted to access file request {} in dept {}",
                        auth.user_id,
                        user_department_id,
                        request.id,
                        req_dept_id
                    );
                    return Err(StatusCode::FORBIDDEN);
                }
            }
            // If request has no department, allow access (tenant-wide)
        }
    }

    // Get uploads for this request
    let uploads = state
        .store
        .file_requests()
        .list_uploads(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base_url = types::config::get_config().web.base_url.clone();

    Ok(Json(json!({
        "id": request.id,
        "name": request.name,
        "destination": request.destination_path,
        "token": request.token,
        "link": format!("{}/upload/{}", base_url, request.token),
        "expires_at": request.expires_at,
        "status": request.status,
        "upload_count": request.upload_count,
        "max_uploads": request.max_uploads,
        "visibility": request.visibility,
        "created_at": request.created_at,
        "uploads": uploads,
    })))
}

/// Revoke/Delete a file request
/// DELETE /api/file-requests/:id
pub async fn delete_file_request(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // First fetch the request to check permissions
    let request = state
        .store
        .file_requests()
        .by_tenant_and_id(auth.tenant_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Security check: enforce visibility/ownership rules for deletion
    let visibility = &request.visibility;

    // Admins can delete any request in their tenant
    if auth.role != "SuperAdmin" && auth.role != "Admin" {
        if visibility == "private" {
            // Private requests: only the creator can delete
            if request.created_by != auth.user_id {
                log::warn!(
                    "User {} attempted to delete private file request {} owned by {}",
                    auth.user_id,
                    request.id,
                    request.created_by
                );
                return Err(StatusCode::FORBIDDEN);
            }
        } else {
            // Department visibility: check department membership
            let user = state
                .store
                .users()
                .user(auth.user_id)
                .await
                .unwrap_or(None);

            let user_department_id = user.and_then(|u| u.department_id);

            // If request has a department, user must be in that department
            if let Some(req_dept_id) = request.department_id {
                if user_department_id != Some(req_dept_id) {
                    log::warn!(
                        "User {} (dept {:?}) attempted to delete file request {} in dept {}",
                        auth.user_id,
                        user_department_id,
                        request.id,
                        req_dept_id
                    );
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }
    }

    // Now perform the deletion
    state
        .store
        .file_requests()
        .revoke(auth.tenant_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({"success": true})))
}

/// Permanently delete a file request (removes from database)
/// DELETE /api/file-requests/:id/permanent
pub async fn permanent_delete_file_request(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Only allow SuperAdmin or Admin to permanently delete
    if auth.role != "SuperAdmin" && auth.role != "Admin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // First delete related uploads
    state
        .store
        .file_requests()
        .delete_uploads(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Then delete the file request
    let deleted = state
        .store
        .file_requests()
        .delete_permanent(auth.tenant_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(json!({"success": true, "permanently_deleted": true})))
}

/// Get uploads for a file request
/// GET /api/file-requests/:id/uploads
pub async fn get_file_request_uploads(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Verify the request belongs to this tenant
    state
        .store
        .file_requests()
        .by_tenant_and_id(auth.tenant_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let uploads = state
        .store
        .file_requests()
        .list_uploads(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!(uploads)))
}

/// Public upload endpoint (no auth required)
/// Uses streaming upload with content-addressed storage (zero-copy, deduplication)
/// POST /api/public-upload/:token
pub async fn public_upload(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<Value>, StatusCode> {
    // Find the file request by token
    let file_request = state
        .store
        .file_requests()
        .by_active_token(&token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check compliance mode - verify public sharing is still allowed
    let compliance_mode = get_tenant_compliance_mode(&state.store, file_request.tenant_id)
        .await
        .unwrap_or_else(|_| "Standard".to_string());
    let restrictions = ComplianceRestrictions::for_mode(&compliance_mode);

    // Block upload if compliance mode now restricts public sharing
    // (e.g., if mode was changed after the request was created)
    if restrictions.public_sharing_blocked {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if expired
    if file_request.expires_at < Utc::now() {
        return Err(StatusCode::GONE); // 410 Gone
    }

    // Check max uploads limit
    if let Some(max) = file_request.max_uploads {
        if file_request.upload_count >= max {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Get tenant upload limits for size validation
    let tenant_model = state
        .store
        .tenants()
        .by_id(file_request.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let tenant_max_upload_size = tenant_model.max_upload_size_bytes;
    let blocked_extensions = tenant_model.blocked_extensions.clone().unwrap_or_default();

    let mut uploaded_files = vec![];

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let file_name = field
            .file_name()
            .ok_or(StatusCode::BAD_REQUEST)?
            .to_string();

        // Check for blocked file extensions
        if !blocked_extensions.is_empty() {
            if let Some(ext) = std::path::Path::new(&file_name)
                .extension()
                .and_then(|e| e.to_str())
            {
                let ext_lower = ext.to_lowercase();
                if blocked_extensions
                    .iter()
                    .any(|b| b.to_lowercase() == ext_lower)
                {
                    log::warn!(
                        "Public upload blocked: attempted to upload blocked extension .{} (file: {}, request: {})",
                        ext_lower, file_name, token
                    );
                    // Create security alert for blocked extension attempt
                    let _ = security_service::alert_blocked_extension(
                        &state.store,
                        file_request.tenant_id,
                        None, // No authenticated user
                        None, // No user email
                        &file_name,
                        &ext_lower,
                        None, // Could extract from headers if needed
                        true, // Is public upload
                    )
                    .await;
                    return Ok(Json(json!({
                        "error": "blocked_extension",
                        "message": format!("File type .{} is not allowed", ext_lower),
                        "extension": ext_lower
                    })));
                }
            }
        }

        let content_type = field.content_type().map(|s| s.to_string());

        // === STREAMING UPLOAD: Stream to temp file while computing Blake3 hash ===
        let temp_dir = std::env::temp_dir();
        let temp_file_name = format!("clovalink_public_upload_{}_{}", Uuid::new_v4(), &file_name);
        let temp_path = temp_dir.join(&temp_file_name);

        let mut temp_file = tokio::fs::File::create(&temp_path).await.map_err(|e| {
            log::error!("Failed to create temp file for public upload: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let mut size: i64 = 0;
        let mut hasher = blake3::Hasher::new();

        // Stream chunks to temp file while computing hash (constant memory usage)
        while let Some(chunk) = field.chunk().await.map_err(|e| {
            log::error!("Failed to read chunk in public upload: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })? {
            size += chunk.len() as i64;

            // Check max upload size limit during streaming
            if let Some(max_size) = tenant_max_upload_size {
                if size > max_size {
                    // Clean up temp file before returning error
                    drop(temp_file);
                    let _ = tokio::fs::remove_file(&temp_path).await;
                    log::warn!("Public upload exceeded max size: {} > {}", size, max_size);
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
            }

            hasher.update(&chunk);
            temp_file.write_all(&chunk).await.map_err(|e| {
                log::error!("Failed to write chunk to temp file: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }

        // Finalize Blake3 hash
        let content_hash = hasher.finalize().to_hex().to_string();

        // Flush and close temp file
        temp_file
            .flush()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        drop(temp_file);

        // Generate unique filename for display (keeps original name readable)
        let unique_filename = format!("{}-{}", nanoid::nanoid!(8), file_name);

        // Content-addressed storage path: tenant_id/department_id/content_hash
        // This enables deduplication - same content stored once
        let department_id = file_request.department_id.unwrap_or(Uuid::nil());
        let storage_path = format!(
            "{}/{}/{}",
            file_request.tenant_id, department_id, content_hash
        );

        // Check if content already exists in storage (deduplication)
        let content_exists = state.storage.exists(&storage_path).await.unwrap_or(false);

        if !content_exists {
            // Acquire transfer scheduler permit based on file size (prioritizes small files)
            let transfer_permit = state.scheduler.acquire_upload_permit(Some(size)).await;
            log::debug!(
                "Public upload permit acquired: token={}, size={}, class={}",
                token,
                size,
                transfer_permit.size_class.name()
            );

            // Upload from temp file (streaming, zero-copy)
            state
                .storage
                .upload_from_path(&storage_path, &temp_path)
                .await
                .map_err(|e| {
                    log::error!("Storage error in public upload: {:?}", e);
                    // Clean up temp file on error
                    let _ = std::fs::remove_file(&temp_path);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            log::debug!("Uploaded new content to storage: {}", storage_path);

            // Permit is released here when upload completes
            drop(transfer_permit);
        } else {
            log::debug!("Content already exists, deduplicating: {}", storage_path);
        }

        // Clean up temp file after successful upload
        if let Err(e) = tokio::fs::remove_file(&temp_path).await {
            log::warn!("Failed to remove temp file: {:?}", e);
        }

        // Enqueue S3 replication if enabled (only for new content, not deduplicated)
        if state.replication_config.enabled && !content_exists {
            let replication_store = state.store.clone();
            let storage_key = storage_path.clone();
            let tenant_id = file_request.tenant_id;
            tokio::spawn(async move {
                if let Err(e) = app_core::replication::enqueue_upload(
                    &replication_store,
                    &storage_key,
                    tenant_id,
                    Some(size),
                )
                .await
                {
                    log::warn!(
                        target: "replication",
                        "Failed to enqueue replication job for public upload (storage_path: {}, error: {})",
                        storage_key,
                        e
                    );
                }
            });
        }

        // 1. Create FileMetadata entry so it shows up in the file manager
        // Include content_hash for deduplication tracking
        let ulid = ulid::Ulid::new().to_string();
        let file_metadata = state
            .store
            .files()
            .create(
                Uuid::new_v4(),
                file_request.tenant_id,
                file_request.department_id,
                unique_filename.clone(),
                storage_path.clone(),
                size,
                content_type.clone(),
                file_request.created_by,
                None, // parent_path
                "private".to_string(), // visibility
                ulid,
                Some(content_hash),
            )
            .await
            .map_err(|e| {
                log::error!("Failed to create file metadata: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Enqueue virus scan job if enabled (non-blocking)
        if state.virus_scan_config.enabled {
            let scan_store = state.store.clone();
            let file_id = file_metadata.id;
            let tenant_id = file_request.tenant_id;
            let max_queue_size = state.virus_scan_config.max_queue_size;
            tokio::spawn(async move {
                if let Err(e) = app_core::virus_scan::enqueue_scan_with_backpressure(
                    &scan_store,
                    file_id,
                    tenant_id,
                    0, // Normal priority
                    max_queue_size,
                )
                .await
                {
                    log::warn!(
                        target: "virus_scan",
                        "Failed to enqueue virus scan job for public upload (file_id: {}, error: {})",
                        file_id,
                        e
                    );
                }
            });
        }

        // 2. Save upload record linked to metadata
        let upload = state
            .store
            .file_requests()
            .create_upload(
                file_request.id,
                Some(file_metadata.id),
                unique_filename,
                file_name,
                size,
                content_type,
                storage_path,
            )
            .await
            .map_err(|e| {
                log::error!("Failed to create upload record: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        uploaded_files.push(upload);
    }

    // Update upload count
    state
        .store
        .file_requests()
        .increment_upload_count(file_request.id, uploaded_files.len() as i32)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Send notification to the file request owner
    if !uploaded_files.is_empty() {
        // Get request owner details
        let owner = state
            .store
            .users()
            .user(file_request.created_by)
            .await
            .ok()
            .flatten();

        if let Some(owner_user) = owner {
            let tenant = Tenant::from(tenant_model);
            // Notify about the first uploaded file (or summarize if multiple)
            let first_file = &uploaded_files[0];
            let uploader_name = "External user"; // Public uploads don't have a known uploader

            if let Some(file_id) = first_file.file_metadata_id {
                let _ = notification_service::notify_file_upload(
                    &state.store,
                    &tenant,
                    file_request.created_by,
                    &owner_user.email,
                    &owner_user.role,
                    &file_request.name,
                    uploader_name,
                    &first_file.original_filename,
                    file_id,
                    file_request.id,
                )
                .await;

                // Also send Discord DM notification (fire-and-forget)
                let store_clone = state.store.clone();
                let tenant_id = file_request.tenant_id;
                let owner_id = file_request.created_by;
                let file_name = first_file.original_filename.clone();
                let request_name = file_request.name.clone();
                tokio::spawn(async move {
                    crate::discord::notify_file_upload(
                        &store_clone,
                        tenant_id,
                        owner_id,
                        &file_name,
                        "External user",
                        &request_name,
                    )
                    .await;
                });
            }
        }
    }

    Ok(Json(json!({
        "success": true,
        "uploaded": uploaded_files.len(),
        "files": uploaded_files,
    })))
}
