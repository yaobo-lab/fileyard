//! Compliance Mode Enforcement Module
//! 
//! This module provides enforcement logic for compliance modes:
//! - Standard: No restrictions
//! - HIPAA: Healthcare data protection requirements
//! - SOX: Financial audit and governance requirements  
//! - GDPR: European data protection requirements

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::AppState;
use clovalink_auth::AuthUser;

// ==================== Compliance Mode Enum ====================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceMode {
    Standard,
    HIPAA,
    SOX,
    GDPR,
}

impl ComplianceMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "HIPAA" => ComplianceMode::HIPAA,
            "SOX" | "SOC2" => ComplianceMode::SOX,
            "GDPR" => ComplianceMode::GDPR,
            _ => ComplianceMode::Standard,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ComplianceMode::Standard => "Standard",
            ComplianceMode::HIPAA => "HIPAA",
            ComplianceMode::SOX => "SOX",
            ComplianceMode::GDPR => "GDPR",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            ComplianceMode::Standard => "Standard",
            ComplianceMode::HIPAA => "HIPAA Secure",
            ComplianceMode::SOX => "SOX Governed",
            ComplianceMode::GDPR => "GDPR Active",
        }
    }
}

// ==================== Compliance Restrictions ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRestrictions {
    pub mode: String,
    pub mode_label: String,
    pub is_active: bool,
    pub mfa_required: bool,
    pub mfa_locked: bool,
    pub session_timeout_minutes: Option<i32>,
    pub session_timeout_locked: bool,
    pub audit_logging_mandatory: bool,
    pub audit_settings_locked: bool,
    pub public_sharing_blocked: bool,
    pub public_sharing_locked: bool,
    pub file_versioning_required: bool,
    pub retention_policy_locked: bool,
    pub min_retention_days: Option<i32>,
    pub deletion_requests_allowed: bool,
    pub consent_tracking_required: bool,
    pub export_logging_required: bool,
    pub enforced_settings: Vec<EnforcedSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcedSetting {
    pub name: String,
    pub description: String,
    pub locked: bool,
    pub forced_value: Option<Value>,
}

impl ComplianceRestrictions {
    /// Get restrictions for a compliance mode
    pub fn for_mode(mode: &str) -> Self {
        let compliance_mode = ComplianceMode::from_str(mode);
        
        match compliance_mode {
            ComplianceMode::Standard => Self::standard(),
            ComplianceMode::HIPAA => Self::hipaa(),
            ComplianceMode::SOX => Self::sox(),
            ComplianceMode::GDPR => Self::gdpr(),
        }
    }

    fn standard() -> Self {
        Self {
            mode: "Standard".to_string(),
            mode_label: "Standard".to_string(),
            is_active: false,
            mfa_required: false,
            mfa_locked: false,
            session_timeout_minutes: None,
            session_timeout_locked: false,
            audit_logging_mandatory: false,
            audit_settings_locked: false,
            public_sharing_blocked: false,
            public_sharing_locked: false,
            file_versioning_required: false,
            retention_policy_locked: false,
            min_retention_days: None,
            deletion_requests_allowed: true,
            consent_tracking_required: false,
            export_logging_required: false,
            enforced_settings: vec![],
        }
    }

