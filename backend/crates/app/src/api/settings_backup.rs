//! Settings Backup, Export/Import, and Settings Profiles
//!
//! Provides comprehensive backup functionality:
//! - Export all tenant settings, users, departments, roles as encrypted JSON
//! - Import/restore from backup with dry-run preview
//! - Apply partial settings profiles (NixOS-style declarative config)
//! - Global settings export/import (SuperAdmin only)
//!
//! Security:
//! - All backups encrypted with ChaCha20-Poly1305 (passphrase-derived key via Argon2id)
//! - Password re-confirmation required for all operations
//! - Sensitive fields (passwords, API keys) redacted by default
//! - Rate-limited exports/imports, brute-force detection on decrypt
//! - Full audit logging of all backup operations

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Json, Response},
    Extension,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use argon2::Argon2;
use base64::Engine;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;

use crate::health::CURRENT_VERSION;
use crate::AppState;
use crate::auth::AuthUser;
use app_entity::entities::*;
use app_core::circuit_breaker::CircuitState;
use app_core::security_service::{self, AlertType};

// ============================================================================
// CONSTANTS
// ============================================================================


const REDACTED: &str = "***REDACTED***";
const NONCE_SIZE: usize = 12;
const SALT_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
const MAX_BACKUP_SIZE: usize = 50 * 1024 * 1024; // 50MB

/// Fields that are NEVER exported (even with include_secrets)
const ALWAYS_REDACTED: &[&str] = &["password_hash", "totp_secret", "recovery_token"];

/// Prefix for encrypted-at-rest values (to distinguish from plaintext)
const ENCRYPTED_PREFIX: &str = "enc:";

/// Valid permission names for import validation
const VALID_PERMISSIONS: &[&str] = &[
    "files.read",
    "files.write",
    "files.delete",
    "files.share",
    "requests.read",
    "requests.write",
    "requests.manage",
    "users.read",
    "users.write",
    "users.manage",
    "roles.read",
    "roles.write",
    "audit.read",
    "audit.manage",
    "settings.read",
    "settings.write",
    "tenants.read",
    "tenants.manage",
    "approvals.view",
    "approvals.manage",
];

async fn upsert_global_setting<C: sea_orm::ConnectionTrait>(
    db: &C,
    key: &str,
    value: Value,
    updated_by: Option<Uuid>,
) -> Result<(), StatusCode> {
    let now = Some(chrono::Utc::now().into());
    if let Some(existing) = app_entity::global_settings::Entity::find_by_id(key)
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let mut active: app_entity::global_settings::ActiveModel = existing.into();
        active.value = sea_orm::Set(value);
        if let Some(uid) = updated_by {
            active.updated_by = sea_orm::Set(Some(uid));
        }
        active.updated_at = sea_orm::Set(now);
        active.update(db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        let active = app_entity::global_settings::ActiveModel {
            key: sea_orm::Set(key.to_string()),
            value: sea_orm::Set(value),
            updated_by: sea_orm::Set(updated_by),
            updated_at: sea_orm::Set(now),
        };
        active.insert(db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(())
}

async fn update_global_email_template<C: sea_orm::ConnectionTrait>(
    db: &C,
    template_key: &str,
    subject: Option<&str>,
    body_html: Option<&str>,
    body_text: Option<&str>,
) -> Result<bool, StatusCode> {
    if let Some(existing) = app_entity::email_templates::Entity::find()
        .filter(app_entity::email_templates::Column::TemplateKey.eq(template_key))
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let mut active: app_entity::email_templates::ActiveModel = existing.into();
        if let Some(s) = subject { active.subject = sea_orm::Set(s.to_string()); }
        if let Some(h) = body_html { active.body_html = sea_orm::Set(h.to_string()); }
        if let Some(t) = body_text { active.body_text = sea_orm::Set(Some(t.to_string())); }
        active.updated_at = sea_orm::Set(Some(chrono::Utc::now().into()));
        active.update(db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(true)
    } else {
        Ok(false)
    }
}


/// Derive a proper key from BACKUP_MASTER_KEY using Argon2id (instead of direct byte-copy)
fn derive_master_key_bytes(master_key: &[u8]) -> [u8; KEY_SIZE] {
    let params = argon2::Params::new(65536, 4, 4, Some(KEY_SIZE)).expect("valid Argon2 params");
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut key = [0u8; KEY_SIZE];
    argon2
        .hash_password_into(master_key, b"CLOVALINK_MASTER_KEY_SALT", &mut key)
        .expect("Argon2 master key derivation failed");
    key
}

/// Check if BACKUP_MASTER_KEY is configured and valid (≥32 chars)
pub(crate) fn is_master_key_configured() -> bool {
    types::config::get_config()
        .backup
        .master_key
        .as_ref()
        .is_some_and(|key| key.len() >= 32)
}

/// Normalize a cron expression to 6-field format (with seconds) for the `cron` crate.
/// Standard 5-field expressions like "0 2 * * *" become "0 0 2 * * *".
fn normalize_cron(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() == 5 {
        format!("0 {}", expr.trim())
    } else {
        expr.to_string()
    }
}

/// Encrypt a passphrase for at-rest storage using BACKUP_MASTER_KEY env var.
/// Returns "enc:<base64(nonce + ciphertext)>" if master key is set, or plaintext if not.
fn encrypt_passphrase_at_rest(passphrase: &str) -> String {
    let master_key = match types::config::get_config().backup.master_key.as_deref() {
        Some(k) if k.len() >= 32 => k,
        _ => {
            tracing::warn!("BACKUP_MASTER_KEY not set or too short — storing passphrase without at-rest encryption");
            return passphrase.to_string();
        }
    };

    let key_bytes = derive_master_key_bytes(master_key.as_bytes());

    let cipher = ChaCha20Poly1305::new((&key_bytes).into());
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    match cipher.encrypt(nonce, passphrase.as_bytes()) {
        Ok(ciphertext) => {
            let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
            combined.extend_from_slice(&nonce_bytes);
            combined.extend_from_slice(&ciphertext);
            format!(
                "{}{}",
                ENCRYPTED_PREFIX,
                base64::engine::general_purpose::STANDARD.encode(combined)
            )
        }
        Err(e) => {
            tracing::error!("Failed to encrypt passphrase at rest: {:?}", e);
            passphrase.to_string()
        }
    }
}

/// Decrypt a passphrase from at-rest storage. Handles both encrypted ("enc:...") and legacy plaintext.
fn decrypt_passphrase_at_rest(stored: &str) -> Result<String, &'static str> {
    if !stored.starts_with(ENCRYPTED_PREFIX) {
        // Legacy plaintext — return as-is
        return Ok(stored.to_string());
    }

    let master_key = types::config::get_config()
        .backup
        .master_key
        .as_deref()
        .ok_or("BACKUP_MASTER_KEY not set — cannot decrypt passphrase")?;
    if master_key.len() < 32 {
        return Err("BACKUP_MASTER_KEY too short");
    }

    let key_bytes = derive_master_key_bytes(master_key.as_bytes());

    let encoded = &stored[ENCRYPTED_PREFIX.len()..];
    let combined = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| "Invalid base64 in encrypted passphrase")?;

    if combined.len() < NONCE_SIZE {
        return Err("Encrypted passphrase too short");
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
    let cipher = ChaCha20Poly1305::new((&key_bytes).into());
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Failed to decrypt passphrase — wrong BACKUP_MASTER_KEY?")?;

    String::from_utf8(plaintext).map_err(|_| "Decrypted passphrase is not valid UTF-8")
}

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Deserialize)]
pub struct ExportParams {
    pub sections: Option<String>,
    pub include_optional: Option<String>,
    pub include_secrets: Option<bool>,
    pub audit_days: Option<i64>,
    pub file_limit: Option<i64>,
    pub approval_days: Option<i64>,
}

#[derive(Deserialize)]
pub struct ImportRequest {
    pub data: String, // base64-encoded encrypted backup
    pub sections: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct SettingsProfileRequest {
    pub profile: Value, // plaintext partial JSON config
    pub dry_run: Option<bool>,
}

#[derive(Deserialize)]
pub struct CurrentSettingsParams {
    pub mode: Option<String>, // "global" or "tenant" (default)
}

#[derive(Deserialize)]
pub struct GlobalToggleRequest {
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct BackupListParams {
    pub mode: Option<String>, // "global" or "tenant" (default)
}

#[derive(Deserialize)]
pub struct GlobalScheduleRequest {
    pub enabled: Option<bool>,
    pub cron: Option<String>,
    pub retention_count: Option<i32>,
}

// ============================================================================
// ENCRYPTION / DECRYPTION
// ============================================================================

/// Encrypt JSON backup data with ChaCha20-Poly1305
/// Key derived from passphrase using Argon2id
fn encrypt_backup(plaintext: &[u8], passphrase: &str) -> Result<Value, StatusCode> {
    // Generate random salt and nonce
    let mut salt = [0u8; SALT_SIZE];
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    // Derive key from passphrase using Argon2id
    let key = derive_key(passphrase, &salt)?;

    // Encrypt with ChaCha20-Poly1305
    let cipher = ChaCha20Poly1305::new((&key).into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext).map_err(|e| {
        tracing::error!("Backup encryption failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(json!({
        "encrypted": true,
        "kdf": "argon2id",
        "salt": base64::engine::general_purpose::STANDARD.encode(salt),
        "nonce": base64::engine::general_purpose::STANDARD.encode(nonce_bytes),
        "data": base64::engine::general_purpose::STANDARD.encode(ciphertext)
    }))
}

/// Decrypt backup data
fn decrypt_backup(encrypted: &Value, passphrase: &str) -> Result<Vec<u8>, &'static str> {
    use base64::Engine;

    let salt_b64 = encrypted
        .get("salt")
        .and_then(|v| v.as_str())
        .ok_or("Missing salt")?;
    let nonce_b64 = encrypted
        .get("nonce")
        .and_then(|v| v.as_str())
        .ok_or("Missing nonce")?;
    let data_b64 = encrypted
        .get("data")
        .and_then(|v| v.as_str())
        .ok_or("Missing data")?;

    let salt = base64::engine::general_purpose::STANDARD
        .decode(salt_b64)
        .map_err(|_| "Invalid salt")?;
    let nonce_bytes = base64::engine::general_purpose::STANDARD
        .decode(nonce_b64)
        .map_err(|_| "Invalid nonce")?;
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .map_err(|_| "Invalid data")?;

    if salt.len() != SALT_SIZE {
        return Err("Invalid salt length");
    }
    if nonce_bytes.len() != NONCE_SIZE {
        return Err("Invalid nonce length");
    }

    // Derive key
    let key = derive_key(passphrase, &salt).map_err(|_| "Key derivation failed")?;

