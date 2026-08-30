use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use crate::AppState;
use clovalink_auth::{AuthUser, require_admin};
use clovalink_core::models::{Department, CreateDepartmentInput, UpdateDepartmentInput};

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

    let departments = sqlx::query_as::<_, Department>(
        "SELECT * FROM departments WHERE tenant_id = $1 ORDER BY name ASC"
    )
    .bind(tenant_id)
    .fetch_all(&state.pool)
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

    let department = sqlx::query_as::<_, Department>(
        r#"
        INSERT INTO departments (tenant_id, name, description)
        VALUES ($1, $2, $3)
        RETURNING *
        "#
    )
    .bind(tenant_id)
    .bind(&input.name)
    .bind(&input.description)
    .fetch_one(&state.pool)
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

    let mut updates = Vec::new();
    let mut param_count = 3; // $1 is id, $2 is tenant_id

    if let Some(_name) = &input.name {
        updates.push(format!("name = ${}", param_count));
        param_count += 1;
    }
    if let Some(_description) = &input.description {
        updates.push(format!("description = ${}", param_count));
    }

    if updates.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    updates.push("updated_at = NOW()".to_string());
    let query = format!(
        "UPDATE departments SET {} WHERE id = $1 AND tenant_id = $2 RETURNING *",
        updates.join(", ")
    );

    let mut db_query = sqlx::query_as::<_, Department>(&query)
        .bind(id)
        .bind(auth.tenant_id);

    if let Some(name) = input.name {
        db_query = db_query.bind(name);
    }
    if let Some(description) = input.description {
        db_query = db_query.bind(description);
    }

    let department = db_query
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
    
    let result = sqlx::query!(
        "DELETE FROM departments WHERE id = $1 AND tenant_id = $2",
        id,
        auth.tenant_id
    )
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(json!({"success": true})))
}
