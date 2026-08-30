//! Extension manifest validation

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("Failed to fetch manifest: {0}")]
    FetchError(String),
    #[error("Invalid JSON: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid field value: {0}")]
    InvalidValue(String),
}

/// Parsed extension manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub name: String,
    pub slug: String,
    pub version: String,
    #[serde(rename = "type")]
    pub extension_type: String,
    pub description: Option<String>,
    pub entrypoint: Option<String>,
    pub permissions: Vec<String>,
    pub webhook: Option<String>,
    pub ui: Option<UIManifest>,
    pub automation: Option<AutomationManifest>,
    pub file_processor: Option<FileProcessorManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIManifest {
    #[serde(default = "default_load_mode")]
    pub load_mode: String,
    #[serde(default)]
    pub sidebar: Vec<SidebarManifestItem>,
    #[serde(default)]
    pub buttons: Vec<ButtonManifestItem>,
    #[serde(default)]
    pub components: Vec<ComponentManifestItem>,
}

fn default_load_mode() -> String {
    "iframe".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarManifestItem {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub entrypoint: String,
    #[serde(default)]
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonManifestItem {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub location: String, // "file_actions", "toolbar", "context_menu"
    pub entrypoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentManifestItem {
    pub id: String,
    pub name: String,
    pub location: String, // "dashboard", "file_details", "sidebar_panel"
    pub entrypoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationManifest {
    pub default_cron: Option<String>,
    pub configurable: bool,
    pub config_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProcessorManifest {
    #[serde(default)]
    pub file_types: Vec<String>, // e.g., ["pdf", "docx", "xlsx"]
    #[serde(default)]
    pub max_file_size_mb: Option<i64>,
    pub async_processing: Option<bool>,
}

/// Valid permissions that can be requested by extensions
pub const VALID_PERMISSIONS: &[&str] = &[
    "read:files",
    "write:files",
    "read:company",
    "read:employees",
    "automation:run",
    "file_processor:run",
];

/// Valid extension types
pub const VALID_EXTENSION_TYPES: &[&str] = &["ui", "file_processor", "automation"];

/// Fetch manifest from URL
pub async fn fetch_manifest(url: &str) -> Result<ExtensionManifest, ManifestError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ManifestError::FetchError(e.to_string()))?;

    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| ManifestError::FetchError(e.to_string()))?;

    if !response.status().is_success() {
        return Err(ManifestError::FetchError(format!(
            "HTTP {} from manifest URL",
            response.status()
        )));
    }

    let manifest: ExtensionManifest = response
        .json()
        .await
        .map_err(|e| ManifestError::ParseError(e.to_string()))?;

    validate_manifest(&manifest)?;

    Ok(manifest)
}

/// Parse manifest from JSON value
pub fn parse_manifest(value: &Value) -> Result<ExtensionManifest, ManifestError> {
    let manifest: ExtensionManifest = serde_json::from_value(value.clone())
        .map_err(|e| ManifestError::ParseError(e.to_string()))?;

    validate_manifest(&manifest)?;

    Ok(manifest)
}

/// Validate manifest structure and values
pub fn validate_manifest(manifest: &ExtensionManifest) -> Result<(), ManifestError> {
    // Validate required fields
    if manifest.name.is_empty() {
        return Err(ManifestError::MissingField("name".to_string()));
    }
    if manifest.slug.is_empty() {
        return Err(ManifestError::MissingField("slug".to_string()));
    }
    if manifest.version.is_empty() {
        return Err(ManifestError::MissingField("version".to_string()));
    }

    // Validate slug format (lowercase alphanumeric with hyphens)
    if !manifest.slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(ManifestError::InvalidValue(
            "slug must be lowercase alphanumeric with hyphens".to_string(),
        ));
    }

    // Validate version format (semver-like)
    let version_parts: Vec<&str> = manifest.version.split('.').collect();
    if version_parts.len() < 2 || version_parts.len() > 3 {
        return Err(ManifestError::InvalidValue(
            "version must be in format X.Y or X.Y.Z".to_string(),
        ));
    }
    for part in &version_parts {
        if part.parse::<u32>().is_err() {
            return Err(ManifestError::InvalidValue(
                "version parts must be numeric".to_string(),
            ));
        }
    }

    // Validate extension type
    if !VALID_EXTENSION_TYPES.contains(&manifest.extension_type.as_str()) {
        return Err(ManifestError::InvalidValue(format!(
            "type must be one of: {}",
            VALID_EXTENSION_TYPES.join(", ")
        )));
    }

    // Validate permissions
    for perm in &manifest.permissions {
        if !VALID_PERMISSIONS.contains(&perm.as_str()) {
            return Err(ManifestError::InvalidValue(format!(
                "invalid permission '{}'. Valid permissions: {}",
                perm,
                VALID_PERMISSIONS.join(", ")
            )));
        }
    }

    // Type-specific validation
    match manifest.extension_type.as_str() {
        "ui" => {
            if manifest.ui.is_none() {
                return Err(ManifestError::MissingField(
                    "ui section required for UI extensions".to_string(),
                ));
            }
            let ui = manifest.ui.as_ref().unwrap();
            if ui.sidebar.is_empty() && ui.buttons.is_empty() && ui.components.is_empty() {
                return Err(ManifestError::ValidationError(
                    "UI extension must define at least one sidebar item, button, or component".to_string(),
                ));
            }
            // Validate load_mode
            if ui.load_mode != "iframe" && ui.load_mode != "esm" {
                return Err(ManifestError::InvalidValue(
                    "ui.load_mode must be 'iframe' or 'esm'".to_string(),
                ));
            }
        }
        "file_processor" => {
            if manifest.webhook.is_none() {
                return Err(ManifestError::MissingField(
                    "webhook URL required for file processor extensions".to_string(),
                ));
            }
            if !manifest.permissions.contains(&"file_processor:run".to_string()) {
                return Err(ManifestError::ValidationError(
                    "file_processor extensions must request 'file_processor:run' permission".to_string(),
                ));
            }
        }
        "automation" => {
            if manifest.webhook.is_none() {
                return Err(ManifestError::MissingField(
                    "webhook URL required for automation extensions".to_string(),
                ));
            }
            if !manifest.permissions.contains(&"automation:run".to_string()) {
                return Err(ManifestError::ValidationError(
                    "automation extensions must request 'automation:run' permission".to_string(),
                ));
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ui_manifest() {
        let json = serde_json::json!({
            "name": "Test Extension",
            "slug": "test-extension",
            "version": "1.0.0",
            "type": "ui",
            "permissions": ["read:files"],
            "ui": {
                "load_mode": "iframe",
                "sidebar": [{
                    "id": "test-sidebar",
                    "name": "Test",
                    "entrypoint": "https://example.com/sidebar"
                }]
            }
        });

        let manifest = parse_manifest(&json).unwrap();
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.extension_type, "ui");
    }

    #[test]
    fn test_invalid_slug() {
        let json = serde_json::json!({
            "name": "Test",
            "slug": "Test Extension", // Invalid: has space and uppercase
            "version": "1.0.0",
            "type": "ui",
            "permissions": [],
            "ui": { "sidebar": [] }
        });

        let result = parse_manifest(&json);
        assert!(result.is_err());
    }
}

