//! SSO Attribute-to-Role Mapping CRUD
//!
//! Protocol-agnostic mappings that work for both OIDC claims and SAML attributes.
//! SuperAdmin only.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use clovalink_auth::{require_super_admin, AuthUser};

// ==================== Models ====================

pub type AttributeMapping = clovalink_entity::entities::sso_attribute_mappings::Model;

#[derive(Debug, Deserialize)]
pub struct CreateMappingInput {
    pub attribute_name: String,
    pub attribute_value: String,
    pub match_type: Option<String>,
    pub target_role: String,
    pub target_custom_role_id: Option<Uuid>,
    pub target_department_id: Option<Uuid>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMappingInput {
    pub attribute_name: Option<String>,
    pub attribute_value: Option<String>,
    pub match_type: Option<String>,
    pub target_role: Option<String>,
    pub target_custom_role_id: Option<Uuid>,
    pub target_department_id: Option<Uuid>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

// ==================== Endpoints ====================

/// List attribute mappings for a provider
/// GET /api/sso/mappings/:protocol/:provider_id
pub async fn list_mappings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((protocol, provider_id)): Path<(String, Uuid)>,
) -> Result<Json<Vec<AttributeMapping>>, StatusCode> {
    require_super_admin(&auth)?;

    if protocol != "oidc" && protocol != "saml" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mappings = state.store.sso().mappings(auth.tenant_id, &protocol, provider_id)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list mappings: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(mappings))
}

/// Create an attribute mapping
/// POST /api/sso/mappings/:protocol/:provider_id
pub async fn create_mapping(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path((protocol, provider_id)): Path<(String, Uuid)>,
    Json(input): Json<CreateMappingInput>,
) -> Result<(StatusCode, Json<AttributeMapping>), (StatusCode, Json<Value>)> {
    require_super_admin(&auth).map_err(|s| (s, Json(json!({"error": "Forbidden"}))))?;

    if protocol != "oidc" && protocol != "saml" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid protocol"})),
        ));
    }

    let match_type = input.match_type.unwrap_or_else(|| "exact".to_string());
    if !["exact", "contains", "regex"].contains(&match_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid match_type. Must be: exact, contains, regex"})),
        ));
    }

    // Validate regex if match_type is regex (with size limit to prevent ReDoS)
    if match_type == "regex" {
        if let Err(e) = regex::RegexBuilder::new(&input.attribute_value)
            .size_limit(10_000)
            .build()
        {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid regex: {}", e)})),
            ));
        }
    }

    let mapping = state.store.sso().create_mapping(clovalink_entity::repositories::NewSsoMapping {
        tenant_id: auth.tenant_id, protocol, provider_id, attribute_name: input.attribute_name,
        attribute_value: input.attribute_value, match_type, target_role: input.target_role,
        target_custom_role_id: input.target_custom_role_id, target_department_id: input.target_department_id,
        priority: input.priority.unwrap_or(0), enabled: input.enabled.unwrap_or(true),
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to create mapping: {:?}", e);
        if e.to_string().contains("duplicate") {
            (
                StatusCode::CONFLICT,
                Json(json!({"error": "Mapping already exists for this attribute name and value"})),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create mapping"})),
            )
        }
    })?;

    Ok((StatusCode::CREATED, Json(mapping)))
}

/// Update an attribute mapping
/// PUT /api/sso/mappings/:mapping_id
pub async fn update_mapping(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(mapping_id): Path<Uuid>,
    Json(input): Json<UpdateMappingInput>,
) -> Result<Json<AttributeMapping>, (StatusCode, Json<Value>)> {
    require_super_admin(&auth).map_err(|s| (s, Json(json!({"error": "Forbidden"}))))?;

    // Verify mapping belongs to this tenant
    let existing = state.store.sso().mapping(auth.tenant_id, mapping_id)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Database error"})),
                )
            })?;

    let existing = existing.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Mapping not found"})),
        )
    })?;

    let match_type = input.match_type.unwrap_or_else(|| existing.match_type.clone());
    if !["exact", "contains", "regex"].contains(&match_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid match_type"})),
        ));
    }

    let attr_value = input.attribute_value.unwrap_or_else(|| existing.attribute_value.clone());
    if match_type == "regex" {
        if let Err(e) = regex::RegexBuilder::new(&attr_value)
            .size_limit(10_000)
            .build()
        {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid regex: {}", e)})),
            ));
        }
    }

    let patch = clovalink_entity::repositories::SsoMappingPatch {
        attribute_name: input.attribute_name.unwrap_or_else(|| existing.attribute_name.clone()), attribute_value: attr_value,
        match_type, target_role: input.target_role.unwrap_or_else(|| existing.target_role.clone()),
        target_custom_role_id: input.target_custom_role_id.or(existing.target_custom_role_id),
        target_department_id: input.target_department_id.or(existing.target_department_id),
        priority: input.priority.unwrap_or(existing.priority), enabled: input.enabled.unwrap_or(existing.enabled),
    };
    let mapping = state.store.sso().update_mapping(existing, patch)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update mapping: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update mapping"})),
        )
    })?;

    Ok(Json(mapping))
}

/// Delete an attribute mapping
/// DELETE /api/sso/mappings/:mapping_id
pub async fn delete_mapping(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(mapping_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    require_super_admin(&auth).map_err(|s| (s, Json(json!({"error": "Forbidden"}))))?;

    let deleted = state.store.sso().delete_mapping(auth.tenant_id, mapping_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Mapping not found"})),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}