    fn hipaa() -> Self {
        Self {
            mode: "HIPAA".to_string(),
            mode_label: "HIPAA Secure".to_string(),
            is_active: true,
            mfa_required: true,
            mfa_locked: true,
            session_timeout_minutes: Some(15),
            session_timeout_locked: true,
            audit_logging_mandatory: true,
            audit_settings_locked: true,
            public_sharing_blocked: true,
            public_sharing_locked: true,
            file_versioning_required: false,
            retention_policy_locked: true, // HIPAA requires minimum retention
            min_retention_days: Some(2190), // 6 years (HIPAA requirement for medical records)
            deletion_requests_allowed: true,
            consent_tracking_required: false,
            export_logging_required: true,
            enforced_settings: vec![
                EnforcedSetting {
                    name: "mfa_required".to_string(),
                    description: "MFA is required for all users to protect PHI".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
                EnforcedSetting {
                    name: "session_timeout".to_string(),
                    description: "Sessions auto-expire after 15 minutes of inactivity".to_string(),
                    locked: true,
                    forced_value: Some(json!(15)),
                },
                EnforcedSetting {
                    name: "public_sharing".to_string(),
                    description: "Public/anonymous sharing is disabled to protect PHI".to_string(),
                    locked: true,
                    forced_value: Some(json!(false)),
                },
                EnforcedSetting {
                    name: "audit_logging".to_string(),
                    description: "All access events are logged for compliance".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
                EnforcedSetting {
                    name: "document_retention".to_string(),
                    description: "Documents must be retained for minimum 6 years per HIPAA regulations".to_string(),
                    locked: true,
                    forced_value: Some(json!(2190)),
                },
            ],
        }
    }

    fn sox() -> Self {
        Self {
            mode: "SOX".to_string(),
            mode_label: "SOX Governed".to_string(),
            is_active: true,
            mfa_required: true,
            mfa_locked: true,
            session_timeout_minutes: Some(30),
            session_timeout_locked: false,
            audit_logging_mandatory: true,
            audit_settings_locked: true,
            public_sharing_blocked: true,
            public_sharing_locked: true,
            file_versioning_required: true,
            retention_policy_locked: true,
            min_retention_days: Some(2555), // 7 years (SOX requirement for financial records)
            deletion_requests_allowed: false, // SOX requires document retention
            consent_tracking_required: false,
            export_logging_required: true,
            enforced_settings: vec![
                EnforcedSetting {
                    name: "mfa_required".to_string(),
                    description: "MFA is required for financial data access controls".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
                EnforcedSetting {
                    name: "file_versioning".to_string(),
                    description: "Files cannot be overwritten; new versions are created".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
                EnforcedSetting {
                    name: "public_sharing".to_string(),
                    description: "Public sharing is disabled for financial documents".to_string(),
                    locked: true,
                    forced_value: Some(json!(false)),
                },
                EnforcedSetting {
                    name: "audit_logging".to_string(),
                    description: "All changes to documents and permissions are logged".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
                EnforcedSetting {
                    name: "retention_policy".to_string(),
                    description: "Documents must be retained for minimum 7 years per SOX regulations".to_string(),
                    locked: true,
                    forced_value: Some(json!(2555)),
                },
            ],
        }
    }

    fn gdpr() -> Self {
        Self {
            mode: "GDPR".to_string(),
            mode_label: "GDPR Active".to_string(),
            is_active: true,
            mfa_required: false,
            mfa_locked: false,
            session_timeout_minutes: None,
            session_timeout_locked: false,
            audit_logging_mandatory: true,
            audit_settings_locked: true,
            public_sharing_blocked: false,
            public_sharing_locked: false,
            file_versioning_required: false,
            retention_policy_locked: false,
            min_retention_days: None,
            deletion_requests_allowed: true, // GDPR mandates right to be forgotten
            consent_tracking_required: true,
            export_logging_required: true,
            enforced_settings: vec![
                EnforcedSetting {
                    name: "deletion_requests".to_string(),
                    description: "Data deletion requests cannot be blocked".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
                EnforcedSetting {
                    name: "consent_tracking".to_string(),
                    description: "User consent must be documented for data processing".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
                EnforcedSetting {
                    name: "export_logging".to_string(),
                    description: "All data exports are logged for traceability".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
                EnforcedSetting {
                    name: "retention_auto_delete".to_string(),
                    description: "Data is automatically deleted when retention period expires".to_string(),
                    locked: true,
                    forced_value: Some(json!(true)),
                },
            ],
        }
    }
}

// ==================== Compliance Helper Functions ====================

/// Get the compliance mode for a tenant
pub async fn get_tenant_compliance_mode(pool: &PgPool, tenant_id: Uuid) -> Result<String, StatusCode> {
    let result = sqlx::query_scalar::<_, String>(
        "SELECT compliance_mode FROM tenants WHERE id = $1"
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get tenant compliance mode: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(result)
}

/// Check if an action is allowed under the current compliance mode
pub async fn check_compliance_action(
    pool: &PgPool,
    tenant_id: Uuid,
    action: ComplianceAction,
) -> Result<(), ComplianceViolation> {
    let mode = get_tenant_compliance_mode(pool, tenant_id)
        .await
        .map_err(|_| ComplianceViolation::InternalError)?;
    
    let restrictions = ComplianceRestrictions::for_mode(&mode);
    
    match action {
        ComplianceAction::PublicShare => {
            if restrictions.public_sharing_blocked {
                return Err(ComplianceViolation::ActionBlocked {
                    action: "public_share".to_string(),
                    reason: format!("{} compliance mode prohibits public sharing", mode),
                });
            }
        }
        ComplianceAction::DisableMfa => {
            if restrictions.mfa_locked {
                return Err(ComplianceViolation::ActionBlocked {
                    action: "disable_mfa".to_string(),
                    reason: format!("{} compliance mode requires MFA to be enabled", mode),
                });
            }
        }
        ComplianceAction::DisableAuditLog(setting) => {
            if restrictions.audit_settings_locked {
                return Err(ComplianceViolation::ActionBlocked {
                    action: format!("disable_audit_{}", setting),
                    reason: format!("{} compliance mode requires audit logging to be enabled", mode),
                });
            }
        }
        ComplianceAction::OverwriteFile => {
            if restrictions.file_versioning_required {
                return Err(ComplianceViolation::ActionBlocked {
                    action: "overwrite_file".to_string(),
                    reason: format!("{} compliance mode requires file versioning; files cannot be overwritten", mode),
                });
            }
        }
        ComplianceAction::SetRetentionDays(days) => {
            if let Some(min_days) = restrictions.min_retention_days {
                if days < min_days {
                    return Err(ComplianceViolation::ActionBlocked {
                        action: "set_retention".to_string(),
                        reason: format!("{} compliance mode requires minimum {} day retention", mode, min_days),
                    });
                }
            }
        }
        ComplianceAction::BlockDeletion => {
            if restrictions.deletion_requests_allowed && mode == "GDPR" {
                return Err(ComplianceViolation::ActionBlocked {
                    action: "block_deletion".to_string(),
                    reason: "GDPR compliance requires that deletion requests cannot be blocked".to_string(),
                });
            }
        }
    }
    
    Ok(())
}

/// Check if audit logging should be forced for an action
pub fn should_force_audit_log(mode: &str, action_type: &str) -> bool {
    let restrictions = ComplianceRestrictions::for_mode(mode);
    
    if !restrictions.audit_logging_mandatory {
        return false;
    }
    
    match mode {
        "HIPAA" => matches!(action_type, "file_view" | "file_download" | "file_preview" | "file_access" | "login" | "login_failed"),
        "SOX" => matches!(action_type, "file_upload" | "file_rename" | "file_delete" | "permission_change" | "role_change" | "settings_change"),
        "GDPR" => matches!(action_type, "file_export" | "data_export" | "deletion_request"),
        _ => false,
    }
}

/// Check if a setting can be modified under current compliance mode
pub fn can_modify_setting(mode: &str, setting: &str) -> bool {
    let restrictions = ComplianceRestrictions::for_mode(mode);
    
    match setting {
        "mfa_required" | "enable_totp" => !restrictions.mfa_locked,
        "session_timeout_minutes" => !restrictions.session_timeout_locked,
        "public_sharing_enabled" => !restrictions.public_sharing_locked,
        "log_logins" | "log_file_operations" | "log_user_changes" | "log_settings_changes" | "log_role_changes" => {
            !restrictions.audit_settings_locked
        }
        "retention_policy_days" => !restrictions.retention_policy_locked,
        _ => true,
    }
}

// ==================== Compliance Actions & Violations ====================

#[derive(Debug, Clone)]
pub enum ComplianceAction {
    PublicShare,
    DisableMfa,
    DisableAuditLog(String),
    OverwriteFile,
    SetRetentionDays(i32),
    BlockDeletion,
}

#[derive(Debug, Clone)]
pub enum ComplianceViolation {
    ActionBlocked { action: String, reason: String },
    InternalError,
}

impl ComplianceViolation {
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            ComplianceViolation::ActionBlocked { .. } => StatusCode::FORBIDDEN,
            ComplianceViolation::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            ComplianceViolation::ActionBlocked { action, reason } => {
                json!({
                    "error": "compliance_violation",
                    "action": action,
                    "reason": reason,
                    "message": format!("Action blocked: {}", reason)
                })
            }
            ComplianceViolation::InternalError => {
                json!({
                    "error": "internal_error",
                    "message": "Failed to check compliance requirements"
                })
            }
        }
    }
}

// ==================== Database Models ====================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserConsent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub consent_type: String,
    pub granted_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeletionRequest {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub requested_by: Uuid,
    pub request_type: String,
    pub resource_id: Option<Uuid>,
    pub reason: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileExport {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub file_id: Option<Uuid>,
    pub export_type: String,
    pub file_count: i32,
    pub total_size_bytes: Option<i64>,
    pub exported_at: DateTime<Utc>,
    pub ip_address: Option<std::net::IpAddr>,
    pub metadata: Option<Value>,
}

// ==================== API Input Types ====================

#[derive(Debug, Deserialize)]
pub struct RecordConsentInput {
    pub consent_type: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeletionRequestInput {
    pub user_id: Option<Uuid>,
    pub request_type: String, // user_data, file, all_data
    pub resource_id: Option<Uuid>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListDeletionRequestsParams {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ==================== API Handlers ====================

/// Get compliance restrictions for current tenant
/// GET /api/compliance/restrictions
pub async fn get_compliance_restrictions(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    use clovalink_core::cache::{keys, ttl};
    
    let cache_key = keys::compliance(auth.tenant_id);
    
    // Try to get from cache first
    if let Some(ref cache) = state.cache {
        if let Ok(cached) = cache.get::<ComplianceRestrictions>(&cache_key).await {
            return Ok(Json(json!(cached)));
        }
    }
    
    // Cache miss - fetch from database
    let mode = get_tenant_compliance_mode(&state.pool, auth.tenant_id).await?;
    let restrictions = ComplianceRestrictions::for_mode(&mode);
    
    // Cache the result
    if let Some(ref cache) = state.cache {
        if let Err(e) = cache.set(&cache_key, &restrictions, ttl::COMPLIANCE).await {
            tracing::warn!("Failed to cache compliance restrictions: {}", e);
        }
    }
    
    Ok(Json(json!(restrictions)))
}

/// Record user consent (GDPR)
/// POST /api/compliance/consent
pub async fn record_consent(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<RecordConsentInput>,
) -> Result<Json<Value>, StatusCode> {
    let consent = sqlx::query_as::<_, UserConsent>(
        r#"
        INSERT INTO user_consent (user_id, tenant_id, consent_type, metadata)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, consent_type) WHERE revoked_at IS NULL
        DO UPDATE SET granted_at = NOW(), metadata = $4, updated_at = NOW()
        RETURNING *
        "#
    )
    .bind(auth.user_id)
    .bind(auth.tenant_id)
    .bind(&input.consent_type)
    .bind(&input.metadata)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to record consent: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Log consent recording
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, metadata, ip_address)
        VALUES ($1, $2, 'consent_recorded', 'user', $3, $4::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(json!({
        "consent_type": input.consent_type,
    }))
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await;
    
    Ok(Json(json!(consent)))
}

/// Get user consent status
/// GET /api/compliance/consent/:user_id
pub async fn get_consent_status(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Users can view their own consent, admins can view any user's consent
    if auth.user_id != user_id && !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let consents = sqlx::query_as::<_, UserConsent>(
        r#"
        SELECT * FROM user_consent 
        WHERE user_id = $1 AND tenant_id = $2 AND revoked_at IS NULL
        ORDER BY granted_at DESC
        "#
    )
    .bind(user_id)
    .bind(auth.tenant_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get consent status: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(Json(json!({
        "user_id": user_id,
        "consents": consents,
    })))
}

/// Revoke consent
/// DELETE /api/compliance/consent/:consent_type
pub async fn revoke_consent(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(consent_type): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    sqlx::query(
        r#"
        UPDATE user_consent 
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE user_id = $1 AND tenant_id = $2 AND consent_type = $3 AND revoked_at IS NULL
        "#
    )
    .bind(auth.user_id)
    .bind(auth.tenant_id)
    .bind(&consent_type)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to revoke consent: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Log consent revocation
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, metadata, ip_address)
        VALUES ($1, $2, 'consent_revoked', 'user', $3, $4::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(json!({
        "consent_type": consent_type,
    }))
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await;
    
    Ok(Json(json!({ "success": true })))
}

/// Create GDPR deletion request
/// POST /api/gdpr/deletion-request
pub async fn create_deletion_request(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<CreateDeletionRequestInput>,
) -> Result<Json<Value>, StatusCode> {
    // Check compliance mode allows deletion requests
    let mode = get_tenant_compliance_mode(&state.pool, auth.tenant_id).await?;
    
    // SOX mode may block deletion for retention requirements
    if mode == "SOX" && input.request_type == "all_data" {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let request = sqlx::query_as::<_, DeletionRequest>(
        r#"
        INSERT INTO deletion_requests (tenant_id, user_id, requested_by, request_type, resource_id, reason)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(auth.tenant_id)
    .bind(input.user_id.unwrap_or(auth.user_id))
    .bind(auth.user_id)
    .bind(&input.request_type)
    .bind(input.resource_id)
    .bind(&input.reason)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create deletion request: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Log deletion request
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, resource_id, metadata, ip_address)
        VALUES ($1, $2, 'deletion_request_created', 'deletion_request', $3, $4, $5::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(request.id)
    .bind(json!({
        "request_type": input.request_type,
        "target_user_id": input.user_id,
        "resource_id": input.resource_id,
    }))
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await;
    
    Ok(Json(json!(request)))
}

/// List GDPR deletion requests
/// GET /api/gdpr/deletion-requests
pub async fn list_deletion_requests(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<ListDeletionRequestsParams>,
) -> Result<Json<Value>, StatusCode> {
    // Only admins can list all requests
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);
    
    let mut query = String::from(
        "SELECT * FROM deletion_requests WHERE tenant_id = $1"
    );
    
    if params.status.is_some() {
        query.push_str(" AND status = $4");
    }
    
    query.push_str(" ORDER BY requested_at DESC LIMIT $2 OFFSET $3");
    
    let mut db_query = sqlx::query_as::<_, DeletionRequest>(&query)
        .bind(auth.tenant_id)
        .bind(limit)
        .bind(offset);
    
    if let Some(status) = params.status {
        db_query = db_query.bind(status);
    }
    
    let requests = db_query
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list deletion requests: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(json!({
        "requests": requests,
        "limit": limit,
        "offset": offset,
    })))
}

/// Process a deletion request (admin only)
/// POST /api/gdpr/deletion-requests/:id/process
pub async fn process_deletion_request(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(request_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Get the request
    let request = sqlx::query_as::<_, DeletionRequest>(
        "SELECT * FROM deletion_requests WHERE id = $1 AND tenant_id = $2"
    )
    .bind(request_id)
    .bind(auth.tenant_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
    
    if request.status != "pending" {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // Mark as processing
    sqlx::query(
        "UPDATE deletion_requests SET status = 'processing', processed_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(request_id)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Process based on request type
    match request.request_type.as_str() {
        "user_data" => {
            if let Some(user_id) = request.user_id {
                // Delete user's files
                sqlx::query(
                    "UPDATE files_metadata SET is_deleted = true, deleted_at = NOW() WHERE owner_id = $1 AND tenant_id = $2"
                )
                .bind(user_id)
                .bind(auth.tenant_id)
                .execute(&state.pool)
                .await
                .ok();
                
                // Delete user preferences
                sqlx::query("DELETE FROM user_preferences WHERE user_id = $1")
                    .bind(user_id)
                    .execute(&state.pool)
                    .await
                    .ok();
            }
        }
        "file" => {
            if let Some(file_id) = request.resource_id {
                sqlx::query(
                    "UPDATE files_metadata SET is_deleted = true, deleted_at = NOW() WHERE id = $1 AND tenant_id = $2"
                )
                .bind(file_id)
                .bind(auth.tenant_id)
                .execute(&state.pool)
                .await
                .ok();
            }
        }
        _ => {}
    }
    
    // Mark as completed
    sqlx::query(
        "UPDATE deletion_requests SET status = 'completed', completed_at = NOW(), updated_at = NOW() WHERE id = $1"
    )
    .bind(request_id)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Log completion
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (tenant_id, user_id, action, resource_type, resource_id, metadata, ip_address)
        VALUES ($1, $2, 'deletion_request_completed', 'deletion_request', $3, $4, $5::inet)
        "#
    )
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(request_id)
    .bind(json!({
        "request_type": request.request_type,
        "processed_by": auth.user_id,
    }))
    .bind(&auth.ip_address)
    .execute(&state.pool)
    .await;
    
    Ok(Json(json!({ "success": true, "status": "completed" })))
}

/// Log a file export for GDPR traceability
pub async fn log_file_export(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    file_id: Option<Uuid>,
    export_type: &str,
    file_count: i32,
    total_size: Option<i64>,
    ip_address: Option<String>,
) -> Result<(), StatusCode> {
    sqlx::query(
        r#"
        INSERT INTO file_exports (tenant_id, user_id, file_id, export_type, file_count, total_size_bytes, ip_address)
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet)
        "#
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(file_id)
    .bind(export_type)
    .bind(file_count)
    .bind(total_size)
    .bind(ip_address)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to log file export: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(())
}
