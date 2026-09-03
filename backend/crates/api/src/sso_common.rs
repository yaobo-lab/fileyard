//! Shared SSO Logic
//!
//! Common functions used by both OIDC and SAML SSO handlers:
//! - Account resolution (identity lookup, email matching, auto-provisioning)
//! - Session creation (fingerprint, JWT, session record)
//! - Attribute-to-role mapping

use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

use clovalink_auth::generate_token_with_fingerprint;
use clovalink_core::security_service;
use clovalink_entity::entities::{tenants::Model as Tenant, users::Model as User};

// ==================== Types ====================

/// Result of attribute mapping evaluation
#[derive(Debug, Clone)]
pub struct SsoRoleMapping {
    pub base_role: String,
    pub custom_role_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
}

/// Result of SSO user resolution
pub enum SsoUserResolution {
    /// Existing user found (by identity or email match)
    ExistingUser(User),
    /// New user auto-provisioned
    NewUser(User),
    /// No account found and auto-provision disabled
    NoAccount,
    /// No email available to match
    NoEmail,
}

/// Result of session creation
pub enum SsoSessionResult {
    /// JWT token ready — redirect to frontend
    Token(String),
    /// 2FA required — redirect to login with pending_2fa
    Pending2fa {
        user_id: Uuid,
        provider_slug: String,
    },
    /// User is suspended
    Suspended,
}

/// Parameters for SSO identity linking (protocol-specific)
#[derive(Debug)]
pub struct SsoIdentityParams {
    pub protocol: String, // "oidc" or "saml"
    pub provider_id: Uuid,
    pub tenant_id: Uuid,
    pub subject: String, // oidc_subject or saml_name_id
    pub issuer: String,  // issuer_url or idp_entity_id
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Provider provisioning config
#[derive(Debug)]
pub struct SsoProvisionConfig {
    pub auto_provision: bool,
    pub default_role: String,
    pub default_custom_role_id: Option<Uuid>,
    pub default_department_id: Option<Uuid>,
    pub trust_idp_mfa: bool,
    pub provider_name: String,
    pub provider_slug: String,
}

// ==================== Attribute Mapping ====================

/// Evaluate SSO attribute mappings for a provider.
/// Returns the first matching role/department mapping (by priority DESC), or None.
pub async fn apply_attribute_mapping(
    store: &clovalink_entity::DataStore,
    protocol: &str,
    provider_id: Uuid,
    attributes: &HashMap<String, Vec<String>>,
) -> Option<SsoRoleMapping> {
    if attributes.is_empty() {
        return None;
    }

    let mappings = store.sso().enabled_mappings(protocol, provider_id)
    .await
    .unwrap_or_default();

    for mapping in &mappings {
        if let Some(attr_values) = attributes.get(&mapping.attribute_name) {
            let matched = attr_values
                .iter()
                .any(|val| match mapping.match_type.as_str() {
                    "exact" => val == &mapping.attribute_value,
                    "contains" => val.contains(&mapping.attribute_value),
                    "regex" => regex::RegexBuilder::new(&mapping.attribute_value)
                        .size_limit(10_000)
                        .build()
                        .map(|re| re.is_match(val))
                        .unwrap_or(false),
                    _ => false,
                });

            if matched {
                return Some(SsoRoleMapping {
                    base_role: mapping.target_role.clone(),
                    custom_role_id: mapping.target_custom_role_id,
                    department_id: mapping.target_department_id,
                });
            }
        }
    }

    None
}

// ==================== Account Resolution ====================

/// Resolve an SSO identity to a ClovaLink user.
///
/// Flow: known identity → email match (auto-link) → auto-provision → reject
///
/// This is protocol-agnostic — the caller (OIDC or SAML handler) provides
/// the identity params and this function handles the rest.
pub async fn resolve_sso_user(
    store: &clovalink_entity::DataStore,
    identity: &SsoIdentityParams,
    config: &SsoProvisionConfig,
    role_override: Option<&SsoRoleMapping>,
) -> Result<SsoUserResolution, (StatusCode, String)> {
    let db_err = |_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )
    };

    // Step 1: Look up by subject in the appropriate identity table
    let existing_user_id = store.sso().identity_user_id(&identity.protocol, identity.provider_id, &identity.subject).await.map_err(db_err)?;

    if let Some(user_id) = existing_user_id {
        // Known identity — load user
        let user = store.sso().active_user(user_id).await
                .map_err(db_err)?
                .ok_or_else(|| (StatusCode::FORBIDDEN, "Account is deactivated".to_string()))?;

        return Ok(SsoUserResolution::ExistingUser(user));
    }

    // Step 2: Try email match within same tenant
    let email = match identity.email.as_deref() {
        Some(e) if !e.is_empty() => e,
        _ => return Ok(SsoUserResolution::NoEmail),
    };

    let email_match = store.sso().active_user_by_email(identity.tenant_id, email)
    .await
    .map_err(db_err)?;

    if let Some(user) = email_match {
        // SECURITY: Log auto-linking events for audit trail
        tracing::info!(
            user_id = %user.id,
            email = %email,
            protocol = %identity.protocol,
            provider_id = %identity.provider_id,
            sso_subject = %identity.subject,
            "SSO identity auto-linked by email match"
        );
        link_sso_identity(store, identity, user.id).await;

        // Update identity_provider to hybrid if currently local
        if user.identity_provider == "local" {
            let _ = store.sso().set_hybrid(user.id).await;
        }

        return Ok(SsoUserResolution::ExistingUser(user));
    }

    // Step 3: Auto-provision if enabled
    if !config.auto_provision {
        return Ok(SsoUserResolution::NoAccount);
    }

    let user_name = identity.name.clone().unwrap_or_else(|| email.to_string());
    let identity_provider = &identity.protocol; // "oidc" or "saml"

    // Determine role/department from mapping override or provider defaults
    let base_role = role_override
        .map(|m| m.base_role.clone())
        .unwrap_or_else(|| config.default_role.clone());
    let custom_role_id = role_override
        .and_then(|m| m.custom_role_id)
        .or(config.default_custom_role_id);
    let department_id = role_override
        .and_then(|m| m.department_id)
        .or(config.default_department_id);

    let new_user = store.sso().create_user(identity.tenant_id,email,&user_name,&base_role,custom_role_id,identity_provider,department_id)
    .await
    .map_err(|e| {
        tracing::error!("Failed to auto-provision user: {:?}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user".to_string())
    })?;

    // Link identity
    link_sso_identity(store, &identity, new_user.id).await;

    tracing::info!(
        user_id = %new_user.id,
        email = %email,
        provider = %config.provider_name,
        protocol = %identity.protocol,
        role = %base_role,
        "Auto-provisioned SSO user"
    );

    Ok(SsoUserResolution::NewUser(new_user))
}