    // Decrypt
    let cipher = ChaCha20Poly1305::new((&key).into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Invalid passphrase")
}

/// Derive a 256-bit key from passphrase using Argon2id
fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_SIZE], StatusCode> {
    let params = argon2::Params::new(65536, 4, 4, Some(KEY_SIZE))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key = [0u8; KEY_SIZE];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| {
            tracing::error!("Argon2 key derivation failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(key)
}

// ============================================================================
// SECURITY HELPERS
// ============================================================================

/// Verify password re-confirmation from X-Confirm-Password header
/// Rate-limited: 5 failures per user per 15 minutes
pub(crate) async fn verify_password_confirmation(
    store: &app_entity::DataStore,
    user_id: Uuid,
    headers: &HeaderMap,
) -> Result<(), StatusCode> {
    // Rate limit: check recent password confirmation failures
    let fifteen_min_ago = Utc::now() - chrono::Duration::minutes(15);
    let fail_count = store
        .security()
        .count_password_confirm_failures(user_id, fifteen_min_ago)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if fail_count >= 5 {
        tracing::warn!("Password confirmation rate limit hit for user {}", user_id);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let password = headers
        .get("X-Confirm-Password")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = store
        .users()
        .user(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let hash = user.password_hash.ok_or(StatusCode::UNAUTHORIZED)?;

    let parsed = argon2::PasswordHash::new(&hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if argon2::PasswordVerifier::verify_password(&Argon2::default(), password.as_bytes(), &parsed)
        .is_err()
    {
        // Record failed attempt as security alert for rate limiting
        let _ = security_service::create_alert(
            store,
            None,
            Some(user_id),
            AlertType::PasswordConfirmFailed,
            "Failed password confirmation for backup operation",
            &format!(
                "Failed password confirmation attempt ({} in 15 min window)",
                fail_count + 1
            ),
            json!({ "attempt_count": fail_count + 1 }),
            None,
        )
        .await;
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(())
}

/// Get backup passphrase from X-Backup-Passphrase header
fn get_passphrase(headers: &HeaderMap) -> Result<String, StatusCode> {
    let passphrase = headers
        .get("X-Backup-Passphrase")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Prevent DoS via extremely long passphrases hitting Argon2id
    if passphrase.len() > 1024 {
        tracing::warn!("Passphrase exceeds max length ({})", passphrase.len());
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(passphrase)
}

/// Check brute-force attempts for backup decrypt
async fn check_and_record_decrypt_failure(
    store: &app_entity::DataStore,
    tenant_id: Uuid,
    user_id: Uuid,
    ip_address: &str,
) -> Result<bool, StatusCode> {
    // Check recent failures
    let fifteen_min_ago = Utc::now() - chrono::Duration::minutes(15);
    let count = store
        .security()
        .count_recent_alerts_by_user(user_id, "backup_decrypt_failed", fifteen_min_ago)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Record the failed attempt
    let _ = security_service::create_alert(
        store,
        Some(tenant_id),
        Some(user_id),
        AlertType::BackupDecryptFailed,
        "Failed backup decrypt attempt",
        &format!(
            "Failed to decrypt backup file (attempt {} in 15 min window)",
            count + 1
        ),
        json!({
            "attempt_count": count + 1,
            "ip_address": ip_address
        }),
        Some(ip_address),
    )
    .await;

    // If 5+ failures, trigger brute-force alert
    if count + 1 >= 5 {
        let _ = security_service::create_alert(
            store,
            Some(tenant_id),
            Some(user_id),
            AlertType::BackupBruteForce,
            "Backup brute-force attempt detected",
            &format!("{} failed backup decrypt attempts in 15 minutes — user locked out of backup operations", count + 1),
            json!({
                "attempt_count": count + 1,
                "ip_address": ip_address,
                "lockout": true
            }),
            Some(ip_address),
        ).await;
        return Ok(true); // locked out
    }

    Ok(false)
}

/// Check if user is locked out from backup operations
async fn is_backup_locked_out(store: &app_entity::DataStore, user_id: Uuid) -> Result<bool, StatusCode> {
    let fifteen_min_ago = Utc::now() - chrono::Duration::minutes(15);
    let count = store
        .security()
        .count_recent_alerts_by_user(user_id, "backup_brute_force", fifteen_min_ago)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(count > 0)
}

/// Log a backup audit event
async fn log_backup_audit(
    store: &app_entity::DataStore,
    tenant_id: Uuid,
    user_id: Uuid,
    action: &str,
    metadata: Value,
    ip_address: &str,
) {
    let _ = store
        .audit()
        .log(
            tenant_id,
            Some(user_id),
            action,
            "backup",
            None,
            Some(metadata),
            Some(ip_address.to_string()),
        )
        .await;
}

// ============================================================================
// PER-TENANT + CIRCUIT BREAKER GUARDS
// ============================================================================

/// Check if backup is enabled for this tenant (SuperAdmin bypasses)
async fn check_backup_enabled(
    store: &app_entity::DataStore,
    tenant_id: Uuid,
    role: &str,
) -> Result<(), StatusCode> {
    if role == "SuperAdmin" {
        return Ok(());
    }
    let tenant = store
        .tenants()
        .tenant(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match tenant {
        Some(t) if t.backup_enabled == Some(false) => Err(StatusCode::FORBIDDEN),
        _ => Ok(()), // default true
    }
}

/// Check circuit breaker and acquire semaphore permit.
/// Returns 503 if circuit is open, 429 if too many concurrent operations.
fn check_backup_infra(state: &AppState) -> Result<tokio::sync::OwnedSemaphorePermit, StatusCode> {
    if !state.backup_circuit_breaker.allow_request() {
        tracing::warn!("Backup circuit breaker is open — rejecting request");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    state
        .backup_semaphore
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            tracing::warn!("Backup concurrency limit reached — rejecting request");
            StatusCode::TOO_MANY_REQUESTS
        })
}

// ============================================================================
// SECTION COLLECTORS (EXPORT)
// ============================================================================

fn redact_value(obj: &mut Value, field: &str, include_secrets: bool) {
    if let Some(map) = obj.as_object_mut() {
        if let Some(val) = map.get_mut(field) {
            if ALWAYS_REDACTED.contains(&field) || !include_secrets {
                *val = Value::String(REDACTED.to_string());
            }
        }
    }
}

async fn collect_tenant_core(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    include_secrets: bool,
) -> Result<Value, StatusCode> {
    let tenant = tenants::Entity::find_by_id(tenant_id)
        .one(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect tenant core: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut result = json!({
        "compliance_mode": tenant.compliance_mode,
        "encryption_standard": tenant.encryption_standard,
        "retention_policy_days": tenant.retention_policy_days,
        "mfa_required": tenant.mfa_required,
        "session_timeout_minutes": tenant.session_timeout_minutes,
        "public_sharing_enabled": tenant.public_sharing_enabled,
        "data_export_enabled": tenant.data_export_enabled,
        "blocked_extensions": tenant.blocked_extensions,
        "password_policy": tenant.password_policy,
        "ip_restriction_mode": tenant.ip_restriction_mode,
        "ip_allowlist": tenant.ip_allowlist,
        "ip_blocklist": tenant.ip_blocklist,
        "storage_quota_bytes": tenant.storage_quota_bytes,
        "max_upload_size_bytes": tenant.max_upload_size_bytes,
        "enable_totp": tenant.enable_totp,
        "enable_passkeys": tenant.enable_passkeys,
        "auth_methods": tenant.auth_methods,
        "approval_workflow_enabled": tenant.approval_workflow_enabled,
        "smtp_host": tenant.smtp_host,
        "smtp_port": tenant.smtp_port,
        "smtp_username": tenant.smtp_username,
        "smtp_password": tenant.smtp_password,
        "smtp_from": tenant.smtp_from,
        "smtp_secure": tenant.smtp_secure,
    });

    redact_value(&mut result, "smtp_password", include_secrets);
    Ok(result)
}

async fn collect_users(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    _include_secrets: bool,
) -> Result<Value, StatusCode> {
    let users_list = users::Entity::find()
        .filter(users::Column::TenantId.eq(tenant_id))
        .order_by_asc(users::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect users: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let rows: Vec<Value> = users_list
        .into_iter()
        .map(|u| {
            json!({
                "email": u.email,
                "name": u.name,
                "role": u.role,
                "status": u.status,
                "department_id": u.department_id,
                "custom_role_id": u.custom_role_id,
                "identity_provider": u.identity_provider,
                "avatar_url": u.avatar_url,
                "allowed_tenant_ids": u.allowed_tenant_ids,
                "allowed_department_ids": u.allowed_department_ids,
                "password_changed_at": u.password_changed_at,
                "suspended_at": u.suspended_at,
                "suspended_until": u.suspended_until,
                "suspension_reason": u.suspension_reason,
                "dashboard_layout": u.dashboard_layout,
                "widget_config": u.widget_config,
                "last_active_at": u.last_active_at,
                "created_at": u.created_at,
                "updated_at": u.updated_at,
            })
        })
        .collect();

    Ok(Value::Array(rows))
}

async fn collect_departments(db: &DatabaseConnection, tenant_id: Uuid) -> Result<Value, StatusCode> {
    let depts = departments::Entity::find()
        .filter(departments::Column::TenantId.eq(tenant_id))
        .order_by_asc(departments::Column::Name)
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect departments: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let rows: Vec<Value> = depts
        .into_iter()
        .map(|d| {
            json!({
                "name": d.name,
                "description": d.description,
                "created_at": d.created_at,
            })
        })
        .collect();

    Ok(Value::Array(rows))
}

async fn collect_roles(db: &DatabaseConnection, tenant_id: Uuid) -> Result<Value, StatusCode> {
    let roles_list = roles::Entity::find()
        .filter(roles::Column::TenantId.eq(tenant_id))
        .order_by_asc(roles::Column::Name)
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect roles: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut rows = Vec::with_capacity(roles_list.len());
    for r in roles_list {
        let perms = role_permissions::Entity::find()
            .filter(role_permissions::Column::RoleId.eq(r.id))
            .all(db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to collect role permissions: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        let perms_val: Vec<Value> = perms
            .into_iter()
            .map(|p| {
                json!({
                    "permission": p.permission,
                    "granted": p.granted,
                })
            })
            .collect();
        rows.push(json!({
            "name": r.name,
            "description": r.description,
            "base_role": r.base_role,
            "is_system": r.is_system,
            "permissions": perms_val,
        }));
    }

    Ok(Value::Array(rows))
}

async fn collect_audit_settings(db: &DatabaseConnection, tenant_id: Uuid) -> Result<Value, StatusCode> {
    let row = audit_settings::Entity::find()
        .filter(audit_settings::Column::TenantId.eq(tenant_id))
        .into_json()
        .one(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect audit settings: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(row.unwrap_or(json!({
        "log_logins": true,
        "log_file_operations": true,
        "log_user_changes": true,
        "log_settings_changes": true,
        "log_role_changes": true,
        "retention_days": 90
    })))
}

async fn collect_virus_scan(db: &DatabaseConnection, tenant_id: Uuid) -> Result<Value, StatusCode> {
    let row = virus_scan_settings::Entity::find()
        .filter(virus_scan_settings::Column::TenantId.eq(tenant_id))
        .into_json()
        .one(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect virus scan: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(row.unwrap_or(Value::Null))
}

async fn collect_ai_settings(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    include_secrets: bool,
) -> Result<Value, StatusCode> {
    let row = tenant_ai_settings::Entity::find()
        .filter(tenant_ai_settings::Column::TenantId.eq(tenant_id))
        .into_json()
        .one(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect ai settings: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match row {
        Some(mut v) => {
            redact_value(&mut v, "api_key_encrypted", include_secrets);
            Ok(v)
        }
        None => Ok(Value::Null),
    }
}

async fn collect_discord_settings(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    _include_secrets: bool,
) -> Result<Value, StatusCode> {
    let row = tenant_discord_settings::Entity::find()
        .filter(tenant_discord_settings::Column::TenantId.eq(tenant_id))
        .into_json()
        .one(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect discord settings: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(row.unwrap_or(Value::Null))
}

async fn collect_sso_oidc(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    include_secrets: bool,
) -> Result<Value, StatusCode> {
    let list = tenant_oidc_providers::Entity::find()
        .filter(tenant_oidc_providers::Column::TenantId.eq(tenant_id))
        .order_by_asc(tenant_oidc_providers::Column::Name)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect sso oidc: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let rows: Vec<Value> = list
        .into_iter()
        .map(|mut v| {
            redact_value(&mut v, "client_secret_encrypted", include_secrets);
            v
        })
        .collect();

    Ok(Value::Array(rows))
}

async fn collect_sso_saml(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    include_secrets: bool,
) -> Result<Value, StatusCode> {
    let list = tenant_saml_providers::Entity::find()
        .filter(tenant_saml_providers::Column::TenantId.eq(tenant_id))
        .order_by_asc(tenant_saml_providers::Column::Name)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect sso saml: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let rows: Vec<Value> = list
        .into_iter()
        .map(|mut v| {
            redact_value(&mut v, "sp_signing_key_encrypted", include_secrets);
            v
        })
        .collect();

    Ok(Value::Array(rows))
}

async fn collect_sso_identities(db: &DatabaseConnection, tenant_id: Uuid) -> Result<Value, StatusCode> {
    let tenant_users = users::Entity::find()
        .filter(users::Column::TenantId.eq(tenant_id))
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to query tenant users for sso identities: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let user_map: HashMap<Uuid, String> = tenant_users
        .into_iter()
        .map(|u| (u.id, u.email))
        .collect();

    let user_ids: Vec<Uuid> = user_map.keys().copied().collect();

    let oidc_identities = if !user_ids.is_empty() {
        user_oidc_identities::Entity::find()
            .filter(user_oidc_identities::Column::UserId.is_in(user_ids.clone()))
            .all(db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to collect oidc identities: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        Vec::new()
    };

    let saml_identities = if !user_ids.is_empty() {
        user_saml_identities::Entity::find()
            .filter(user_saml_identities::Column::UserId.is_in(user_ids))
            .all(db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to collect saml identities: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        Vec::new()
    };

    let oidc_val: Vec<Value> = oidc_identities
        .into_iter()
        .map(|oi| {
            let email = user_map.get(&oi.user_id).cloned().unwrap_or_default();
            json!({
                "user_email": email,
                "oidc_subject": oi.oidc_subject,
                "oidc_issuer": oi.oidc_issuer,
                "oidc_email": oi.oidc_email,
                "oidc_name": oi.oidc_name,
                "login_count": oi.login_count,
            })
        })
        .collect();

    let saml_val: Vec<Value> = saml_identities
        .into_iter()
        .map(|si| {
            let email = user_map.get(&si.user_id).cloned().unwrap_or_default();
            json!({
                "user_email": email,
                "saml_name_id": si.saml_name_id,
                "saml_name_id_format": si.saml_name_id_format,
                "saml_email": si.saml_email,
                "saml_name": si.saml_name,
                "login_count": si.login_count,
            })
        })
        .collect();

    Ok(json!({
        "oidc": oidc_val,
        "saml": saml_val
    }))
}

async fn collect_sso_mappings(db: &DatabaseConnection, tenant_id: Uuid) -> Result<Value, StatusCode> {
    let list = sso_attribute_mappings::Entity::find()
        .filter(sso_attribute_mappings::Column::TenantId.eq(tenant_id))
        .order_by_asc(sso_attribute_mappings::Column::Priority)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect sso mappings: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Value::Array(list))
}

async fn collect_approval_policies(
    db: &DatabaseConnection,
    tenant_id: Uuid,
) -> Result<Value, StatusCode> {
    let list = approval_policies::Entity::find()
        .filter(approval_policies::Column::TenantId.eq(tenant_id))
        .order_by_asc(approval_policies::Column::Name)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect approval policies: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Value::Array(list))
}

async fn collect_email_templates(
    db: &DatabaseConnection,
    tenant_id: Uuid,
) -> Result<Value, StatusCode> {
    let list = tenant_email_templates::Entity::find()
        .filter(tenant_email_templates::Column::TenantId.eq(tenant_id))
        .order_by_asc(tenant_email_templates::Column::TemplateKey)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect email templates: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Value::Array(list))
}

async fn collect_notification_settings(
    db: &DatabaseConnection,
    tenant_id: Uuid,
) -> Result<Value, StatusCode> {
    let list = tenant_notification_settings::Entity::find()
        .filter(tenant_notification_settings::Column::TenantId.eq(tenant_id))
        .order_by_asc(tenant_notification_settings::Column::EventType)
        .order_by_asc(tenant_notification_settings::Column::Role)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect notification settings: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Value::Array(list))
}

// Optional large sections

async fn collect_file_metadata(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: i64,
) -> Result<Value, StatusCode> {
    let limit = limit.clamp(1, 100_000) as u64;
    let files = files_metadata::Entity::find()
        .filter(files_metadata::Column::TenantId.eq(tenant_id))
        .order_by_asc(files_metadata::Column::CreatedAt)
        .limit(limit)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect file metadata: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let shares = file_shares::Entity::find()
        .filter(file_shares::Column::TenantId.eq(tenant_id))
        .order_by_asc(file_shares::Column::CreatedAt)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect file shares: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(json!({
        "files": files,
        "shares": shares
    }))
}

async fn collect_audit_logs(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    days: i64,
) -> Result<Value, StatusCode> {
    let days = days.clamp(1, 3650);
    let since = Utc::now() - chrono::Duration::days(days);
    let rows = audit_logs::Entity::find()
        .filter(audit_logs::Column::TenantId.eq(tenant_id))
        .filter(audit_logs::Column::CreatedAt.gt(since))
        .order_by_asc(audit_logs::Column::CreatedAt)
        .limit(500_000)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect audit logs: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Value::Array(rows))
}

async fn collect_approval_history(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    days: i64,
) -> Result<Value, StatusCode> {
    let days = days.clamp(1, 3650);
    let since = Utc::now() - chrono::Duration::days(days);
    let rows = approval_requests::Entity::find()
        .filter(approval_requests::Column::TenantId.eq(tenant_id))
        .filter(approval_requests::Column::CreatedAt.gt(since))
        .order_by_asc(approval_requests::Column::CreatedAt)
        .limit(500_000)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect approval history: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Value::Array(rows))
}

/// Keys that must never appear in exported backup data
const SENSITIVE_GLOBAL_KEYS: &[&str] = &["auto_backup_passphrase"];

/// Patterns in key names that indicate sensitive data
const SENSITIVE_KEY_PATTERNS: &[&str] = &["secret", "password", "key", "token", "encrypted"];

/// Check if a global settings key is sensitive (exact match or pattern)
fn is_sensitive_key(key: &str) -> bool {
    if SENSITIVE_GLOBAL_KEYS.contains(&key) {
        return true;
    }
    let lower = key.to_lowercase();
    SENSITIVE_KEY_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Strip sensitive keys from a global_settings JSON object before export
fn strip_sensitive_keys(mut settings: Value) -> Value {
    if let Some(map) = settings.as_object_mut() {
        let sensitive_keys: Vec<String> = map
            .keys()
            .filter(|k| is_sensitive_key(k))
            .cloned()
            .collect();
        for key in sensitive_keys {
            map.remove(&key);
        }
    }
    settings
}

// Global settings collector
async fn collect_global_settings(store: &app_entity::DataStore) -> Result<Value, StatusCode> {
    let settings = store
        .global_settings()
        .all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut map = serde_json::Map::new();
    for item in settings {
        map.insert(item.key, item.value);
    }
    Ok(Value::Object(map))
}

async fn collect_global_email_templates(db: &DatabaseConnection) -> Result<Value, StatusCode> {
    let rows = email_templates::Entity::find()
        .order_by_asc(email_templates::Column::TemplateKey)
        .into_json()
        .all(db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to collect global email templates: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Value::Array(rows))
}

pub async fn export_tenant_backup(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<ExportParams>,
) -> Result<Response<Body>, StatusCode> {
    // Auth: Admin+ only
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Per-tenant check + circuit breaker + concurrency
    check_backup_enabled(&state.store, auth.tenant_id, &auth.role).await?;
    let _permit = check_backup_infra(&state)?;

    // Check lockout
    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    // Verify password
    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;

    // Get passphrase
    let passphrase = get_passphrase(&headers)?;
    if passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // include_secrets requires SuperAdmin
    let include_secrets = params.include_secrets.unwrap_or(false);
    if include_secrets && auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    // Pagination params for large sections
    let audit_days = params.audit_days.unwrap_or(90);
    let file_limit = params.file_limit.unwrap_or(50000);
    let approval_days = params.approval_days.unwrap_or(90);

    // Parse requested sections
    let valid_tenant_sections: &[&str] = &[
        "tenant_core",
        "users",
        "departments",
        "roles",
        "settings_audit",
        "settings_virus_scan",
        "settings_ai",
        "settings_discord",
        "sso_oidc",
        "sso_saml",
        "sso_mappings",
        "sso_identities",
        "approval_policies",
        "email_templates",
        "notification_settings",
    ];
    let valid_optional_sections: &[&str] = &["file_metadata", "audit_logs", "approval_history"];
    let sections: Vec<String> = params
        .sections
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(|| {
            valid_tenant_sections
                .iter()
                .map(|s| s.to_string())
                .collect()
        });

    // Reject unknown sections
    for s in &sections {
        if !valid_tenant_sections.contains(&s.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let optional: Vec<String> = params
        .include_optional
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    for s in &optional {
        if !valid_optional_sections.contains(&s.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Get tenant name
    let tenant_name = state
        .store
        .tenants()
        .tenant(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|t| t.name)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Build backup JSON
    let mut backup = json!({
        "_meta": {
            "format": "clovalink-backup",
            "format_version": 1,
            "clovalink_version": CURRENT_VERSION,
            "export_type": "tenant",
            "tenant_id": auth.tenant_id.to_string(),
            "tenant_name": tenant_name,
            "exported_at": Utc::now().to_rfc3339(),
            "exported_by": auth.user_id.to_string(),
            "include_secrets": include_secrets,
            "sections": &sections
        }
    });

    let backup_map = backup.as_object_mut().unwrap();

    // Collect each section
    for section in &sections {
        let value = match section.as_str() {
            "tenant_core" => {
                collect_tenant_core(state.store.db(), auth.tenant_id, include_secrets).await?
            }
            "users" => collect_users(state.store.db(), auth.tenant_id, include_secrets).await?,
            "departments" => collect_departments(state.store.db(), auth.tenant_id).await?,
            "roles" => collect_roles(state.store.db(), auth.tenant_id).await?,
            "settings_audit" => collect_audit_settings(state.store.db(), auth.tenant_id).await?,
            "settings_virus_scan" => collect_virus_scan(state.store.db(), auth.tenant_id).await?,
            "settings_ai" => {
                collect_ai_settings(state.store.db(), auth.tenant_id, include_secrets).await?
            }
            "settings_discord" => {
                collect_discord_settings(state.store.db(), auth.tenant_id, include_secrets).await?
            }
            "sso_oidc" => collect_sso_oidc(state.store.db(), auth.tenant_id, include_secrets).await?,
            "sso_saml" => collect_sso_saml(state.store.db(), auth.tenant_id, include_secrets).await?,
            "sso_mappings" => collect_sso_mappings(state.store.db(), auth.tenant_id).await?,
            "sso_identities" => collect_sso_identities(state.store.db(), auth.tenant_id).await?,
            "approval_policies" => collect_approval_policies(state.store.db(), auth.tenant_id).await?,
            "email_templates" => collect_email_templates(state.store.db(), auth.tenant_id).await?,
            "notification_settings" => {
                collect_notification_settings(state.store.db(), auth.tenant_id).await?
            }
            _ => continue,
        };
        backup_map.insert(section.clone(), value);
    }

    // Collect optional sections (with pagination limits)
    for section in &optional {
        let value = match section.as_str() {
            "file_metadata" => {
                collect_file_metadata(state.store.db(), auth.tenant_id, file_limit).await?
            }
            "audit_logs" => collect_audit_logs(state.store.db(), auth.tenant_id, audit_days).await?,
            "approval_history" => {
                collect_approval_history(state.store.db(), auth.tenant_id, approval_days).await?
            }
            _ => continue,
        };
        backup_map.insert(section.clone(), value);
    }

    // Encrypt the backup
    let plaintext = serde_json::to_vec(&backup).map_err(|e| {
        state.backup_circuit_breaker.record_failure();
        tracing::error!("Backup serialization failed: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let encrypted = encrypt_backup(&plaintext, &passphrase).map_err(|e| {
        state.backup_circuit_breaker.record_failure();
        e
    })?;
    let encrypted_bytes =
        serde_json::to_vec_pretty(&encrypted).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.backup_circuit_breaker.record_success();

    // Audit log
    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_export",
        json!({
            "sections": &sections,
            "optional_sections": &optional,
            "include_secrets": include_secrets,
            "size_bytes": encrypted_bytes.len()
        }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    // Security alert if secrets were included
    if include_secrets {
        let _ = security_service::create_alert(
            &state.store,
            Some(auth.tenant_id),
            Some(auth.user_id),
            AlertType::BackupExportSecrets,
            "Backup exported with secrets",
            &format!(
                "User exported backup including encrypted secrets for tenant {}",
                tenant_name
            ),
            json!({
                "sections": &sections,
                "tenant_name": tenant_name
            }),
            auth.ip_address.as_deref(),
        )
        .await;
    }

    // Build response with download headers
    let filename = format!(
        "clovalink-backup-{}-{}-{:06x}.clovalink.json",
        tenant_name.to_lowercase().replace(' ', "-"),
        Utc::now().format("%Y%m%d-%H%M%S"),
        rand::random::<u32>() & 0xFFFFFF
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .header("X-Content-Type-Options", "nosniff")
        .body(Body::from(encrypted_bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// GET /api/backup/global/export
/// Export global settings as encrypted download (SuperAdmin only)
pub async fn export_global(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<ExportParams>,
) -> Result<Response<Body>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    check_global_backup_enabled(&state.store).await?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;
    let passphrase = get_passphrase(&headers)?;
    if passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let valid_global = ["global_settings", "global_email_templates"];
    let selected: Vec<String> = params
        .sections
        .as_ref()
        .map(|s| {
            s.split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| valid_global.iter().map(|s| s.to_string()).collect());

    // Reject unknown sections
    for s in &selected {
        if !valid_global.contains(&s.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let mut backup = json!({
        "_meta": {
            "format": "clovalink-backup",
            "format_version": 1,
            "clovalink_version": CURRENT_VERSION,
            "export_type": "global",
            "exported_at": Utc::now().to_rfc3339(),
            "exported_by": auth.user_id.to_string(),
            "sections": &selected
        }
    });

    let backup_map = backup.as_object_mut().unwrap();
    for section in &selected {
        let value = match section.as_str() {
            "global_settings" => strip_sensitive_keys(collect_global_settings(&state.store).await?),
            "global_email_templates" => collect_global_email_templates(state.store.db()).await?,
            _ => continue,
        };
        backup_map.insert(section.clone(), value);
    }

    let plaintext = serde_json::to_vec(&backup).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let encrypted = encrypt_backup(&plaintext, &passphrase)?;
    let encrypted_bytes =
        serde_json::to_vec_pretty(&encrypted).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_export_global",
        json!({ "size_bytes": encrypted_bytes.len() }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    let filename = format!(
        "clovalink-global-backup-{}-{:06x}.clovalink.json",
        Utc::now().format("%Y%m%d-%H%M%S"),
        rand::random::<u32>() & 0xFFFFFF
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .header("X-Content-Type-Options", "nosniff")
        .body(Body::from(encrypted_bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ============================================================================
// IMPORT / PREVIEW HANDLERS
// ============================================================================

/// POST /api/backup/import/preview
/// Dry-run import showing what would change
pub async fn preview_import(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<ImportRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Per-tenant check + circuit breaker
    check_backup_enabled(&state.store, auth.tenant_id, &auth.role).await?;
    let _permit = check_backup_infra(&state)?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;
    let passphrase = get_passphrase(&headers)?;
    if passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Size check
    if body.data.len() > MAX_BACKUP_SIZE {
        return Ok(Json(
            json!({ "valid": false, "errors": ["Backup file exceeds 50MB limit"] }),
        ));
    }

    // Parse and decrypt
    let encrypted: Value = serde_json::from_str(&body.data).map_err(|_| StatusCode::BAD_REQUEST)?;

    let plaintext = match decrypt_backup(&encrypted, &passphrase) {
        Ok(data) => data,
        Err(_) => {
            let locked = check_and_record_decrypt_failure(
                &state.store,
                auth.tenant_id,
                auth.user_id,
                auth.ip_address.as_deref().unwrap_or("unknown"),
            )
            .await?;
            if locked {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            return Ok(Json(
                json!({ "valid": false, "errors": ["Invalid passphrase"] }),
            ));
        }
    };

    let backup: Value = serde_json::from_slice(&plaintext).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Validate format
    let meta = backup.get("_meta").ok_or(StatusCode::BAD_REQUEST)?;
    let format = meta.get("format").and_then(|v| v.as_str()).unwrap_or("");
    if format != "clovalink-backup" {
        return Ok(Json(
            json!({ "valid": false, "errors": ["Invalid backup format"] }),
        ));
    }

    let format_version = meta
        .get("format_version")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if format_version != 1 {
        return Ok(Json(
            json!({ "valid": false, "errors": [format!("Unsupported backup version: {}", format_version)] }),
        ));
    }

    // Build preview of changes per section
    let mut sections_preview: HashMap<String, Value> = HashMap::new();
    let mut warnings = Vec::new();

    let selected_sections = body.sections.clone().unwrap_or_else(|| {
        meta.get("sections")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    });

    for section in &selected_sections {
        if let Some(section_data) = backup.get(section) {
            match section.as_str() {
                "users" => {
                    if let Some(users) = section_data.as_array() {
                        let mut added = 0;
                        let mut updated = 0;
                        for user in users {
                            if let Some(email) = user.get("email").and_then(|v| v.as_str()) {
                                let count = app_entity::users::Entity::find()
                                    .filter(app_entity::users::Column::Email.eq(email))
                                    .filter(app_entity::users::Column::TenantId.eq(auth.tenant_id))
                                    .count(state.store.db())
                                    .await
                                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                                if count > 0 {
                                    updated += 1;
                                } else {
                                    added += 1;
                                }
                            }
                        }
                        sections_preview.insert(
                            section.clone(),
                            json!({
                                "total": users.len(),
                                "new_users": added,
                                "existing_users_updated": updated
                            }),
                        );
                    }
                }
                "departments" => {
                    if let Some(depts) = section_data.as_array() {
                        let mut added = 0;
                        let mut existing = 0;
                        for dept in depts {
                            if let Some(name) = dept.get("name").and_then(|v| v.as_str()) {
                                let count = app_entity::departments::Entity::find()
                                    .filter(app_entity::departments::Column::Name.eq(name))
                                    .filter(app_entity::departments::Column::TenantId.eq(auth.tenant_id))
                                    .count(state.store.db())
                                    .await
                                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                                if count > 0 {
                                    existing += 1;
                                } else {
                                    added += 1;
                                }
                            }
                        }
                        sections_preview.insert(
                            section.clone(),
                            json!({
                                "total": depts.len(),
                                "new": added,
                                "existing": existing
                            }),
                        );
                    }
                }
                "roles" => {
                    if let Some(roles) = section_data.as_array() {
                        sections_preview.insert(section.clone(), json!({
                            "total": roles.len(),
                            "note": "Custom roles will be matched by name; permissions will be replaced"
                        }));
                    }
                }
                "tenant_core" => {
                    // Show which fields would change
                    let current = collect_tenant_core(state.store.db(), auth.tenant_id, false).await?;
                    let mut changes = Vec::new();
                    if let (Some(cur_map), Some(new_map)) =
                        (current.as_object(), section_data.as_object())
                    {
                        for (key, new_val) in new_map {
                            if ALWAYS_REDACTED.contains(&key.as_str()) {
                                continue;
                            }
                            if new_val.as_str() == Some(REDACTED) {
                                continue;
                            }
                            if let Some(cur_val) = cur_map.get(key) {
                                if cur_val != new_val {
                                    changes.push(json!({
                                        "field": key,
                                        "current": cur_val,
                                        "new": new_val
                                    }));
                                }
                            }
                        }
                    }
                    sections_preview.insert(
                        section.clone(),
                        json!({
                            "changes": changes,
                            "change_count": changes.len()
                        }),
                    );
                }
                _ => {
                    // Generic: just show count
                    if let Some(arr) = section_data.as_array() {
                        sections_preview.insert(
                            section.clone(),
                            json!({
                                "items": arr.len()
                            }),
                        );
                    } else {
                        sections_preview.insert(
                            section.clone(),
                            json!({
                                "has_data": !section_data.is_null()
                            }),
                        );
                    }
                }
            }
        } else {
            warnings.push(format!("Section '{}' not found in backup file", section));
        }
    }

    Ok(Json(json!({
        "valid": true,
        "meta": meta,
        "sections": sections_preview,
        "warnings": warnings,
        "errors": []
    })))
}

/// POST /api/backup/import
/// Import tenant backup (full restore)
pub async fn import_tenant_backup(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<ImportRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Per-tenant check + circuit breaker
    check_backup_enabled(&state.store, auth.tenant_id, &auth.role).await?;
    let _permit = check_backup_infra(&state)?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;
    let passphrase = get_passphrase(&headers)?;
    if passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    if body.data.len() > MAX_BACKUP_SIZE {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Parse and decrypt
    let encrypted: Value = serde_json::from_str(&body.data).map_err(|_| StatusCode::BAD_REQUEST)?;

    let plaintext = match decrypt_backup(&encrypted, &passphrase) {
        Ok(data) => data,
        Err(_) => {
            let locked = check_and_record_decrypt_failure(
                &state.store,
                auth.tenant_id,
                auth.user_id,
                auth.ip_address.as_deref().unwrap_or("unknown"),
            )
            .await?;
            if locked {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
            return Ok(Json(
                json!({ "success": false, "error": "Invalid passphrase" }),
            ));
        }
    };

    let backup: Value = serde_json::from_slice(&plaintext).map_err(|_| StatusCode::BAD_REQUEST)?;

    let meta = backup.get("_meta").ok_or(StatusCode::BAD_REQUEST)?;
    let format = meta.get("format").and_then(|v| v.as_str()).unwrap_or("");
    if format != "clovalink-backup" {
        return Ok(Json(
            json!({ "success": false, "error": "Invalid backup format" }),
        ));
    }

    let selected_sections = body.sections.clone().unwrap_or_else(|| {
        meta.get("sections")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    });

    // Run import in a transaction
    let tx = state
        .store.db()
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut results: HashMap<String, Value> = HashMap::new();
    let tenant_id = auth.tenant_id;

    for section in &selected_sections {
        if let Some(section_data) = backup.get(section) {
            let result = match section.as_str() {
                "tenant_core" => apply_tenant_core(&tx, tenant_id, section_data).await,
                "departments" => apply_departments(&tx, tenant_id, section_data).await,
                "roles" => apply_roles(&tx, tenant_id, section_data).await,
                "users" => apply_users(&tx, tenant_id, section_data).await,
                "settings_audit" => apply_audit_settings(&tx, tenant_id, section_data).await,
                "settings_virus_scan" => apply_virus_scan(&tx, tenant_id, section_data).await,
                "settings_ai" => apply_ai_settings(&tx, tenant_id, section_data).await,
                "settings_discord" => {
                    apply_discord_settings(&tx, tenant_id, section_data).await
                }
                "approval_policies" => {
                    apply_approval_policies(&tx, tenant_id, section_data).await
                }
                "email_templates" => apply_email_templates(&tx, tenant_id, section_data).await,
                "notification_settings" => {
                    apply_notification_settings(&tx, tenant_id, section_data).await
                }
                _ => Ok(json!({ "skipped": true, "reason": "Section not supported for import" })),
            };

            match result {
                Ok(r) => {
                    results.insert(section.clone(), r);
                }
                Err(e) => {
                    // Rollback on any error
                    let _ = tx.rollback().await;
                    return Ok(Json(json!({
                        "success": false,
                        "error": format!("Failed to import section '{}': {:?}", section, e),
                        "partial_results": results
                    })));
                }
            }
        }
    }

    // Commit transaction
    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit backup import: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Audit log
    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_import",
        json!({
            "sections": &selected_sections,
            "results": &results
        }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    // Invalidate caches
    if let Some(ref cache) = state.cache {
        let _ = cache
            .delete(&app_core::cache::keys::tenant_settings(tenant_id))
            .await;
        let _ = cache
            .delete(&app_core::cache::keys::global_settings())
            .await;
    }

    Ok(Json(json!({
        "success": true,
        "sections_imported": selected_sections,
        "results": results
    })))
}

/// POST /api/backup/apply-profile
/// Apply a partial settings profile (merge semantics)
pub async fn apply_profile(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<ImportRequest>,
) -> Result<Json<Value>, StatusCode> {
    // Same as import but the backup data can be partial
    // Reuse import logic
    import_tenant_backup(State(state), Extension(auth), headers, Json(body)).await
}

/// POST /api/backup/apply-settings-profile
/// Apply a plaintext partial settings profile (NixOS-style declarative merge)
/// SuperAdmin only — no encryption needed, just password confirmation
pub async fn apply_settings_profile(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<SettingsProfileRequest>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    check_backup_enabled(&state.store, auth.tenant_id, &auth.role).await?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;

    let profile = &body.profile;
    let dry_run = body.dry_run.unwrap_or(false);

    // Validate that profile is an object
    if !profile.is_object() {
        return Ok(Json(
            json!({ "success": false, "error": "Profile must be a JSON object" }),
        ));
    }

    let profile_obj = profile.as_object().unwrap();
    if profile_obj.is_empty() {
        return Ok(Json(
            json!({ "success": false, "error": "Profile is empty — nothing to apply" }),
        ));
    }

    // Validate section names
    let valid_sections = [
        "tenant_core",
        "departments",
        "roles",
        "users",
        "settings_audit",
        "settings_virus_scan",
        "settings_ai",
        "settings_discord",
        "approval_policies",
        "email_templates",
        "notification_settings",
    ];
    let unknown: Vec<&str> = profile_obj
        .keys()
        .filter(|k| !valid_sections.contains(&k.as_str()))
        .map(|k| k.as_str())
        .collect();
    if !unknown.is_empty() {
        return Ok(Json(json!({
            "success": false,
            "error": format!("Unknown sections: {}. Valid sections: {}", unknown.join(", "), valid_sections.join(", "))
        })));
    }

    let tx = state
        .store.db()
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut results: HashMap<String, Value> = HashMap::new();
    let tenant_id = auth.tenant_id;

    for (section, section_data) in profile_obj {
        let result = match section.as_str() {
            "tenant_core" => apply_tenant_core(&tx, tenant_id, section_data).await,
            "departments" => apply_departments(&tx, tenant_id, section_data).await,
            "roles" => apply_roles(&tx, tenant_id, section_data).await,
            "users" => apply_users(&tx, tenant_id, section_data).await,
            "settings_audit" => apply_audit_settings(&tx, tenant_id, section_data).await,
            "settings_virus_scan" => apply_virus_scan(&tx, tenant_id, section_data).await,
            "settings_ai" => apply_ai_settings(&tx, tenant_id, section_data).await,
            "settings_discord" => apply_discord_settings(&tx, tenant_id, section_data).await,
            "approval_policies" => apply_approval_policies(&tx, tenant_id, section_data).await,
            "email_templates" => apply_email_templates(&tx, tenant_id, section_data).await,
            "notification_settings" => {
                apply_notification_settings(&tx, tenant_id, section_data).await
            }
            _ => Ok(json!({ "skipped": true })),
        };

        match result {
            Ok(r) => {
                results.insert(section.clone(), r);
            }
            Err(e) => {
                let _ = tx.rollback().await;
                return Ok(Json(json!({
                    "success": false,
                    "error": format!("Failed to apply section '{}': {:?}", section, e),
                    "partial_results": results
                })));
            }
        }
    }

    if dry_run {
        let _ = tx.rollback().await;
        return Ok(Json(json!({
            "success": true,
            "dry_run": true,
            "sections": profile_obj.keys().collect::<Vec<_>>(),
            "results": results
        })));
    }

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit settings profile: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_apply_profile",
        json!({
            "sections": profile_obj.keys().collect::<Vec<&String>>(),
            "results": &results
        }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    if let Some(ref cache) = state.cache {
        let _ = cache
            .delete(&app_core::cache::keys::tenant_settings(tenant_id))
            .await;
        let _ = cache
            .delete(&app_core::cache::keys::global_settings())
            .await;
    }

    Ok(Json(json!({
        "success": true,
        "sections_applied": profile_obj.keys().collect::<Vec<&String>>(),
        "results": results
    })))
}

/// GET /api/backup/current-settings
/// Returns current settings as plain JSON for the profile editor (SuperAdmin only)
pub async fn get_current_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<CurrentSettingsParams>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let mode = params.mode.as_deref().unwrap_or("tenant");

    match mode {
        "global" => {
            let global_settings = collect_global_settings(&state.store).await?;
            let email_templates = collect_global_email_templates(state.store.db()).await?;
            Ok(Json(json!({
                "global_settings": global_settings,
                "global_email_templates": email_templates
            })))
        }
        "tenant" | _ => {
            let tenant_id = auth.tenant_id;
            let tenant_core = collect_tenant_core(state.store.db(), tenant_id, false).await?;
            let audit = collect_audit_settings(state.store.db(), tenant_id).await?;
            let virus = collect_virus_scan(state.store.db(), tenant_id).await?;
            let ai = collect_ai_settings(state.store.db(), tenant_id, false).await?;
            let discord = collect_discord_settings(state.store.db(), tenant_id, false).await?;
            let policies = collect_approval_policies(state.store.db(), tenant_id).await?;
            let emails = collect_email_templates(state.store.db(), tenant_id).await?;
            let notifs = collect_notification_settings(state.store.db(), tenant_id).await?;

            Ok(Json(json!({
                "tenant_core": tenant_core,
                "settings_audit": audit,
                "settings_virus_scan": virus,
                "settings_ai": ai,
                "settings_discord": discord,
                "approval_policies": policies,
                "email_templates": emails,
                "notification_settings": notifs
            })))
        }
    }
}

/// POST /api/backup/global/apply-settings-profile
/// Apply a plaintext partial global settings profile (SuperAdmin only)
pub async fn apply_global_settings_profile(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<SettingsProfileRequest>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    check_global_backup_enabled(&state.store).await?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;

    let profile = &body.profile;
    let dry_run = body.dry_run.unwrap_or(false);

    if !profile.is_object() {
        return Ok(Json(
            json!({ "success": false, "error": "Profile must be a JSON object" }),
        ));
    }

    let profile_obj = profile.as_object().unwrap();
    if profile_obj.is_empty() {
        return Ok(Json(
            json!({ "success": false, "error": "Profile is empty — nothing to apply" }),
        ));
    }

    let valid_sections = ["global_settings", "global_email_templates"];
    let unknown: Vec<&str> = profile_obj
        .keys()
        .filter(|k| !valid_sections.contains(&k.as_str()))
        .map(|k| k.as_str())
        .collect();
    if !unknown.is_empty() {
        return Ok(Json(json!({
            "success": false,
            "error": format!("Unknown sections: {}. Valid sections: {}", unknown.join(", "), valid_sections.join(", "))
        })));
    }

    let tx = state
        .store.db()
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut updated = 0;

    // Apply global_settings
    if let Some(settings) = profile_obj
        .get("global_settings")
        .and_then(|v| v.as_object())
    {
        for (key, value) in settings {
            // Skip redacted values
            if value.as_str() == Some(REDACTED) {
                continue;
            }
            upsert_global_setting(&tx, key, value.clone(), Some(auth.user_id)).await?;
            updated += 1;
        }
    }

    // Apply global_email_templates
    if let Some(templates) = profile_obj
        .get("global_email_templates")
        .and_then(|v| v.as_array())
    {
        for template in templates {
            let key = template
                .get("template_key")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if key.is_empty() {
                continue;
            }
            let subject = template.get("subject").and_then(|v| v.as_str());
            let body_html = template.get("body_html").and_then(|v| v.as_str());
            let body_text = template.get("body_text").and_then(|v| v.as_str());
            if update_global_email_template(&tx, key, subject, body_html, body_text).await? {
                updated += 1;
            }
        }
    }

    if dry_run {
        let _ = tx.rollback().await;
        return Ok(Json(json!({
            "success": true,
            "dry_run": true,
            "settings_updated": updated,
            "sections": profile_obj.keys().collect::<Vec<_>>()
        })));
    }

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit global settings profile: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(ref cache) = state.cache {
        let _ = cache
            .delete(&app_core::cache::keys::global_settings())
            .await;
    }

    log_backup_audit(
        &state.store, auth.tenant_id, auth.user_id,
        "backup_apply_global_profile",
        json!({ "sections": profile_obj.keys().collect::<Vec<&String>>(), "settings_updated": updated }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    ).await;

    Ok(Json(json!({
        "success": true,
        "settings_updated": updated,
        "sections_applied": profile_obj.keys().collect::<Vec<&String>>()
    })))
}

/// PUT /api/backup/global/toggle
/// Enable or disable global backups (SuperAdmin only)
pub async fn toggle_global_backup(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<GlobalToggleRequest>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;

    upsert_global_setting(state.store.db(), "global_backup_enabled", json!(body.enabled), Some(auth.user_id)).await?;

    if let Some(ref cache) = state.cache {
        let _ = cache
            .delete(&app_core::cache::keys::global_settings())
            .await;
    }

    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_global_toggle",
        json!({ "enabled": body.enabled }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    Ok(Json(json!({ "success": true, "enabled": body.enabled })))
}

/// Check if global backup is enabled (defaults to true)
async fn check_global_backup_enabled(store: &app_entity::DataStore) -> Result<(), StatusCode> {
    let row = store
        .global_settings()
        .get("global_backup_enabled")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match row {
        Some(m) if m.value == json!(false) => Err(StatusCode::FORBIDDEN),
        _ => Ok(()),
    }
}

/// GET /api/backup/global/status
/// Returns whether global backup is enabled
pub async fn global_backup_status(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let row = state
        .store
        .global_settings()
        .get("global_backup_enabled")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let enabled = match row {
        Some(m) => m.value != json!(false),
        None => true, // default enabled
    };

    Ok(Json(json!({ "enabled": enabled })))
}

/// POST /api/backup/global/import/preview
pub async fn preview_global_import(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<ImportRequest>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    check_global_backup_enabled(&state.store).await?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;
    let passphrase = get_passphrase(&headers)?;
    if passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let encrypted: Value = serde_json::from_str(&body.data).map_err(|_| StatusCode::BAD_REQUEST)?;

    let plaintext = match decrypt_backup(&encrypted, &passphrase) {
        Ok(data) => data,
        Err(msg) => {
            return Ok(Json(json!({ "valid": false, "errors": [msg] })));
        }
    };

    let backup: Value = serde_json::from_slice(&plaintext).map_err(|_| StatusCode::BAD_REQUEST)?;

    let meta = backup.get("_meta").ok_or(StatusCode::BAD_REQUEST)?;
    let export_type = meta
        .get("export_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if export_type != "global" {
        return Ok(Json(
            json!({ "valid": false, "errors": ["This is not a global settings backup"] }),
        ));
    }

    let mut changes = Vec::new();
    if let Some(new_settings) = backup.get("global_settings").and_then(|v| v.as_object()) {
        let current = collect_global_settings(&state.store).await?;
        if let Some(cur_map) = current.as_object() {
            for (key, new_val) in new_settings {
                // Skip sensitive keys entirely
                if is_sensitive_key(key.as_str()) {
                    continue;
                }
                if let Some(cur_val) = cur_map.get(key) {
                    if cur_val != new_val {
                        changes.push(json!({ "field": key, "current": cur_val, "new": new_val }));
                    }
                } else {
                    changes.push(json!({ "field": key, "current": null, "new": new_val }));
                }
            }
        }
    }

    Ok(Json(json!({
        "valid": true,
        "meta": meta,
        "changes": changes,
        "change_count": changes.len()
    })))
}

/// POST /api/backup/global/import
pub async fn import_global(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<ImportRequest>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    check_global_backup_enabled(&state.store).await?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;
    let passphrase = get_passphrase(&headers)?;
    if passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let encrypted: Value = serde_json::from_str(&body.data).map_err(|_| StatusCode::BAD_REQUEST)?;

    let plaintext = match decrypt_backup(&encrypted, &passphrase) {
        Ok(data) => data,
        Err(msg) => {
            return Ok(Json(json!({ "success": false, "error": msg })));
        }
    };

    let backup: Value = serde_json::from_slice(&plaintext).map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut updated = 0;

    // Apply global settings (skip sensitive keys that should never be imported)
    if let Some(settings) = backup.get("global_settings").and_then(|v| v.as_object()) {
        for (key, value) in settings {
            if is_sensitive_key(key.as_str()) {
                continue;
            }
            upsert_global_setting(state.store.db(), key, value.clone(), Some(auth.user_id)).await?;
            updated += 1;
        }
    }

    // Apply global email templates
    if let Some(templates) = backup
        .get("global_email_templates")
        .and_then(|v| v.as_array())
    {
        for template in templates {
            let key = template
                .get("template_key")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if key.is_empty() {
                continue;
            }

            let subject = template.get("subject").and_then(|v| v.as_str());
            let body_html = template.get("body_html").and_then(|v| v.as_str());
            let body_text = template.get("body_text").and_then(|v| v.as_str());
            if update_global_email_template(state.store.db(), key, subject, body_html, body_text).await? {
                updated += 1;
            }
        }
    }

    // Invalidate cache
    if let Some(ref cache) = state.cache {
        let _ = cache
            .delete(&app_core::cache::keys::global_settings())
            .await;
    }

    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_import_global",
        json!({ "settings_updated": updated }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    Ok(Json(json!({
        "success": true,
        "settings_updated": updated
    })))
}

// ============================================================================
// APPLY FUNCTIONS (per section)
// ============================================================================

async fn apply_tenant_core(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    let obj = data.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let mut updated = Vec::new();

    let tenant = app_entity::tenants::Entity::find_by_id(tenant_id)
        .one(tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app_entity::tenants::ActiveModel = tenant.into();

    if let Some(v) = obj.get("compliance_mode").and_then(|v| v.as_str()) {
        active.compliance_mode = sea_orm::Set(v.to_string());
        updated.push("compliance_mode");
    }
    if let Some(v) = obj.get("retention_policy_days").and_then(|v| v.as_i64()) {
        active.retention_policy_days = sea_orm::Set(v as i32);
        updated.push("retention_policy_days");
    }
    if let Some(v) = obj.get("enable_totp").and_then(|v| v.as_bool()) {
        active.enable_totp = sea_orm::Set(Some(v));
        updated.push("enable_totp");
    }
    if let Some(v) = obj.get("enable_passkeys").and_then(|v| v.as_bool()) {
        active.enable_passkeys = sea_orm::Set(Some(v));
        updated.push("enable_passkeys");
    }
    if let Some(v) = obj.get("mfa_required").and_then(|v| v.as_bool()) {
        active.mfa_required = sea_orm::Set(Some(v));
        updated.push("mfa_required");
    }
    if let Some(v) = obj.get("session_timeout_minutes").and_then(|v| v.as_i64()) {
        active.session_timeout_minutes = sea_orm::Set(Some(v as i32));
        updated.push("session_timeout_minutes");
    }
    if let Some(v) = obj.get("public_sharing_enabled").and_then(|v| v.as_bool()) {
        active.public_sharing_enabled = sea_orm::Set(Some(v));
        updated.push("public_sharing_enabled");
    }
    if let Some(v) = obj.get("data_export_enabled").and_then(|v| v.as_bool()) {
        active.data_export_enabled = sea_orm::Set(Some(v));
        updated.push("data_export_enabled");
    }
    if let Some(v) = obj.get("blocked_extensions").and_then(|v| v.as_array()) {
        let arr: Vec<String> = v.iter().filter_map(|x| x.as_str().map(String::from)).collect();
        active.blocked_extensions = sea_orm::Set(Some(arr));
        updated.push("blocked_extensions");
    }
    if let Some(v) = obj.get("password_policy") {
        if !v.is_null() {
            active.password_policy = sea_orm::Set(Some(v.clone()));
            updated.push("password_policy");
        }
    }
    if let Some(v) = obj.get("ip_restriction_mode").and_then(|v| v.as_str()) {
        active.ip_restriction_mode = sea_orm::Set(Some(v.to_string()));
        updated.push("ip_restriction_mode");
    }
    if let Some(v) = obj.get("ip_allowlist").and_then(|v| v.as_array()) {
        let arr: Vec<String> = v.iter().filter_map(|x| x.as_str().map(String::from)).collect();
        active.ip_allowlist = sea_orm::Set(Some(arr));
        updated.push("ip_allowlist");
    }
    if let Some(v) = obj.get("ip_blocklist").and_then(|v| v.as_array()) {
        let arr: Vec<String> = v.iter().filter_map(|x| x.as_str().map(String::from)).collect();
        active.ip_blocklist = sea_orm::Set(Some(arr));
        updated.push("ip_blocklist");
    }
    if let Some(v) = obj.get("approval_workflow_enabled").and_then(|v| v.as_bool()) {
        active.approval_workflow_enabled = sea_orm::Set(Some(v));
        updated.push("approval_workflow_enabled");
    }

    if !updated.is_empty() {
        active.updated_at = sea_orm::Set(chrono::Utc::now().into());
        active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(json!({ "updated_fields": updated }))
}

async fn apply_departments(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    let depts = data.as_array().ok_or(StatusCode::BAD_REQUEST)?;
    let mut created = 0;
    let mut updated = 0;

    for dept in depts {
        let name = dept
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(StatusCode::BAD_REQUEST)?;
        let description = dept.get("description").and_then(|v| v.as_str());

        let existing = app_entity::departments::Entity::find()
            .filter(app_entity::departments::Column::TenantId.eq(tenant_id))
            .filter(app_entity::departments::Column::Name.eq(name))
            .one(tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(ext) = existing {
            let mut active: app_entity::departments::ActiveModel = ext.into();
            if let Some(desc) = description {
                active.description = sea_orm::Set(Some(desc.to_string()));
            }
            active.updated_at = sea_orm::Set(chrono::Utc::now().into());
            active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            updated += 1;
        } else {
            let id = dept.get("id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()).unwrap_or_else(Uuid::new_v4);
            let now = chrono::Utc::now().into();
            let active = app_entity::departments::ActiveModel {
                id: sea_orm::Set(id),
                tenant_id: sea_orm::Set(tenant_id),
                name: sea_orm::Set(name.to_string()),
                description: sea_orm::Set(description.map(String::from)),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            created += 1;
        }
    }

    Ok(json!({ "created": created, "updated": updated }))
}

async fn apply_roles(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    let roles = data.as_array().ok_or(StatusCode::BAD_REQUEST)?;
    let mut created = 0;
    let mut updated = 0;

    for role in roles {
        let name = role
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(StatusCode::BAD_REQUEST)?;
        let description = role.get("description").and_then(|v| v.as_str());
        let base_role = role
            .get("base_role")
            .and_then(|v| v.as_str())
            .unwrap_or("viewer");

        let existing = app_entity::roles::Entity::find()
            .filter(app_entity::roles::Column::TenantId.eq(Some(tenant_id)))
            .filter(app_entity::roles::Column::Name.eq(name))
            .one(tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let role_id = if let Some(ext) = existing {
            let id = ext.id;
            let mut active: app_entity::roles::ActiveModel = ext.into();
            if let Some(desc) = description {
                active.description = sea_orm::Set(Some(desc.to_string()));
            }
            active.base_role = sea_orm::Set(base_role.to_string());
            active.updated_at = sea_orm::Set(chrono::Utc::now().into());
            active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            updated += 1;
            id
        } else {
            let id = role.get("id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()).unwrap_or_else(Uuid::new_v4);
            let now = chrono::Utc::now().into();
            let active = app_entity::roles::ActiveModel {
                id: sea_orm::Set(id),
                tenant_id: sea_orm::Set(Some(tenant_id)),
                name: sea_orm::Set(name.to_string()),
                description: sea_orm::Set(description.map(String::from)),
                base_role: sea_orm::Set(base_role.to_string()),
                is_system: sea_orm::Set(Some(false)),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            };
            active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            created += 1;
            id
        };

        if let Some(perms) = role.get("permissions").and_then(|v| v.as_array()) {
            let _ = app_entity::role_permissions::Entity::delete_many()
                .filter(app_entity::role_permissions::Column::RoleId.eq(role_id))
                .exec(tx)
                .await;

            for perm in perms {
                let permission = perm
                    .get("permission")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let granted = perm
                    .get("granted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if !permission.is_empty() {
                    if !VALID_PERMISSIONS.contains(&permission) {
                        tracing::warn!("Skipping unknown permission in import: {}", permission);
                        continue;
                    }
                    let p_active = app_entity::role_permissions::ActiveModel {
                        id: sea_orm::Set(Uuid::new_v4()),
                        role_id: sea_orm::Set(role_id),
                        permission: sea_orm::Set(permission.to_string()),
                        granted: sea_orm::Set(Some(granted)),
                        created_at: sea_orm::Set(chrono::Utc::now().into()),
                    };
                    let _ = p_active.insert(tx).await;
                }
            }
        }
    }

    Ok(json!({ "created": created, "updated": updated }))
}

async fn apply_users(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    let users = data.as_array().ok_or(StatusCode::BAD_REQUEST)?;
    let mut created = 0;
    let mut updated = 0;

    for user in users {
        let email = user
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or(StatusCode::BAD_REQUEST)?;

        if !email.contains('@') || !email.contains('.') || email.len() > 254 {
            tracing::warn!("Skipping user with invalid email in import: {}", email);
            continue;
        }

        let name = user.get("name").and_then(|v| v.as_str()).unwrap_or(email);
        let role = user
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("Employee");
        let status = user
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("active");
        let identity_provider = user
            .get("identity_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("local");

        let existing = app_entity::users::Entity::find()
            .filter(app_entity::users::Column::TenantId.eq(tenant_id))
            .filter(app_entity::users::Column::Email.eq(email))
            .one(tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(ext) = existing {
            let mut active: app_entity::users::ActiveModel = ext.into();
            active.name = sea_orm::Set(name.to_string());
            active.role = sea_orm::Set(role.to_string());
            active.status = sea_orm::Set(status.to_string());
            active.identity_provider = sea_orm::Set(identity_provider.to_string());
            if let Some(dept_id) = user.get("department_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()) {
                active.department_id = sea_orm::Set(Some(dept_id));
            }
            active.updated_at = sea_orm::Set(chrono::Utc::now().into());
            active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            updated += 1;
        } else {
            let random_hash = format!(
                concat!("$argon2id$v=19$m=65536,t=3,p=1", "$", "{}", "$", "{}"),
                nanoid::nanoid!(22),
                nanoid::nanoid!(43)
            );
            let id = user.get("id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()).unwrap_or_else(Uuid::new_v4);
            let now = chrono::Utc::now().into();
            let active = app_entity::users::ActiveModel {
                id: sea_orm::Set(id),
                tenant_id: sea_orm::Set(tenant_id),
                department_id: sea_orm::Set(None),
                custom_role_id: sea_orm::Set(None),
                email: sea_orm::Set(email.to_string()),
                name: sea_orm::Set(name.to_string()),
                password_hash: sea_orm::Set(Some(random_hash)),
                role: sea_orm::Set(role.to_string()),
                status: sea_orm::Set(status.to_string()),
                avatar_url: sea_orm::Set(None),
                allowed_tenant_ids: sea_orm::Set(None),
                allowed_department_ids: sea_orm::Set(None),
                totp_secret: sea_orm::Set(None),
                recovery_token: sea_orm::Set(None),
                recovery_token_expires_at: sea_orm::Set(None),
                password_changed_at: sea_orm::Set(None),
                suspended_at: sea_orm::Set(None),
                suspended_until: sea_orm::Set(None),
                suspension_reason: sea_orm::Set(None),
                dashboard_layout: sea_orm::Set(None),
                widget_config: sea_orm::Set(None),
                last_active_at: sea_orm::Set(None),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
                identity_provider: sea_orm::Set(identity_provider.to_string()),
            };
            active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            created += 1;
        }
    }

    Ok(json!({ "created": created, "updated": updated }))
}

async fn apply_audit_settings(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    let obj = data.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let now = chrono::Utc::now().into();

    let existing = app_entity::audit_settings::Entity::find()
        .filter(app_entity::audit_settings::Column::TenantId.eq(tenant_id))
        .one(tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let log_logins = obj.get("log_logins").and_then(|v| v.as_bool()).unwrap_or(true);
    let log_file_operations = obj.get("log_file_operations").and_then(|v| v.as_bool()).unwrap_or(true);
    let log_user_changes = obj.get("log_user_changes").and_then(|v| v.as_bool()).unwrap_or(true);
    let log_settings_changes = obj.get("log_settings_changes").and_then(|v| v.as_bool()).unwrap_or(true);
    let log_role_changes = obj.get("log_role_changes").and_then(|v| v.as_bool()).unwrap_or(true);
    let retention_days = obj.get("retention_days").and_then(|v| v.as_i64()).unwrap_or(90) as i32;

    if let Some(ext) = existing {
        let mut active: app_entity::audit_settings::ActiveModel = ext.into();
        active.log_logins = sea_orm::Set(Some(log_logins));
        active.log_file_operations = sea_orm::Set(Some(log_file_operations));
        active.log_user_changes = sea_orm::Set(Some(log_user_changes));
        active.log_settings_changes = sea_orm::Set(Some(log_settings_changes));
        active.log_role_changes = sea_orm::Set(Some(log_role_changes));
        active.retention_days = sea_orm::Set(Some(retention_days));
        active.updated_at = sea_orm::Set(now);
        active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        let active = app_entity::audit_settings::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            tenant_id: sea_orm::Set(tenant_id),
            log_logins: sea_orm::Set(Some(log_logins)),
            log_file_operations: sea_orm::Set(Some(log_file_operations)),
            log_user_changes: sea_orm::Set(Some(log_user_changes)),
            log_settings_changes: sea_orm::Set(Some(log_settings_changes)),
            log_role_changes: sea_orm::Set(Some(log_role_changes)),
            retention_days: sea_orm::Set(Some(retention_days)),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(json!({ "applied": true }))
}

async fn apply_virus_scan(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    if data.is_null() {
        return Ok(json!({ "skipped": true, "reason": "No virus scan settings in backup" }));
    }
    let obj = data.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let now = chrono::Utc::now().into();

    let existing = app_entity::virus_scan_settings::Entity::find()
        .filter(app_entity::virus_scan_settings::Column::TenantId.eq(tenant_id))
        .one(tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let enabled = obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let file_types: Option<Vec<String>> = obj.get("file_types").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect());
    let max_file_size_mb = obj.get("max_file_size_mb").and_then(|v| v.as_i64()).unwrap_or(100) as i32;
    let action_on_detect = obj.get("action_on_detect").and_then(|v| v.as_str()).unwrap_or("quarantine");
    let notify_admin = obj.get("notify_admin").and_then(|v| v.as_bool()).unwrap_or(true);
    let notify_uploader = obj.get("notify_uploader").and_then(|v| v.as_bool()).unwrap_or(true);
    let auto_suspend_uploader = obj.get("auto_suspend_uploader").and_then(|v| v.as_bool()).unwrap_or(false);
    let suspend_threshold = obj.get("suspend_threshold").and_then(|v| v.as_i64()).unwrap_or(3) as i32;

    if let Some(ext) = existing {
        let mut active: app_entity::virus_scan_settings::ActiveModel = ext.into();
        active.enabled = sea_orm::Set(enabled);
        if let Some(ft) = file_types { active.file_types = sea_orm::Set(Some(ft)); }
        active.max_file_size_mb = sea_orm::Set(Some(max_file_size_mb));
        active.action_on_detect = sea_orm::Set(action_on_detect.to_string());
        active.notify_admin = sea_orm::Set(notify_admin);
        active.notify_uploader = sea_orm::Set(notify_uploader);
        active.auto_suspend_uploader = sea_orm::Set(auto_suspend_uploader);
        active.suspend_threshold = sea_orm::Set(suspend_threshold);
        active.updated_at = sea_orm::Set(now);
        active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        let active = app_entity::virus_scan_settings::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            tenant_id: sea_orm::Set(tenant_id),
            enabled: sea_orm::Set(enabled),
            file_types: sea_orm::Set(file_types),
            max_file_size_mb: sea_orm::Set(Some(max_file_size_mb)),
            action_on_detect: sea_orm::Set(action_on_detect.to_string()),
            notify_admin: sea_orm::Set(notify_admin),
            notify_uploader: sea_orm::Set(notify_uploader),
            auto_suspend_uploader: sea_orm::Set(auto_suspend_uploader),
            suspend_threshold: sea_orm::Set(suspend_threshold),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(json!({ "applied": true }))
}

async fn apply_ai_settings(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    if data.is_null() {
        return Ok(json!({ "skipped": true }));
    }
    let obj = data.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let now = chrono::Utc::now().into();

    let existing = app_entity::tenant_ai_settings::Entity::find_by_id(tenant_id)
        .one(tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let enabled = obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let provider = obj.get("provider").and_then(|v| v.as_str()).unwrap_or("openai");
    let allowed_roles: Vec<String> = obj.get("allowed_roles").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let monthly_token_limit = obj.get("monthly_token_limit").and_then(|v| v.as_i64()).unwrap_or(1_000_000) as i32;
    let daily_request_limit = obj.get("daily_request_limit").and_then(|v| v.as_i64()).unwrap_or(1_000) as i32;
    let api_key = obj.get("api_key_encrypted").and_then(|v| v.as_str()).filter(|s| *s != REDACTED);

    if let Some(ext) = existing {
        let mut active: app_entity::tenant_ai_settings::ActiveModel = ext.into();
        active.enabled = sea_orm::Set(enabled);
        active.provider = sea_orm::Set(provider.to_string());
        active.allowed_roles = sea_orm::Set(allowed_roles);
        active.monthly_token_limit = sea_orm::Set(monthly_token_limit);
        active.daily_request_limit = sea_orm::Set(daily_request_limit);
        if let Some(key) = api_key {
            active.api_key_encrypted = sea_orm::Set(Some(key.to_string()));
        }
        active.updated_at = sea_orm::Set(now);
        active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        let active = app_entity::tenant_ai_settings::ActiveModel {
            tenant_id: sea_orm::Set(tenant_id),
            enabled: sea_orm::Set(enabled),
            provider: sea_orm::Set(provider.to_string()),
            api_key_encrypted: sea_orm::Set(api_key.map(String::from)),
            allowed_roles: sea_orm::Set(allowed_roles),
            hipaa_approved_only: sea_orm::Set(false),
            sox_read_only: sea_orm::Set(false),
            monthly_token_limit: sea_orm::Set(monthly_token_limit),
            daily_request_limit: sea_orm::Set(daily_request_limit),
            tokens_used_this_month: sea_orm::Set(0),
            requests_today: sea_orm::Set(0),
            last_usage_reset: sea_orm::Set(None),
            maintenance_mode: sea_orm::Set(false),
            maintenance_message: sea_orm::Set(None),
            custom_endpoint: sea_orm::Set(None),
            custom_model: sea_orm::Set(None),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(json!({ "applied": true }))
}

async fn apply_discord_settings(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    if data.is_null() {
        return Ok(json!({ "skipped": true }));
    }
    let obj = data.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let enabled = obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let now = chrono::Utc::now().into();

    let existing = app_entity::tenant_discord_settings::Entity::find_by_id(tenant_id)
        .one(tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(ext) = existing {
        let mut active: app_entity::tenant_discord_settings::ActiveModel = ext.into();
        active.enabled = sea_orm::Set(enabled);
        active.updated_at = sea_orm::Set(now);
        active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        let active = app_entity::tenant_discord_settings::ActiveModel {
            tenant_id: sea_orm::Set(tenant_id),
            enabled: sea_orm::Set(enabled),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(json!({ "applied": true }))
}

async fn apply_approval_policies(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    let policies = data.as_array().ok_or(StatusCode::BAD_REQUEST)?;

    app_entity::approval_policies::Entity::delete_many()
        .filter(app_entity::approval_policies::Column::TenantId.eq(tenant_id))
        .exec(tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = chrono::Utc::now().into();
    for policy in policies {
        let name = policy.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let scope = policy.get("scope").and_then(|v| v.as_str()).unwrap_or("all");
        let scope_value = policy.get("scope_value").and_then(|v| v.as_str());
        let required = policy.get("required_approvals").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
        let active = policy.get("is_active").and_then(|v| v.as_bool()).unwrap_or(true);

        let p_active = app_entity::approval_policies::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            tenant_id: sea_orm::Set(tenant_id),
            name: sea_orm::Set(name.to_string()),
            scope: sea_orm::Set(scope.to_string()),
            scope_value: sea_orm::Set(scope_value.map(String::from)),
            required_approvals: sea_orm::Set(required),
            is_active: sea_orm::Set(active),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        p_active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(json!({ "replaced": policies.len() }))
}

async fn apply_email_templates(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    let templates = data.as_array().ok_or(StatusCode::BAD_REQUEST)?;
    let mut applied = 0;

    for template in templates {
        let key = template
            .get("template_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if key.is_empty() {
            continue;
        }

        let subject = template.get("subject").and_then(|v| v.as_str()).unwrap_or("");
        let body_html = template.get("body_html").and_then(|v| v.as_str()).unwrap_or("");
        let body_text = template.get("body_text").and_then(|v| v.as_str());

        let existing = app_entity::tenant_email_templates::Entity::find()
            .filter(app_entity::tenant_email_templates::Column::TenantId.eq(tenant_id))
            .filter(app_entity::tenant_email_templates::Column::TemplateKey.eq(key))
            .one(tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(ext) = existing {
            let mut active: app_entity::tenant_email_templates::ActiveModel = ext.into();
            active.subject = sea_orm::Set(subject.to_string());
            active.body_html = sea_orm::Set(body_html.to_string());
            active.body_text = sea_orm::Set(body_text.map(String::from));
            active.updated_at = sea_orm::Set(Some(chrono::Utc::now().into()));
            active.update(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        } else {
            let now = chrono::Utc::now().into();
            let active = app_entity::tenant_email_templates::ActiveModel {
                id: sea_orm::Set(Uuid::new_v4()),
                tenant_id: sea_orm::Set(tenant_id),
                template_key: sea_orm::Set(key.to_string()),
                subject: sea_orm::Set(subject.to_string()),
                body_html: sea_orm::Set(body_html.to_string()),
                body_text: sea_orm::Set(body_text.map(String::from)),
                created_at: sea_orm::Set(Some(now)),
                updated_at: sea_orm::Set(Some(now)),
            };
            active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        applied += 1;
    }

    Ok(json!({ "applied": applied }))
}

async fn apply_notification_settings(
    tx: &DatabaseTransaction,
    tenant_id: Uuid,
    data: &Value,
) -> Result<Value, StatusCode> {
    let settings = data.as_array().ok_or(StatusCode::BAD_REQUEST)?;

    app_entity::tenant_notification_settings::Entity::delete_many()
        .filter(app_entity::tenant_notification_settings::Column::TenantId.eq(tenant_id))
        .exec(tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let now = chrono::Utc::now().into();
    for setting in settings {
        let event_type = setting
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if event_type.is_empty() {
            continue;
        }

        let active = app_entity::tenant_notification_settings::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            tenant_id: sea_orm::Set(tenant_id),
            event_type: sea_orm::Set(event_type.to_string()),
            role: sea_orm::Set(setting.get("role").and_then(|v| v.as_str()).map(String::from)),
            enabled: sea_orm::Set(setting.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)),
            email_enforced: sea_orm::Set(setting.get("email_enforced").and_then(|v| v.as_bool()).unwrap_or(false)),
            in_app_enforced: sea_orm::Set(setting.get("in_app_enforced").and_then(|v| v.as_bool()).unwrap_or(false)),
            default_email: sea_orm::Set(setting.get("default_email").and_then(|v| v.as_bool()).unwrap_or(true)),
            default_in_app: sea_orm::Set(setting.get("default_in_app").and_then(|v| v.as_bool()).unwrap_or(true)),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
        };
        active.insert(tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(json!({ "replaced": settings.len() }))
}

pub async fn section_counts(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let counts = state
        .store
        .backup()
        .section_counts(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "users": counts.users,
        "departments": counts.departments,
        "roles": counts.roles,
        "file_metadata": counts.file_metadata,
        "audit_logs": counts.audit_logs,
        "approval_policies": counts.approval_policies,
        "approval_history": counts.approval_history,
        "sso_oidc": counts.sso_oidc,
        "sso_saml": counts.sso_saml,
    })))
}

// ============================================================================
// SAVE-TO-STORAGE ENDPOINTS
// ============================================================================

/// POST /api/backup/save
/// Export backup and save to storage backend (not browser download)
pub async fn save_backup_to_storage(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<ExportParams>,
) -> Result<Json<Value>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    check_backup_enabled(&state.store, auth.tenant_id, &auth.role).await?;
    let _permit = check_backup_infra(&state)?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;
    let passphrase = get_passphrase(&headers)?;
    if passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let include_secrets = params.include_secrets.unwrap_or(false);
    if include_secrets && auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let start = std::time::Instant::now();

    // Build backup payload (reuse existing collect logic)
    let (encrypted_bytes, filename, sections) = build_backup_payload(
        &state,
        auth.tenant_id,
        auth.user_id,
        include_secrets,
        &params,
        &passphrase,
    )
    .await
    .map_err(|e| {
        state.backup_circuit_breaker.record_failure();
        e
    })?;

    // Save to storage
    let storage_path = format!("_backups/{}", filename);
    let size_bytes = encrypted_bytes.len() as i64;

    state
        .storage
        .upload(&storage_path, encrypted_bytes.clone())
        .await
        .map_err(|e| {
            tracing::error!("Failed to save backup to storage: {:?}", e);
            state.backup_circuit_breaker.record_failure();
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let duration_ms = start.elapsed().as_millis() as i32;

    // Record in backup_history
    let record_id = state
        .store
        .backup()
        .record_backup(
            Some(auth.tenant_id),
            filename.clone(),
            storage_path.clone(),
            size_bytes,
            json!(sections),
            false,
            duration_ms,
            Some(auth.user_id),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to record backup history: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state.backup_circuit_breaker.record_success();

    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_save",
        json!({
            "filename": filename,
            "size_bytes": size_bytes,
            "duration_ms": duration_ms,
            "sections": sections,
        }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    Ok(Json(json!({
        "success": true,
        "id": record_id,
        "filename": filename,
        "size_bytes": size_bytes,
        "duration_ms": duration_ms,
    })))
}

/// Build backup payload — shared by export-to-browser, save-to-storage, and auto-backup.
/// Returns (encrypted_bytes, filename, sections_list).
async fn build_backup_payload(
    state: &AppState,
    tenant_id: Uuid,
    user_id: Uuid,
    include_secrets: bool,
    params: &ExportParams,
    passphrase: &str,
) -> Result<(Vec<u8>, String, Vec<String>), StatusCode> {
    let valid_sections: &[&str] = &[
        "tenant_core",
        "users",
        "departments",
        "roles",
        "settings_audit",
        "settings_virus_scan",
        "settings_ai",
        "settings_discord",
        "sso_oidc",
        "sso_saml",
        "sso_mappings",
        "sso_identities",
        "approval_policies",
        "email_templates",
        "notification_settings",
    ];
    let valid_optional: &[&str] = &["file_metadata", "audit_logs", "approval_history"];
    let sections: Vec<String> = params
        .sections
        .as_ref()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_else(|| valid_sections.iter().map(|s| s.to_string()).collect());

    for s in &sections {
        if !valid_sections.contains(&s.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let optional: Vec<String> = params
        .include_optional
        .as_ref()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    for s in &optional {
        if !valid_optional.contains(&s.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let audit_days = params.audit_days.unwrap_or(90);
    let file_limit = params.file_limit.unwrap_or(50000);
    let approval_days = params.approval_days.unwrap_or(90);

    let tenant_name = state
        .store
        .tenants()
        .tenant(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|t| t.name)
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut all_sections = sections.clone();
    all_sections.extend(optional.iter().cloned());

    let mut backup = json!({
        "_meta": {
            "format": "clovalink-backup",
            "format_version": 1,
            "clovalink_version": CURRENT_VERSION,
            "export_type": "tenant",
            "tenant_id": tenant_id.to_string(),
            "tenant_name": tenant_name,
            "exported_at": Utc::now().to_rfc3339(),
            "exported_by": user_id.to_string(),
            "include_secrets": include_secrets,
            "sections": &all_sections
        }
    });

    let backup_map = backup.as_object_mut().unwrap();

    for section in &sections {
        let value = match section.as_str() {
            "tenant_core" => collect_tenant_core(state.store.db(), tenant_id, include_secrets).await?,
            "users" => collect_users(state.store.db(), tenant_id, include_secrets).await?,
            "departments" => collect_departments(state.store.db(), tenant_id).await?,
            "roles" => collect_roles(state.store.db(), tenant_id).await?,
            "settings_audit" => collect_audit_settings(state.store.db(), tenant_id).await?,
            "settings_virus_scan" => collect_virus_scan(state.store.db(), tenant_id).await?,
            "settings_ai" => collect_ai_settings(state.store.db(), tenant_id, include_secrets).await?,
            "settings_discord" => {
                collect_discord_settings(state.store.db(), tenant_id, include_secrets).await?
            }
            "sso_oidc" => collect_sso_oidc(state.store.db(), tenant_id, include_secrets).await?,
            "sso_saml" => collect_sso_saml(state.store.db(), tenant_id, include_secrets).await?,
            "sso_mappings" => collect_sso_mappings(state.store.db(), tenant_id).await?,
            "sso_identities" => collect_sso_identities(state.store.db(), tenant_id).await?,
            "approval_policies" => collect_approval_policies(state.store.db(), tenant_id).await?,
            "email_templates" => collect_email_templates(state.store.db(), tenant_id).await?,
            "notification_settings" => {
                collect_notification_settings(state.store.db(), tenant_id).await?
            }
            _ => continue,
        };
        backup_map.insert(section.clone(), value);
    }

    for section in &optional {
        let value = match section.as_str() {
            "file_metadata" => collect_file_metadata(state.store.db(), tenant_id, file_limit).await?,
            "audit_logs" => collect_audit_logs(state.store.db(), tenant_id, audit_days).await?,
            "approval_history" => {
                collect_approval_history(state.store.db(), tenant_id, approval_days).await?
            }
            _ => continue,
        };
        backup_map.insert(section.clone(), value);
    }

    let plaintext = serde_json::to_vec(&backup).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let encrypted = encrypt_backup(&plaintext, passphrase)?;
    let encrypted_bytes =
        serde_json::to_vec_pretty(&encrypted).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!(
        "clovalink-backup-{}-{}-{:06x}.clovalink.json",
        tenant_name.to_lowercase().replace(' ', "-"),
        Utc::now().format("%Y%m%d-%H%M%S"),
        rand::random::<u32>() & 0xFFFFFF
    );

    Ok((encrypted_bytes, filename, all_sections))
}

/// GET /api/backup/saved
/// List saved backups for this tenant
pub async fn list_saved_backups(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<BackupListParams>,
) -> Result<Json<Value>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let is_global = params.mode.as_deref() == Some("global");
    if is_global && auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let tenant_id = if is_global { None } else { Some(auth.tenant_id) };
    let rows = state
        .store
        .backup()
        .list_history(tenant_id, 50)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let backups: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "filename": r.filename,
                "storage_path": r.storage_path,
                "size_bytes": r.size_bytes,
                "sections": r.sections,
                "is_auto_backup": r.is_auto_backup,
                "status": r.status,
                "error_message": r.error_message,
                "duration_ms": r.duration_ms,
                "created_by": r.created_by,
                "created_at": r.created_at,
            })
        })
        .collect();

    Ok(Json(json!(backups)))
}

/// GET /api/backup/saved/:id/download
/// Download a saved backup from storage
pub async fn download_saved_backup(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Response<Body>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let backup = state
        .store
        .backup()
        .find_history_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Authorization: global backups require SuperAdmin, tenant backups require matching tenant
    match backup.tenant_id {
        None => {
            if auth.role != "SuperAdmin" {
                return Err(StatusCode::FORBIDDEN);
            }
        }
        Some(tid) => {
            if tid != auth.tenant_id {
                return Err(StatusCode::NOT_FOUND);
            }
        }
    }

    let data = state.storage.download(&backup.storage_path).await.map_err(|e| {
        tracing::error!("Failed to download backup from storage: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", backup.filename),
        )
        .header(header::CACHE_CONTROL, "no-store")
        .header("X-Content-Type-Options", "nosniff")
        .body(Body::from(data))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// DELETE /api/backup/saved/:id
/// Delete a saved backup from storage and history
pub async fn delete_saved_backup(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let backup = state
        .store
        .backup()
        .find_history_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Authorization: global backups require SuperAdmin, tenant backups require matching tenant
    match backup.tenant_id {
        None => {
            if auth.role != "SuperAdmin" {
                return Err(StatusCode::FORBIDDEN);
            }
        }
        Some(tid) => {
            if tid != auth.tenant_id {
                return Err(StatusCode::NOT_FOUND);
            }
        }
    }

    // Delete from storage (best effort)
    let _ = state.storage.delete(&backup.storage_path).await;

    // Delete from history
    state
        .store
        .backup()
        .delete_history_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_delete",
        json!({ "backup_id": id, "storage_path": backup.storage_path }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    Ok(Json(json!({ "success": true })))
}

/// GET /api/backup/health
/// Circuit breaker state
pub async fn backup_health(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    if !["Admin", "SuperAdmin"].contains(&auth.role.as_str()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let cb = &state.backup_circuit_breaker;
    let state_str = match cb.state() {
        CircuitState::Closed => "closed",
        CircuitState::Open => "open",
        CircuitState::HalfOpen => "half_open",
    };

    Ok(Json(json!({
        "state": state_str,
        "failure_count": cb.metrics().failure_count,
        "master_key_configured": is_master_key_configured(),
    })))
}

/// GET /api/backup/metrics
/// Backup performance metrics (SuperAdmin only)
pub async fn backup_metrics(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let cb = &state.backup_circuit_breaker;
    let cb_state = match cb.state() {
        CircuitState::Closed => "closed",
        CircuitState::Open => "open",
        CircuitState::HalfOpen => "half_open",
    };

    let max_concurrent = types::config::get_config().backup.max_concurrent;
    let available = state.backup_semaphore.available_permits();

    let metrics = state
        .store
        .backup()
        .get_metrics()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tenant_list: Vec<Value> = metrics
        .by_tenant
        .into_iter()
        .map(|t| {
            json!({
                "tenant_name": t.tenant_name,
                "backup_count": t.backup_count,
                "last_backup": t.last_backup,
                "auto_enabled": t.auto_enabled,
            })
        })
        .collect();

    Ok(Json(json!({
        "circuit_breaker": { "state": cb_state, "failure_count": cb.metrics().failure_count },
        "concurrency": { "max": max_concurrent, "available": available, "active": max_concurrent - available },
        "total_backups": metrics.total,
        "total_auto_backups": metrics.auto_count,
        "total_manual_backups": metrics.total - metrics.auto_count,
        "last_backup_at": metrics.last_backup_at,
        "last_backup_duration_ms": metrics.last_backup_duration_ms,
        "failed_backups_24h": metrics.failed_24h,
        "total_storage_bytes": metrics.total_storage,
        "by_tenant": tenant_list,
    })))
}

pub async fn start_backup_scheduler(
    store: app_entity::DataStore,
    storage: Arc<dyn crate::storage::Storage>,
    circuit_breaker: Arc<app_core::circuit_breaker::CircuitBreaker>,
    semaphore: Arc<tokio::sync::Semaphore>,
    redis_url: String,
) {
    tracing::info!("Backup scheduler started");
    let db = store.db().clone();

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));


    loop {
        interval.tick().await;

        // Skip if circuit breaker is open
        if !circuit_breaker.allow_request() {
            tracing::debug!("Backup scheduler: circuit breaker open, skipping cycle");
            continue;
        }

        // Acquire distributed lock via Redis
        let lock_acquired = match acquire_scheduler_lock(&redis_url).await {
            Ok(acquired) => acquired,
            Err(e) => {
                tracing::debug!("Backup scheduler: failed to acquire lock: {:?}", e);
                continue;
            }
        };
        if !lock_acquired {
            continue;
        }

        // Find tenants with auto-backup due
        let due_tenants = match find_due_tenants(&db).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Backup scheduler: failed to find due tenants: {:?}", e);
                continue;
            }
        };

        // Check for global auto-backup
        if let Err(e) =
            check_and_run_global_auto_backup(&store, &db, &storage, &circuit_breaker, &semaphore).await
        {
            tracing::debug!("Global auto-backup check: {:?}", e);
        }

        if due_tenants.is_empty() {
            continue;
        }

        tracing::info!(
            "Backup scheduler: {} tenants due for backup",
            due_tenants.len()
        );

        // Process at most 5 per cycle
        let batch: Vec<_> = due_tenants.into_iter().take(5).collect();

        for (tenant_id, tenant_name, _cron_expr, retention_count) in batch {
            // Acquire semaphore (shared with manual backups)
            let permit = match semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    tracing::info!("Backup scheduler: semaphore full, deferring remaining tenants");
                    break;
                }
            };

            // Random jitter 0-30 seconds
            let jitter_ms = rand::random::<u64>() % 30_000;
            tokio::time::sleep(std::time::Duration::from_millis(jitter_ms)).await;

            let store_clone = store.clone();
            let pool_clone = db.clone();
            let storage_clone = storage.clone();
            let cb_clone = circuit_breaker.clone();

            let result = run_auto_backup(
                &store_clone,
                &pool_clone,
                &storage_clone,
                &cb_clone,
                tenant_id,
                &tenant_name,
            )
            .await;

            match result {
                Ok((size, duration_ms)) => {
                    tracing::info!(
                        "Auto-backup completed for '{}': {}KB in {}ms",
                        tenant_name,
                        size / 1024,
                        duration_ms
                    );
                    // Enforce retention: delete oldest beyond limit
                    let _ =
                        enforce_retention(&pool_clone, &storage_clone, tenant_id, retention_count)
                            .await;
                }
                Err(e) => {
                    tracing::error!("Auto-backup failed for '{}': {:?}", tenant_name, e);
                }
            }

            drop(permit);
        }
    }
}

/// Acquire a distributed lock via Redis SET NX EX
async fn acquire_scheduler_lock(
    redis_url: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    let result: Option<String> = redis::cmd("SET")
        .arg("clovalink:backup:scheduler")
        .arg("locked")
        .arg("NX")
        .arg("EX")
        .arg(55) // 55 second TTL (less than 60s poll interval)
        .query_async(&mut conn)
        .await?;
    Ok(result.is_some())
}

/// Find tenants with auto-backup enabled that are due for a backup
async fn find_due_tenants(
    db: &DatabaseConnection,
) -> Result<Vec<(Uuid, String, String, i32)>, StatusCode> {
    let tenant_models = app_entity::tenants::Entity::find()
        .filter(app_entity::tenants::Column::AutoBackupEnabled.eq(Some(true)))
        .filter(app_entity::tenants::Column::BackupEnabled.ne(Some(false)))
        .filter(app_entity::tenants::Column::Status.eq("active"))
        .order_by_asc(app_entity::tenants::Column::Id)
        .all(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tenants: Vec<(Uuid, String, String, i32)> = tenant_models
        .into_iter()
        .map(|t| {
            let cron = t.auto_backup_cron.unwrap_or_else(|| "0 2 * * 0".to_string());
            let ret = t.auto_backup_retention_count.unwrap_or(5);
            (t.id, t.name, cron, ret)
        })
        .collect();

    let mut due = Vec::new();
    let now = Utc::now();

    for (tenant_id, name, cron_expr, retention) in tenants {
        let schedule = match normalize_cron(&cron_expr).parse::<cron::Schedule>() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    "Invalid cron '{}' for tenant '{}': {:?}",
                    cron_expr,
                    name,
                    e
                );
                continue;
            }
        };

        let last_backup = app_entity::backup_history::Entity::find()
            .filter(app_entity::backup_history::Column::TenantId.eq(Some(tenant_id)))
            .filter(app_entity::backup_history::Column::IsAutoBackup.eq(true))
            .order_by_desc(app_entity::backup_history::Column::CreatedAt)
            .one(db)
            .await
            .ok()
            .flatten();

        let last_time: Option<chrono::DateTime<Utc>> = last_backup.map(|b| b.created_at.into());

        // Check if a backup is due: find the most recent scheduled time before now
        // and check if it's after the last backup
        let should_run = if let Some(prev_time) = schedule
            .after(&(now - chrono::Duration::hours(25)))
            .take(1)
            .next()
        {
            if prev_time <= now {
                match last_time {
                    Some(lt) => prev_time > lt,
                    None => true, // Never backed up
                }
            } else {
                false
            }
        } else {
            false
        };

        if should_run {
            due.push((tenant_id, name, cron_expr, retention));
        }
    }

    Ok(due)
}

/// Run an automatic backup for a tenant
async fn run_auto_backup(
    store: &app_entity::DataStore,
    db: &DatabaseConnection,
    storage: &Arc<dyn crate::storage::Storage>,
    circuit_breaker: &Arc<app_core::circuit_breaker::CircuitBreaker>,
    tenant_id: Uuid,
    tenant_name: &str,
) -> Result<(i64, i32), StatusCode> {
    let start = std::time::Instant::now();

    // Auto-backups use system passphrase from global_settings
    let passphrase = get_or_create_auto_passphrase(db).await?;

    // Core sections only for auto-backup (no large optional data)
    let default_sections = vec![
        "tenant_core",
        "users",
        "departments",
        "roles",
        "settings_audit",
        "settings_virus_scan",
        "settings_ai",
        "settings_discord",
        "sso_oidc",
        "sso_saml",
        "sso_mappings",
        "sso_identities",
        "approval_policies",
        "email_templates",
        "notification_settings",
    ];
    let sections: Vec<String> = default_sections.iter().map(|s| s.to_string()).collect();

    let mut backup = json!({
        "_meta": {
            "format": "clovalink-backup",
            "format_version": 1,
            "clovalink_version": CURRENT_VERSION,
            "export_type": "tenant",
            "tenant_id": tenant_id.to_string(),
            "tenant_name": tenant_name,
            "exported_at": Utc::now().to_rfc3339(),
            "exported_by": "system-auto-backup",
            "include_secrets": false,
            "sections": &sections
        }
    });

    let backup_map = backup.as_object_mut().unwrap();

    for section in &sections {
        let value = match section.as_str() {
            "tenant_core" => collect_tenant_core(db, tenant_id, false).await?,
            "users" => collect_users(db, tenant_id, false).await?,
            "departments" => collect_departments(db, tenant_id).await?,
            "roles" => collect_roles(db, tenant_id).await?,
            "settings_audit" => collect_audit_settings(db, tenant_id).await?,
            "settings_virus_scan" => collect_virus_scan(db, tenant_id).await?,
            "settings_ai" => collect_ai_settings(db, tenant_id, false).await?,
            "settings_discord" => collect_discord_settings(db, tenant_id, false).await?,
            "sso_oidc" => collect_sso_oidc(db, tenant_id, false).await?,
            "sso_saml" => collect_sso_saml(db, tenant_id, false).await?,
            "sso_mappings" => collect_sso_mappings(db, tenant_id).await?,
            "sso_identities" => collect_sso_identities(db, tenant_id).await?,
            "approval_policies" => collect_approval_policies(db, tenant_id).await?,
            "email_templates" => collect_email_templates(db, tenant_id).await?,
            "notification_settings" => collect_notification_settings(db, tenant_id).await?,
            _ => continue,
        };
        backup_map.insert(section.clone(), value);
    }

    let plaintext = serde_json::to_vec(&backup).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let encrypted = encrypt_backup(&plaintext, &passphrase)?;
    let encrypted_bytes =
        serde_json::to_vec_pretty(&encrypted).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!(
        "clovalink-auto-backup-{}-{}-{:06x}.clovalink.json",
        tenant_name.to_lowercase().replace(' ', "-"),
        Utc::now().format("%Y%m%d-%H%M%S"),
        rand::random::<u32>() & 0xFFFFFF
    );
    let storage_path = format!("_backups/{}", filename);
    let size_bytes = encrypted_bytes.len() as i64;

    storage
        .upload(&storage_path, encrypted_bytes)
        .await
        .map_err(|e| {
            tracing::error!("Auto-backup storage upload failed: {:?}", e);
            circuit_breaker.record_failure();
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let duration_ms = start.elapsed().as_millis() as i32;

    circuit_breaker.record_success();

    // Record in backup_history
    store
        .backup()
        .record_backup(
            Some(tenant_id),
            filename.clone(),
            storage_path.clone(),
            size_bytes,
            json!(sections),
            true,
            duration_ms,
            None,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Audit log
    log_backup_audit(
        store,
        tenant_id,
        Uuid::nil(),
        "backup_auto",
        json!({
            "filename": filename,
            "size_bytes": size_bytes,
            "duration_ms": duration_ms,
        }),
        "system",
    )
    .await;

    Ok((size_bytes, duration_ms))
}

/// Get or create the system auto-backup passphrase
async fn get_or_create_auto_passphrase(db: &DatabaseConnection) -> Result<String, StatusCode> {
    let existing = app_entity::global_settings::Entity::find_by_id("auto_backup_passphrase")
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(model) = existing {
        if let Some(stored) = model.value.as_str() {
            let passphrase = decrypt_passphrase_at_rest(stored).map_err(|e| {
                tracing::error!("Failed to decrypt auto-backup passphrase: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            if !stored.starts_with(ENCRYPTED_PREFIX) && is_master_key_configured() {
                let encrypted = encrypt_passphrase_at_rest(&passphrase);
                let _ = upsert_global_setting(db, "auto_backup_passphrase", json!(encrypted), None).await;
            }

            return Ok(passphrase);
        }
    }

    let passphrase = nanoid::nanoid!(32);
    let stored_value = encrypt_passphrase_at_rest(&passphrase);
    upsert_global_setting(db, "auto_backup_passphrase", json!(stored_value), None).await?;

    Ok(passphrase)
}

/// Enforce retention: delete oldest auto-backups beyond the limit
async fn enforce_retention(
    db: &DatabaseConnection,
    storage: &Arc<dyn crate::storage::Storage>,
    tenant_id: Uuid,
    retention_count: i32,
) -> Result<(), StatusCode> {
    let old_backups = app_entity::backup_history::Entity::find()
        .filter(app_entity::backup_history::Column::TenantId.eq(Some(tenant_id)))
        .filter(app_entity::backup_history::Column::IsAutoBackup.eq(true))
        .filter(app_entity::backup_history::Column::Status.eq("completed"))
        .order_by_desc(app_entity::backup_history::Column::CreatedAt)
        .offset(Some(retention_count as u64))
        .all(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for backup in old_backups {
        let _ = storage.delete(&backup.storage_path).await;
        let _ = app_entity::backup_history::Entity::delete_by_id(backup.id)
            .exec(db)
            .await;
        tracing::info!(
            "Retention cleanup: deleted backup {} for tenant {}",
            backup.id,
            tenant_id
        );
    }

    Ok(())
}

// ============================================================================
// GLOBAL BACKUP: SAVE-TO-STORAGE, SCHEDULE, AUTO-BACKUP
// ============================================================================

/// Check if global auto-backup is due and run it
async fn check_and_run_global_auto_backup(
    store: &app_entity::DataStore,
    db: &DatabaseConnection,
    storage: &Arc<dyn crate::storage::Storage>,
    circuit_breaker: &Arc<app_core::circuit_breaker::CircuitBreaker>,
    semaphore: &Arc<tokio::sync::Semaphore>,
) -> Result<(), StatusCode> {
    let enabled_setting = app_entity::global_settings::Entity::find_by_id("global_auto_backup_enabled")
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let is_enabled = match enabled_setting {
        Some(m) => m.value.as_bool().unwrap_or(false) || m.value.as_str() == Some("true"),
        None => false,
    };
    if !is_enabled {
        return Ok(());
    }

    check_global_backup_enabled(store).await?;

    let cron_setting = app_entity::global_settings::Entity::find_by_id("global_auto_backup_cron")
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cron_expr = cron_setting
        .and_then(|m| m.value.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "0 3 * * 0".to_string());

    let schedule = match normalize_cron(&cron_expr).parse::<cron::Schedule>() {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let last_backup = app_entity::backup_history::Entity::find()
        .filter(app_entity::backup_history::Column::TenantId.is_null())
        .filter(app_entity::backup_history::Column::IsAutoBackup.eq(true))
        .order_by_desc(app_entity::backup_history::Column::CreatedAt)
        .one(db)
        .await
        .ok()
        .flatten();

    let last_time: Option<chrono::DateTime<Utc>> = last_backup.map(|b| b.created_at.into());

    let now = Utc::now();
    let should_run = if let Some(prev_time) = schedule
        .after(&(now - chrono::Duration::hours(25)))
        .take(1)
        .next()
    {
        if prev_time <= now {
            match last_time {
                Some(lt) => prev_time > lt,
                None => true,
            }
        } else {
            false
        }
    } else {
        false
    };

    if !should_run {
        return Ok(());
    }

    let permit = semaphore
        .clone()
        .try_acquire_owned()
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;

    tracing::info!("Running global auto-backup");

    let result = run_auto_backup_global(store, db, storage, circuit_breaker).await;

    match result {
        Ok((size, duration_ms)) => {
            tracing::info!(
                "Global auto-backup completed: {}KB in {}ms",
                size / 1024,
                duration_ms
            );
            // Enforce retention
            let retention_setting = app_entity::global_settings::Entity::find_by_id("global_auto_backup_retention_count")
                .one(db)
                .await
                .ok()
                .flatten();
            let retention = retention_setting
                .and_then(|m| {
                    m.value.as_i64()
                        .or_else(|| m.value.as_str().and_then(|s| s.parse().ok()))
                })
                .unwrap_or(5) as i32;
            let _ = enforce_global_retention(db, storage, retention).await;
        }
        Err(e) => {
            tracing::error!("Global auto-backup failed: {:?}", e);
        }
    }

    drop(permit);
    Ok(())
}

/// Build global backup payload — returns (encrypted_bytes, filename, sections_list)
async fn build_global_backup_payload(
    store: &app_entity::DataStore,
    db: &DatabaseConnection,
    user_id: Uuid,
    sections: &[String],
    passphrase: &str,
) -> Result<(Vec<u8>, String, Vec<String>), StatusCode> {
    let valid = ["global_settings", "global_email_templates"];
    let selected: Vec<String> = if sections.is_empty() {
        valid.iter().map(|s| s.to_string()).collect()
    } else {
        sections
            .iter()
            .filter(|s| valid.contains(&s.as_str()))
            .cloned()
            .collect()
    };

    let mut backup = json!({
        "_meta": {
            "format": "clovalink-backup",
            "format_version": 1,
            "clovalink_version": CURRENT_VERSION,
            "export_type": "global",
            "exported_at": Utc::now().to_rfc3339(),
            "exported_by": user_id.to_string(),
            "sections": &selected
        }
    });

    let backup_map = backup.as_object_mut().unwrap();

    for section in &selected {
        let value = match section.as_str() {
            "global_settings" => strip_sensitive_keys(collect_global_settings(store).await?),
            "global_email_templates" => collect_global_email_templates(db).await?,
            _ => continue,
        };
        backup_map.insert(section.clone(), value);
    }

    let plaintext = serde_json::to_vec(&backup).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let encrypted = encrypt_backup(&plaintext, passphrase)?;
    let encrypted_bytes =
        serde_json::to_vec_pretty(&encrypted).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!(
        "clovalink-global-backup-{}-{:06x}.clovalink.json",
        Utc::now().format("%Y%m%d-%H%M%S"),
        rand::random::<u32>() & 0xFFFFFF
    );

    Ok((encrypted_bytes, filename, selected))
}

/// POST /api/backup/global/save
/// Save a global backup to storage
pub async fn save_global_backup_to_storage(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Query(params): Query<ExportParams>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    check_global_backup_enabled(&state.store).await?;
    let _permit = check_backup_infra(&state)?;

    if is_backup_locked_out(&state.store, auth.user_id).await? {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;
    let passphrase = get_passphrase(&headers)?;
    if passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let start = std::time::Instant::now();

    let sections: Vec<String> = params
        .sections
        .as_ref()
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let (encrypted_bytes, filename, selected_sections) =
        build_global_backup_payload(&state.store, state.store.db(), auth.user_id, &sections, &passphrase)
            .await
            .map_err(|e| {
                state.backup_circuit_breaker.record_failure();
                e
            })?;

    let storage_path = format!("_backups/{}", filename);
    let size_bytes = encrypted_bytes.len() as i64;

    state
        .storage
        .upload(&storage_path, encrypted_bytes)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save global backup to storage: {:?}", e);
            state.backup_circuit_breaker.record_failure();
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let duration_ms = start.elapsed().as_millis() as i32;

    let record_id = state
        .store
        .backup()
        .record_backup(
            None,
            filename.clone(),
            storage_path.clone(),
            size_bytes,
            json!(selected_sections),
            false,
            duration_ms,
            Some(auth.user_id),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to record global backup history: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state.backup_circuit_breaker.record_success();

    log_backup_audit(
        &state.store,
        auth.tenant_id,
        auth.user_id,
        "backup_global_save",
        json!({
            "filename": filename,
            "size_bytes": size_bytes,
            "duration_ms": duration_ms,
            "sections": selected_sections,
        }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    )
    .await;

    Ok(Json(json!({
        "success": true,
        "id": record_id,
        "filename": filename,
        "size_bytes": size_bytes,
        "duration_ms": duration_ms,
    })))
}

/// GET /api/backup/global/schedule
pub async fn get_global_backup_schedule(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let rows = app_entity::global_settings::Entity::find()
        .filter(app_entity::global_settings::Column::Key.starts_with("global_auto_backup_"))
        .all(state.store.db())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut enabled = false;
    let mut cron = "0 3 * * 0".to_string();
    let mut retention = 5;

    for row in rows {
        match row.key.as_str() {
            "global_auto_backup_enabled" => {
                enabled = row.value.as_bool().unwrap_or(false)
                    || row.value.as_str().map(|s| s == "true").unwrap_or(false);
            }
            "global_auto_backup_cron" => {
                cron = row.value.as_str().unwrap_or("0 3 * * 0").to_string();
            }
            "global_auto_backup_retention_count" => {
                retention = row.value
                    .as_i64()
                    .unwrap_or(row.value.as_str().and_then(|s| s.parse().ok()).unwrap_or(5))
                    as i32;
            }
            _ => {}
        }
    }

    Ok(Json(json!({
        "enabled": enabled,
        "cron": cron,
        "retention_count": retention
    })))
}

/// PUT /api/backup/global/schedule
pub async fn set_global_backup_schedule(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    headers: HeaderMap,
    Json(body): Json<GlobalScheduleRequest>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    verify_password_confirmation(&state.store, auth.user_id, &headers).await?;

    if let Some(enabled) = body.enabled {
        if enabled && !is_master_key_configured() {
            return Ok(Json(json!({
                "success": false,
                "error": "BACKUP_MASTER_KEY must be configured to enable auto-backups. See deployment docs."
            })));
        }
        upsert_global_setting(state.store.db(), "global_auto_backup_enabled", json!(enabled), Some(auth.user_id)).await?;
    }

    if let Some(ref cron_expr) = body.cron {
        if normalize_cron(cron_expr)
            .parse::<cron::Schedule>()
            .is_err()
        {
            return Ok(Json(
                json!({ "success": false, "error": "Invalid cron expression" }),
            ));
        }
        upsert_global_setting(state.store.db(), "global_auto_backup_cron", json!(cron_expr), Some(auth.user_id)).await?;
    }

    if let Some(retention) = body.retention_count {
        upsert_global_setting(state.store.db(), "global_auto_backup_retention_count", json!(retention), Some(auth.user_id)).await?;
    }

    if let Some(ref cache) = state.cache {
        let _ = cache
            .delete(&app_core::cache::keys::global_settings())
            .await;
    }

    log_backup_audit(
        &state.store, auth.tenant_id, auth.user_id,
        "backup_global_schedule_update",
        json!({ "enabled": body.enabled, "cron": body.cron, "retention_count": body.retention_count }),
        auth.ip_address.as_deref().unwrap_or("unknown"),
    ).await;

    Ok(Json(json!({ "success": true })))
}

/// Run an automatic global backup
async fn run_auto_backup_global(
    store: &app_entity::DataStore,
    db: &DatabaseConnection,
    storage: &Arc<dyn crate::storage::Storage>,
    circuit_breaker: &Arc<app_core::circuit_breaker::CircuitBreaker>,
) -> Result<(i64, i32), StatusCode> {
    let start = std::time::Instant::now();
    let passphrase = get_or_create_auto_passphrase(db).await?;

    let sections = vec![
        "global_settings".to_string(),
        "global_email_templates".to_string(),
    ];

    let (encrypted_bytes, filename, selected) =
        build_global_backup_payload(store, db, Uuid::nil(), &sections, &passphrase).await?;

    let storage_path = format!("_backups/{}", filename);
    let size_bytes = encrypted_bytes.len() as i64;

    storage
        .upload(&storage_path, encrypted_bytes)
        .await
        .map_err(|e| {
            tracing::error!("Global auto-backup storage upload failed: {:?}", e);
            circuit_breaker.record_failure();
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let duration_ms = start.elapsed().as_millis() as i32;
    circuit_breaker.record_success();

    store
        .backup()
        .record_backup(
            None,
            filename.clone(),
            storage_path.clone(),
            size_bytes,
            json!(selected),
            true,
            duration_ms,
            None,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_backup_audit(
        store,
        Uuid::nil(),
        Uuid::nil(),
        "backup_global_auto",
        json!({ "filename": filename, "size_bytes": size_bytes, "duration_ms": duration_ms }),
        "system",
    )
    .await;

    Ok((size_bytes, duration_ms))
}

/// Enforce retention for global auto-backups
async fn enforce_global_retention(
    db: &DatabaseConnection,
    storage: &Arc<dyn crate::storage::Storage>,
    retention_count: i32,
) -> Result<(), StatusCode> {
    let old_backups = app_entity::backup_history::Entity::find()
        .filter(app_entity::backup_history::Column::TenantId.is_null())
        .filter(app_entity::backup_history::Column::IsAutoBackup.eq(true))
        .filter(app_entity::backup_history::Column::Status.eq("completed"))
        .order_by_desc(app_entity::backup_history::Column::CreatedAt)
        .offset(Some(retention_count as u64))
        .all(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for backup in old_backups {
        let _ = storage.delete(&backup.storage_path).await;
        let _ = app_entity::backup_history::Entity::delete_by_id(backup.id)
            .exec(db)
            .await;
        tracing::info!("Global retention cleanup: deleted backup {}", backup.id);
    }

    Ok(())
}
