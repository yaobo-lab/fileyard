use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::AppState;
use clovalink_auth::{AuthUser, require_admin, require_super_admin};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EmailTemplate {
    pub id: Uuid,
    pub template_key: String,
    pub name: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: Option<String>,
    pub variables: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantEmailTemplate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub template_key: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateInput {
    pub subject: String,
    pub body_html: String,
    pub body_text: Option<String>,
}

// ==================== Global Templates (SuperAdmin) ====================

/// List all global email templates
/// GET /api/email-templates
pub async fn list_global_templates(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    let templates: Vec<EmailTemplate> = sqlx::query_as(
        "SELECT id, template_key, name, subject, body_html, body_text, variables, created_at, updated_at 
         FROM email_templates ORDER BY name"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch email templates: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!(templates)))
}

/// Get a specific global template
/// GET /api/email-templates/:key
pub async fn get_global_template(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    let template: Option<EmailTemplate> = sqlx::query_as(
        "SELECT id, template_key, name, subject, body_html, body_text, variables, created_at, updated_at 
         FROM email_templates WHERE template_key = $1"
    )
    .bind(&key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch email template: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match template {
        Some(t) => Ok(Json(json!(t))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Update a global template
/// PUT /api/email-templates/:key
pub async fn update_global_template(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(key): Path<String>,
    Json(input): Json<UpdateTemplateInput>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    // Verify template exists
    let exists: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM email_templates WHERE template_key = $1"
    )
    .bind(&key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Update template
    let template: EmailTemplate = sqlx::query_as(
        "UPDATE email_templates SET subject = $1, body_html = $2, body_text = $3, updated_at = NOW()
         WHERE template_key = $4
         RETURNING id, template_key, name, subject, body_html, body_text, variables, created_at, updated_at"
    )
    .bind(&input.subject)
    .bind(&input.body_html)
    .bind(&input.body_text)
    .bind(&key)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update email template: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("SuperAdmin {} updated global email template: {}", auth.user_id, key);

    Ok(Json(json!(template)))
}

// ==================== Tenant Templates (Admin) ====================

/// List tenant email templates with global defaults
/// GET /api/settings/email-templates
pub async fn list_tenant_templates(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Get all global templates
    let global_templates: Vec<EmailTemplate> = sqlx::query_as(
        "SELECT id, template_key, name, subject, body_html, body_text, variables, created_at, updated_at 
         FROM email_templates ORDER BY name"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch email templates: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get tenant-specific overrides
    let tenant_overrides: Vec<TenantEmailTemplate> = sqlx::query_as(
        "SELECT id, tenant_id, template_key, subject, body_html, body_text, created_at, updated_at 
         FROM tenant_email_templates WHERE tenant_id = $1"
    )
    .bind(auth.tenant_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch tenant email templates: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Merge global templates with tenant overrides
    let mut results = Vec::new();
    for global in global_templates {
        let override_template = tenant_overrides.iter().find(|o| o.template_key == global.template_key);
        
        results.push(json!({
            "template_key": global.template_key,
            "name": global.name,
            "variables": global.variables,
            "is_customized": override_template.is_some(),
            "subject": override_template.map(|o| o.subject.clone()).unwrap_or(global.subject.clone()),
            "body_html": override_template.map(|o| o.body_html.clone()).unwrap_or(global.body_html.clone()),
            "body_text": override_template.map(|o| o.body_text.clone()).flatten().or(global.body_text.clone()),
            "global_subject": global.subject,
            "global_body_html": global.body_html,
            "global_body_text": global.body_text,
            "updated_at": override_template.map(|o| o.updated_at).unwrap_or(global.updated_at),
        }));
    }

    Ok(Json(json!(results)))
}

/// Get a specific tenant template (with fallback to global)
/// GET /api/settings/email-templates/:key
pub async fn get_tenant_template(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Get global template first
    let global: Option<EmailTemplate> = sqlx::query_as(
        "SELECT id, template_key, name, subject, body_html, body_text, variables, created_at, updated_at 
         FROM email_templates WHERE template_key = $1"
    )
    .bind(&key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let global = global.ok_or(StatusCode::NOT_FOUND)?;

    // Check for tenant override
    let override_template: Option<TenantEmailTemplate> = sqlx::query_as(
        "SELECT id, tenant_id, template_key, subject, body_html, body_text, created_at, updated_at 
         FROM tenant_email_templates WHERE tenant_id = $1 AND template_key = $2"
    )
    .bind(auth.tenant_id)
    .bind(&key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "template_key": global.template_key,
        "name": global.name,
        "variables": global.variables,
        "is_customized": override_template.is_some(),
        "subject": override_template.as_ref().map(|o| o.subject.clone()).unwrap_or(global.subject.clone()),
        "body_html": override_template.as_ref().map(|o| o.body_html.clone()).unwrap_or(global.body_html.clone()),
        "body_text": override_template.as_ref().map(|o| o.body_text.clone()).flatten().or(global.body_text.clone()),
        "global_subject": global.subject,
        "global_body_html": global.body_html,
        "global_body_text": global.body_text,
    })))
}

/// Update or create a tenant template override
/// PUT /api/settings/email-templates/:key
pub async fn update_tenant_template(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(key): Path<String>,
    Json(input): Json<UpdateTemplateInput>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Verify global template exists
    let global_exists: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM email_templates WHERE template_key = $1"
    )
    .bind(&key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if global_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Upsert tenant template
    let template: TenantEmailTemplate = sqlx::query_as(
        r#"
        INSERT INTO tenant_email_templates (tenant_id, template_key, subject, body_html, body_text)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (tenant_id, template_key) DO UPDATE SET
            subject = EXCLUDED.subject,
            body_html = EXCLUDED.body_html,
            body_text = EXCLUDED.body_text,
            updated_at = NOW()
        RETURNING id, tenant_id, template_key, subject, body_html, body_text, created_at, updated_at
        "#
    )
    .bind(auth.tenant_id)
    .bind(&key)
    .bind(&input.subject)
    .bind(&input.body_html)
    .bind(&input.body_text)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update tenant email template: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Admin {} in tenant {} updated email template: {}", auth.user_id, auth.tenant_id, key);

    Ok(Json(json!({
        "success": true,
        "template": template,
    })))
}

/// Reset tenant template to global default
/// DELETE /api/settings/email-templates/:key
pub async fn reset_tenant_template(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    let result = sqlx::query(
        "DELETE FROM tenant_email_templates WHERE tenant_id = $1 AND template_key = $2"
    )
    .bind(auth.tenant_id)
    .bind(&key)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to reset tenant email template: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        return Ok(Json(json!({
            "success": true,
            "message": "Template was already using global default",
        })));
    }

    tracing::info!("Admin {} in tenant {} reset email template to default: {}", auth.user_id, auth.tenant_id, key);

    Ok(Json(json!({
        "success": true,
        "message": "Template reset to global default",
    })))
}

/// Preview a template with sample data
/// POST /api/settings/email-templates/:key/preview
#[derive(Debug, Deserialize)]
pub struct PreviewInput {
    pub subject: String,
    pub body_html: String,
}

pub async fn preview_template(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(key): Path<String>,
    Json(input): Json<PreviewInput>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Get variables for this template
    let template: Option<EmailTemplate> = sqlx::query_as(
        "SELECT id, template_key, name, subject, body_html, body_text, variables, created_at, updated_at 
         FROM email_templates WHERE template_key = $1"
    )
    .bind(&key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = template.ok_or(StatusCode::NOT_FOUND)?;

    // Get tenant and user info for preview
    let tenant_name: String = sqlx::query_scalar(
        "SELECT name FROM tenants WHERE id = $1"
    )
    .bind(auth.tenant_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or_else(|_| "Your Company".to_string());

    let user_name: String = sqlx::query_scalar(
        "SELECT name FROM users WHERE id = $1"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or_else(|_| "John Doe".to_string());

    // Sample data for preview
    let sample_data = json!({
        "user_name": user_name,
        "company_name": tenant_name,
        "file_name": "example-document.pdf",
        "request_name": "Q4 Reports",
        "uploader_name": "Jane Smith",
        "sharer_name": "Bob Johnson",
        "new_user_name": "New Employee",
        "new_user_email": "new@company.com",
        "new_user_role": "Employee",
        "old_role": "Employee",
        "new_role": "Manager",
        "role": "Employee",
        "days_until_expiry": "3",
        "percentage_used": "85",
        "alert_type": "Retention Policy Violation",
        "message": "Files older than 30 days found that should have been archived.",
        "reset_link": "https://app.example.com/reset-password?token=xxx",
        "user_email": "user@company.com",
        "temp_password": "Temp123!",
        "app_url": "https://app.example.com",
    });

    // Replace variables in subject and body
    let mut preview_subject = input.subject.clone();
    let mut preview_body = input.body_html.clone();

    if let Some(vars) = template.variables.as_array() {
        for var in vars {
            if let Some(var_name) = var.as_str() {
                let placeholder = format!("{{{{{}}}}}", var_name);
                let default_value = format!("[{}]", var_name);
                let value = sample_data.get(var_name)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&default_value);
                preview_subject = preview_subject.replace(&placeholder, value);
                preview_body = preview_body.replace(&placeholder, value);
            }
        }
    }

    Ok(Json(json!({
        "subject": preview_subject,
        "body_html": preview_body,
        "variables": template.variables,
        "sample_data": sample_data,
    })))
}

