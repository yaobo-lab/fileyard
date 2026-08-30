//! Extension data models

use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::types::Uuid;

/// Extension type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    Ui,
    FileProcessor,
    Automation,
}

impl ExtensionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtensionType::Ui => "ui",
            ExtensionType::FileProcessor => "file_processor",
            ExtensionType::Automation => "automation",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ui" => Some(ExtensionType::Ui),
            "file_processor" => Some(ExtensionType::FileProcessor),
            "automation" => Some(ExtensionType::Automation),
            _ => None,
        }
    }
}

/// Signature algorithm for webhook verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAlgorithmType {
    HmacSha256,
    Ed25519,
}

impl SignatureAlgorithmType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignatureAlgorithmType::HmacSha256 => "hmac_sha256",
            SignatureAlgorithmType::Ed25519 => "ed25519",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hmac_sha256" => Some(SignatureAlgorithmType::HmacSha256),
            "ed25519" => Some(SignatureAlgorithmType::Ed25519),
            _ => None,
        }
    }
}

/// Extension record from database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Extension {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub extension_type: String,
    pub manifest_url: String,
    pub webhook_url: Option<String>,
    pub public_key: Option<String>,
    pub signature_algorithm: String,
    pub status: String,
    /// List of tenant IDs that can install this extension.
    /// NULL = only owner tenant, empty = disabled, UUIDs = specific tenants allowed
    pub allowed_tenant_ids: Option<Vec<Uuid>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Extension version record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtensionVersion {
    pub id: Uuid,
    pub extension_id: Uuid,
    pub version: String,
    pub manifest: serde_json::Value,
    pub changelog: Option<String>,
    pub is_current: bool,
    pub created_at: DateTime<Utc>,
}

/// Extension installation record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtensionInstallation {
    pub id: Uuid,
    pub extension_id: Uuid,
    pub tenant_id: Uuid,
    pub version_id: Uuid,
    pub enabled: bool,
    pub settings: serde_json::Value,
    pub installed_by: Option<Uuid>,
    pub installed_at: DateTime<Utc>,
}

/// Extension permission record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtensionPermission {
    pub id: Uuid,
    pub installation_id: Uuid,
    pub permission: String,
    pub granted_at: DateTime<Utc>,
}

/// Extension event trigger record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtensionEventTrigger {
    pub id: Uuid,
    pub extension_id: Uuid,
    pub event_type: String,
    pub filter_config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Automation job record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AutomationJob {
    pub id: Uuid,
    pub extension_id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub cron_expression: Option<String>,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Webhook log record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtensionWebhookLog {
    pub id: Uuid,
    pub extension_id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub payload: Option<serde_json::Value>,
    pub request_headers: Option<serde_json::Value>,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ==================== Input DTOs ====================

/// Input for registering a new extension
#[derive(Debug, Deserialize)]
pub struct RegisterExtensionInput {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub manifest_url: String,
    pub webhook_url: Option<String>,
    pub signature_algorithm: Option<String>,
    /// List of tenant IDs that can install this extension.
    /// If not provided, only the registering tenant can install.
    /// Empty array means no one can install (disabled).
    pub allowed_tenant_ids: Option<Vec<Uuid>>,
}

/// Input for installing an extension
#[derive(Debug, Deserialize)]
pub struct InstallExtensionInput {
    pub extension_id: Uuid,
    pub permissions: Vec<String>,
    pub settings: Option<serde_json::Value>,
}

/// Input for validating a manifest
#[derive(Debug, Deserialize)]
pub struct ValidateManifestInput {
    pub manifest_url: Option<String>,
    pub manifest: Option<serde_json::Value>,
}

/// Input for updating extension settings
#[derive(Debug, Deserialize)]
pub struct UpdateExtensionSettingsInput {
    pub enabled: Option<bool>,
    pub settings: Option<serde_json::Value>,
}

/// Input for updating extension access (which companies can install)
#[derive(Debug, Deserialize)]
pub struct UpdateExtensionAccessInput {
    /// List of tenant IDs that can install this extension.
    /// Empty array means no one can install (disabled).
    /// Omit to keep current value.
    pub allowed_tenant_ids: Option<Vec<Uuid>>,
}

/// Input for creating an automation job
#[derive(Debug, Deserialize)]
pub struct CreateAutomationJobInput {
    pub extension_id: Uuid,
    pub name: String,
    pub cron_expression: String,
    pub config: Option<serde_json::Value>,
}

// ==================== Response DTOs ====================

/// Extension with current version info
#[derive(Debug, Serialize)]
pub struct ExtensionWithVersion {
    #[serde(flatten)]
    pub extension: Extension,
    pub current_version: Option<String>,
    pub manifest: Option<serde_json::Value>,
}

/// Installed extension with details
#[derive(Debug, Serialize)]
pub struct InstalledExtension {
    pub installation_id: Uuid,
    pub extension: Extension,
    pub version: String,
    pub enabled: bool,
    pub settings: serde_json::Value,
    pub permissions: Vec<String>,
    pub installed_at: DateTime<Utc>,
}

/// UI extension components for frontend injection
#[derive(Debug, Serialize)]
pub struct UIExtensionComponents {
    pub sidebar: Vec<UISidebarItem>,
    pub buttons: Vec<UIButton>,
    pub components: Vec<UIComponent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UISidebarItem {
    pub id: String,
    pub extension_id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub entrypoint: String,
    pub load_mode: String, // "iframe" or "esm"
    pub order: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UIButton {
    pub id: String,
    pub extension_id: Uuid,
    pub name: String,
    pub icon: Option<String>,
    pub location: String, // "file_actions", "toolbar", etc.
    pub entrypoint: String,
    pub load_mode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UIComponent {
    pub id: String,
    pub extension_id: Uuid,
    pub name: String,
    pub location: String, // "dashboard", "file_details", etc.
    pub entrypoint: String,
    pub load_mode: String,
}

