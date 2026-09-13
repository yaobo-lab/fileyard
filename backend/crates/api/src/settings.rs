use crate::compliance::ComplianceRestrictions;
use crate::AppState;
use axum::{extract::State, http::StatusCode, response::Json, Extension};
use clovalink_auth::{require_admin, AuthUser};
use clovalink_core::cache::{keys as cache_keys, ttl as cache_ttl};
use clovalink_core::models::Tenant;
use clovalink_core::notification_service;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct UpdateComplianceInput {
    pub compliance_mode: String, // HIPAA, SOX, GDPR, Standard
    pub retention_policy_days: Option<i32>,
    pub data_export_enabled: Option<bool>,
}

/// Cached compliance settings response
#[derive(Serialize, Deserialize, Clone)]
struct ComplianceSettingsCache {
    compliance_mode: String,
    encryption_standard: String,
    retention_policy_days: i32,
    mfa_required: bool,
    session_timeout_minutes: Option<i32>,
    public_sharing_enabled: bool,
    data_export_enabled: bool,
}

#[derive(Deserialize)]
pub struct UpdateBlockedExtensionsInput {
    pub blocked_extensions: Vec<String>,
}

/// Get compliance settings for current tenant
/// GET /api/settings/compliance
/// Requires Admin or SuperAdmin role
pub async fn get_compliance(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    // SECURITY: Settings are Admin/SuperAdmin only
    require_admin(&auth)?;

    let cache_key = cache_keys::tenant_settings(auth.tenant_id);

    // Try cache first
    if let Some(ref cache) = state.cache {
        if let Ok(cached) = cache.get::<ComplianceSettingsCache>(&cache_key).await {
            let restrictions = ComplianceRestrictions::for_mode(&cached.compliance_mode);
            return Ok(Json(json!({
                "compliance_mode": cached.compliance_mode,
                "encryption_standard": cached.encryption_standard,
                "retention_policy_days": cached.retention_policy_days,
                "mfa_required": cached.mfa_required,
                "session_timeout_minutes": cached.session_timeout_minutes,
                "public_sharing_enabled": cached.public_sharing_enabled,
                "data_export_enabled": cached.data_export_enabled,
                "restrictions": restrictions,
            })));
        }
    }

    let tenant = state
        .store
        .tenants()
        .by_id(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let compliance_mode = tenant.compliance_mode;
    let encryption_standard = tenant.encryption_standard;
    let retention_policy_days = tenant.retention_policy_days;
    let mfa_required = tenant.mfa_required;
    let session_timeout_minutes = tenant.session_timeout_minutes;
    let public_sharing_enabled = tenant.public_sharing_enabled;
    let data_export_enabled = tenant.data_export_enabled;

    // Get compliance restrictions for the mode
    let restrictions = ComplianceRestrictions::for_mode(&compliance_mode);

    // Cache the settings
    let cache_data = ComplianceSettingsCache {
        compliance_mode: compliance_mode.clone(),
        encryption_standard: encryption_standard.clone(),
        retention_policy_days,
        mfa_required: mfa_required.unwrap_or(false),
        session_timeout_minutes,
        public_sharing_enabled: public_sharing_enabled.unwrap_or(true),
        data_export_enabled: data_export_enabled.unwrap_or(true),
    };

    if let Some(ref cache) = state.cache {
        let _ = cache
            .set(&cache_key, &cache_data, cache_ttl::TENANT_SETTINGS)
            .await;
    }

    Ok(Json(json!({
        "compliance_mode": compliance_mode,
        "encryption_standard": encryption_standard,
        "retention_policy_days": retention_policy_days,
        "mfa_required": mfa_required.unwrap_or(false),
        "session_timeout_minutes": session_timeout_minutes,
        "public_sharing_enabled": public_sharing_enabled.unwrap_or(true),
        "data_export_enabled": data_export_enabled.unwrap_or(true),
        "restrictions": restrictions,
    })))
}

/// Update compliance mode
/// PUT /api/settings/compliance
/// Requires Admin role
pub async fn update_compliance(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<UpdateComplianceInput>,
) -> Result<Json<Value>, StatusCode> {
    // Check permissions
    require_admin(&auth)?;

    // Normalize compliance mode names
    let compliance_mode = match input.compliance_mode.to_uppercase().as_str() {
        "HIPAA" => "HIPAA",
        "SOX" | "SOC2" => "SOX",
        "GDPR" => "GDPR",
        "STANDARD" | "NONE" | "" => "Standard",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Get restrictions for the new mode
    let restrictions = ComplianceRestrictions::for_mode(compliance_mode);

    // Validate retention policy against compliance requirements
    let retention_days = input.retention_policy_days.unwrap_or(90);
    if let Some(min_days) = restrictions.min_retention_days {
        if retention_days < min_days {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Validate retention policy values
    let valid_retention = [30, 60, 90, 120, 180, 365, 730];
    if !valid_retention.contains(&retention_days) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Apply compliance mode enforcement settings
    let mfa_required = restrictions.mfa_required;
    let public_sharing_enabled = !restrictions.public_sharing_blocked;
    let session_timeout = restrictions.session_timeout_minutes;
    let data_export_enabled = input.data_export_enabled.unwrap_or(true);

    // Update tenant with compliance settings
    state
        .store
        .tenants()
        .update_compliance_settings(
            auth.tenant_id,
            compliance_mode.to_string(),
            retention_days,
            mfa_required,
            public_sharing_enabled,
            session_timeout,
            data_export_enabled,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create audit log
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            "update_compliance_mode",
            "tenant",
            Some(auth.tenant_id),
            Some(json!({
                "new_mode": compliance_mode,
                "new_retention_days": retention_days,
                "mfa_required": mfa_required,
                "public_sharing_enabled": public_sharing_enabled,
                "enforced_settings": restrictions.enforced_settings,
            })),
            auth.ip_address.clone(),
        )
        .await;

    // Invalidate compliance and tenant settings caches
    if let Some(ref cache) = state.cache {
        let compliance_key = cache_keys::compliance(auth.tenant_id);
        let settings_key = cache_keys::tenant_settings(auth.tenant_id);
        if let Err(e) = cache.delete(&compliance_key).await {
            tracing::warn!("Failed to invalidate compliance cache: {}", e);
        }
        if let Err(e) = cache.delete(&settings_key).await {
            tracing::warn!("Failed to invalidate tenant settings cache: {}", e);
        }
    }

    // Notify all admins about the compliance mode change
    if let Ok(Some(tenant_model)) = state.store.tenants().by_id(auth.tenant_id).await {
        let tenant: Tenant = tenant_model.into();
        let _ = notification_service::notify_all_admins(
            &state.store,
            &tenant,
            notification_service::NotificationType::ComplianceAlert,
            &format!("Compliance mode changed to {}", compliance_mode),
            &format!(
                "The compliance mode has been updated to {}. New restrictions: MFA {}, Public sharing {}.",
                compliance_mode,
                if mfa_required { "required" } else { "optional" },
                if public_sharing_enabled { "enabled" } else { "disabled" }
            ),
            Some(json!({
                "new_mode": compliance_mode,
                "mfa_required": mfa_required,
                "public_sharing_enabled": public_sharing_enabled,
                "changed_by": auth.user_id
            })),
        ).await;
    }

    Ok(Json(json!({
        "compliance_mode": compliance_mode,
        "retention_policy_days": retention_days,
        "mfa_required": mfa_required,
        "public_sharing_enabled": public_sharing_enabled,
        "data_export_enabled": data_export_enabled,
        "session_timeout_minutes": session_timeout,
        "restrictions": restrictions,
        "success": true
    })))
}

/// Get blocked file extensions for current tenant
/// GET /api/settings/blocked-extensions
/// Requires Admin or SuperAdmin role
pub async fn get_blocked_extensions(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    let tenant = state
        .store
        .tenants()
        .by_id(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let extensions = tenant.blocked_extensions.unwrap_or_default();

    Ok(Json(json!({
        "blocked_extensions": extensions
    })))
}

/// Update blocked file extensions for current tenant
/// PUT /api/settings/blocked-extensions
/// Requires Admin or SuperAdmin role
pub async fn update_blocked_extensions(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<UpdateBlockedExtensionsInput>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Normalize extensions: lowercase, remove dots, trim whitespace
    let normalized: Vec<String> = input
        .blocked_extensions
        .iter()
        .map(|ext| {
            ext.trim()
                .to_lowercase()
                .trim_start_matches('.')
                .to_string()
        })
        .filter(|ext| !ext.is_empty())
        .collect();

    // Update tenant
    state
        .store
        .tenants()
        .update_blocked_extensions(auth.tenant_id, normalized.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create audit log
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            "update_blocked_extensions",
            "settings",
            Some(auth.tenant_id),
            Some(json!({
                "blocked_extensions": &normalized,
                "count": normalized.len()
            })),
            auth.ip_address.clone(),
        )
        .await;

    Ok(Json(json!({
        "blocked_extensions": normalized,
        "success": true
    })))
}

// ============================================================================
// Password Policy Settings
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PasswordPolicy {
    pub min_length: i32,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_number: bool,
    pub require_special: bool,
    pub max_age_days: Option<i32>,
    pub prevent_reuse: i32,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_number: true,
            require_special: false,
            max_age_days: None,
            prevent_reuse: 0,
        }
    }
}

/// Validate a password against the policy
pub fn validate_password(password: &str, policy: &PasswordPolicy) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if password.len() < policy.min_length as usize {
        errors.push(format!(
            "Password must be at least {} characters",
            policy.min_length
        ));
    }

    if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        errors.push("Password must contain at least one uppercase letter".to_string());
    }

    if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
        errors.push("Password must contain at least one lowercase letter".to_string());
    }

    if policy.require_number && !password.chars().any(|c| c.is_numeric()) {
        errors.push("Password must contain at least one number".to_string());
    }

    if policy.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
        errors.push("Password must contain at least one special character".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Get password policy for current tenant
/// GET /api/settings/password-policy
pub async fn get_password_policy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<PasswordPolicy>, StatusCode> {
    let tenant = state
        .store
        .tenants()
        .by_id(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match tenant.and_then(|t| t.password_policy) {
        Some(json_value) => {
            let policy: PasswordPolicy = serde_json::from_value(json_value).unwrap_or_default();
            Ok(Json(policy))
        }
        None => Ok(Json(PasswordPolicy::default())),
    }
}

/// Update password policy for current tenant
/// PUT /api/settings/password-policy
/// Requires Admin or SuperAdmin role
pub async fn update_password_policy(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(policy): Json<PasswordPolicy>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Validate policy values
    if policy.min_length < 4 || policy.min_length > 128 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if policy.prevent_reuse < 0 || policy.prevent_reuse > 24 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let policy_json =
        serde_json::to_value(&policy).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state
        .store
        .tenants()
        .update_password_policy(auth.tenant_id, policy_json.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            "update_password_policy",
            "settings",
            Some(auth.tenant_id),
            Some(policy_json),
            auth.ip_address.clone(),
        )
        .await;

    Ok(Json(json!({ "success": true, "policy": policy })))
}

// ============================================================================
// IP Restriction Settings
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IpRestrictions {
    pub mode: String, // "disabled", "allowlist_only", "blocklist_only", "both"
    pub allowlist: Vec<String>,
    pub blocklist: Vec<String>,
}

impl Default for IpRestrictions {
    fn default() -> Self {
        Self {
            mode: "disabled".to_string(),
            allowlist: Vec::new(),
            blocklist: Vec::new(),
        }
    }
}

/// Get IP restrictions for current tenant
/// GET /api/settings/ip-restrictions
/// Requires Admin or SuperAdmin role
pub async fn get_ip_restrictions(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<IpRestrictions>, StatusCode> {
    require_admin(&auth)?;

    let tenant = state
        .store
        .tenants()
        .by_id(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match tenant {
        Some(t) => Ok(Json(IpRestrictions {
            mode: t.ip_restriction_mode.unwrap_or_else(|| "disabled".to_string()),
            allowlist: t.ip_allowlist.unwrap_or_default(),
            blocklist: t.ip_blocklist.unwrap_or_default(),
        })),
        None => Ok(Json(IpRestrictions::default())),
    }
}

/// Update IP restrictions for current tenant
/// PUT /api/settings/ip-restrictions
/// Requires Admin or SuperAdmin role
pub async fn update_ip_restrictions(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(restrictions): Json<IpRestrictions>,
) -> Result<Json<Value>, StatusCode> {
    require_admin(&auth)?;

    // Validate mode
    let valid_modes = ["disabled", "allowlist_only", "blocklist_only", "both"];
    if !valid_modes.contains(&restrictions.mode.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Basic validation of IP/CIDR format (simple check)
    let validate_ip_list = |list: &[String]| -> bool {
        list.iter().all(|ip| {
            // Allow IP addresses and CIDR notation
            let trimmed = ip.trim();
            if trimmed.is_empty() {
                return false;
            }
            // Simple check - contains dots or colons (IPv4/IPv6)
            trimmed.contains('.') || trimmed.contains(':')
        })
    };

    if !restrictions.allowlist.is_empty() && !validate_ip_list(&restrictions.allowlist) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !restrictions.blocklist.is_empty() && !validate_ip_list(&restrictions.blocklist) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Normalize lists
    let allowlist: Vec<String> = restrictions
        .allowlist
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let blocklist: Vec<String> = restrictions
        .blocklist
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    state
        .store
        .tenants()
        .update_ip_restrictions(
            auth.tenant_id,
            restrictions.mode.clone(),
            allowlist.clone(),
            blocklist.clone(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    let _ = state
        .store
        .audit()
        .log(
            auth.tenant_id,
            Some(auth.user_id),
            "update_ip_restrictions",
            "settings",
            Some(auth.tenant_id),
            Some(json!({
                "mode": &restrictions.mode,
                "allowlist_count": allowlist.len(),
                "blocklist_count": blocklist.len()
            })),
            auth.ip_address.clone(),
        )
        .await;

    Ok(Json(json!({
        "success": true,
        "mode": restrictions.mode,
        "allowlist": allowlist,
        "blocklist": blocklist
    })))
}