/// Create an SSO identity link record in the appropriate table.
async fn link_sso_identity(store: &clovalink_entity::DataStore, identity: &SsoIdentityParams, user_id: Uuid) {
    let _ = store.sso().link_identity(&identity.protocol,user_id,identity.provider_id,&identity.subject,&identity.issuer,identity.email.as_deref(),identity.name.as_deref()).await;
}

// ==================== Session Creation ====================

/// Create an SSO session (JWT + session record + security tracking).
///
/// Handles: suspension check, 2FA check, fingerprinting, JWT generation,
/// session record, IP tracking. Returns a token or pending_2fa redirect.
pub async fn create_sso_session(
    store: &clovalink_entity::DataStore,
    user: &User,
    tenant: &Tenant,
    headers: &HeaderMap,
    config: &SsoProvisionConfig,
    _frontend_url: &str,
) -> Result<SsoSessionResult, (StatusCode, String)> {
    // Check suspension
    if user.suspended_at.is_some() {
        if let Some(until) = user.suspended_until {
            if until > Utc::now() {
                return Ok(SsoSessionResult::Suspended);
            }
        } else {
            return Ok(SsoSessionResult::Suspended);
        }
    }

    // Check if 2FA is required and provider doesn't trust IdP MFA
    if tenant.enable_totp.unwrap_or(false) && !config.trust_idp_mfa && user.totp_secret.is_some() {
        return Ok(SsoSessionResult::Pending2fa {
            user_id: user.id,
            provider_slug: config.provider_slug.clone(),
        });
    }

    // Update last active
    let _ = store.sso().touch_user(user.id).await;

    // Generate session fingerprint
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        });

    let device_info = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let fingerprint_hash = {
        let accept_language = headers
            .get("accept-language")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let partial_ip = ip_address
            .as_ref()
            .map(|ip| {
                let parts: Vec<&str> = ip.split('.').take(3).collect();
                if parts.len() == 3 {
                    parts.join(".")
                } else {
                    ip.split(':').next().unwrap_or("unknown").to_string()
                }
            })
            .unwrap_or_else(|| "unknown".to_string());

        let fingerprint_data = format!(
            "{}|{}|{}",
            device_info.as_deref().unwrap_or(""),
            accept_language,
            partial_ip
        );

        let mut hasher = Sha256::new();
        hasher.update(fingerprint_data.as_bytes());
        hex::encode(hasher.finalize())
    };

    // Generate JWT
    let token = generate_token_with_fingerprint(
        user.id,
        tenant.id,
        user.role.clone(),
        Some(fingerprint_hash.clone()),
    )
    .map_err(|e| {
        tracing::error!("Token generation error: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Token generation failed".to_string(),
        )
    })?;

    // Create session record
    let token_hash = {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    };

    let _ = store.sso().upsert_session(user.id,&token_hash,device_info.as_deref(),ip_address.as_deref(),&fingerprint_hash).await;

    // Track login IP for security
    let _ = security_service::check_and_record_login_ip(
        store,
        user.id,
        tenant.id,
        ip_address.as_deref(),
        device_info.as_deref(),
        &user.email,
    )
    .await;

    tracing::info!(
        user_id = %user.id,
        email = %user.email,
        provider = %config.provider_name,
        "SSO login successful"
    );

    Ok(SsoSessionResult::Token(token))
}
