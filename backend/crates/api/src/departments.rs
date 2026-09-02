use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use clovalink_auth::{require_admin, AuthUser};
use clovalink_core::models::{CreateDepartmentInput, UpdateDepartmentInput};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// List departments for a tenant
/// GET /api/departments
pub async fn list_departments(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_id = if auth.role == "SuperAdmin" {
        if let Some(tid) = params.get("tenant_id") {
            Uuid::parse_str(tid).unwrap_or(auth.tenant_id)
        } else {
            auth.tenant_id
        }
    } else {
        auth.tenant_id
    };

    let departments = state
        .store
        .departments()
        .list(tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list departments: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!(departments)))
}

/// Create a new department
/// POST /api/departments
pub async fn create_department(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    Json(input): Json<CreateDepartmentInput>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    let tenant_id = if auth.role == "SuperAdmin" {
        if let Some(tid) = params.get("tenant_id") {
            Uuid::parse_str(tid).unwrap_or(auth.tenant_id)
        } else {
            auth.tenant_id
        }
    } else {
        auth.tenant_id
    };

    let department = state
        .store
        .departments()
        .create(tenant_id, input.name, input.description)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create department: {:?}", e);
            if e.to_string().contains("unique") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(json!(department)))
}

/// Update a department
/// PUT /api/departments/:id
pub async fn update_department(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateDepartmentInput>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    if input.name.is_none() && input.description.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let department = state
        .store
        .departments()
        .update(id, auth.tenant_id, input.name, input.description)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let department = department.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(json!(department)))
}

/// Delete a department
/// DELETE /api/departments/:id
pub async fn delete_department(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Check if there are users or files assigned?
    // The DB constraint might handle it (ON DELETE SET NULL was used in migration).

    let deleted = state
        .store
        .departments()
        .delete(id, auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(json!({"success": true})))
}
