use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use crate::auth::{require_admin, require_super_admin, AuthUser};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

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

    let templates = state
        .store
        .email_templates()
        .list_global()
        .await
        .map_err(|e| {
            log::error!("Failed to fetch email templates: {:?}", e);
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

    let template = state
        .store
        .email_templates()
        .get_global(&key)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch email template: {:?}", e);
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

    let template = state
        .store
        .email_templates()
        .update_global(&key, input.subject, input.body_html, input.body_text)
        .await
        .map_err(|e| {
            log::error!("Failed to update email template: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let Some(template) = template else {
        return Err(StatusCode::NOT_FOUND);
    };

    log::info!(
        "SuperAdmin {} updated global email template: {}",
        auth.user_id,
        key
    );

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

    let global_templates = state
        .store
        .email_templates()
        .list_global()
        .await
        .map_err(|e| {
            log::error!("Failed to fetch email templates: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let tenant_overrides = state
        .store
        .email_templates()
        .list_tenant(auth.tenant_id)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch tenant email templates: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut results = Vec::new();
    for global in global_templates {
        let override_template = tenant_overrides
            .iter()
            .find(|o| o.template_key == global.template_key);

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

    let global = state
        .store
        .email_templates()
        .get_global(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let override_template = state
        .store
        .email_templates()
        .get_tenant(auth.tenant_id, &key)
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

    let global_exists = state
        .store
        .email_templates()
        .get_global(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if global_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let template = state
        .store
        .email_templates()
        .upsert_tenant(
            auth.tenant_id,
            &key,
            input.subject,
            input.body_html,
            input.body_text,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to update tenant email template: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    log::info!(
        "Admin {} in tenant {} updated email template: {}",
        auth.user_id,
        auth.tenant_id,
        key
    );

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

    let deleted = state
        .store
        .email_templates()
        .reset_tenant(auth.tenant_id, &key)
        .await
        .map_err(|e| {
            log::error!("Failed to reset tenant email template: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !deleted {
        return Ok(Json(json!({
            "success": true,
            "message": "Template was already using global default",
        })));
    }

    log::info!(
        "Admin {} in tenant {} reset email template to default: {}",
        auth.user_id,
        auth.tenant_id,
        key
    );

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

    let template = state
        .store
        .email_templates()
        .get_global(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let tenant_name = state
        .store
        .tenants()
        .by_id(auth.tenant_id)
        .await
        .ok()
        .flatten()
        .map(|t| t.name)
        .unwrap_or_else(|| "Your Company".to_string());

    let user_name = state
        .store
        .users()
        .user(auth.user_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.name)
        .unwrap_or_else(|| "John Doe".to_string());

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

    if let Some(vars) = template.variables.as_ref().and_then(|v| v.as_array()) {
        for var in vars {
            if let Some(var_name) = var.as_str() {
                let placeholder = format!("{{{{{}}}}}", var_name);
                let default_value = format!("[{}]", var_name);
                let value = sample_data
                    .get(var_name)
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
