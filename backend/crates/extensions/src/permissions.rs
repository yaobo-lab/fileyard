//! Extension permission enforcement

use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

/// Available permissions for extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    ReadFiles,
    WriteFiles,
    ReadCompany,
    ReadEmployees,
    AutomationRun,
    FileProcessorRun,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::ReadFiles => "read:files",
            Permission::WriteFiles => "write:files",
            Permission::ReadCompany => "read:company",
            Permission::ReadEmployees => "read:employees",
            Permission::AutomationRun => "automation:run",
            Permission::FileProcessorRun => "file_processor:run",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read:files" => Some(Permission::ReadFiles),
            "write:files" => Some(Permission::WriteFiles),
            "read:company" => Some(Permission::ReadCompany),
            "read:employees" => Some(Permission::ReadEmployees),
            "automation:run" => Some(Permission::AutomationRun),
            "file_processor:run" => Some(Permission::FileProcessorRun),
            _ => None,
        }
    }

    pub fn all() -> Vec<Permission> {
        vec![
            Permission::ReadFiles,
            Permission::WriteFiles,
            Permission::ReadCompany,
            Permission::ReadEmployees,
            Permission::AutomationRun,
            Permission::FileProcessorRun,
        ]
    }

    pub fn description(&self) -> &'static str {
        match self {
            Permission::ReadFiles => "Read file metadata and contents",
            Permission::WriteFiles => "Upload, modify, and delete files",
            Permission::ReadCompany => "Read company information and settings",
            Permission::ReadEmployees => "Read employee/user information",
            Permission::AutomationRun => "Execute automation tasks on schedule",
            Permission::FileProcessorRun => "Process files when uploaded",
        }
    }

    /// Permissions required by each extension type
    pub fn required_for_type(extension_type: &str) -> Vec<Permission> {
        match extension_type {
            "file_processor" => vec![Permission::FileProcessorRun],
            "automation" => vec![Permission::AutomationRun],
            "ui" => vec![], // UI extensions don't have required permissions
            _ => vec![],
        }
    }
}

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Permission denied: {0}")]
    Denied(String),
    #[error("Extension not installed for this tenant")]
    NotInstalled,
    #[error("Extension is disabled")]
    Disabled,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Check if an extension installation has a specific permission
pub async fn check_permission(
    pool: &PgPool,
    installation_id: Uuid,
    permission: Permission,
) -> Result<bool, PermissionError> {
    let result = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM extension_permissions
            WHERE installation_id = $1 AND permission = $2
        ) as "exists!"
        "#,
        installation_id,
        permission.as_str()
    )
    .fetch_one(pool)
    .await
    .map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

    Ok(result.exists)
}

/// Check if an extension has permission for a specific tenant
pub async fn check_extension_permission(
    pool: &PgPool,
    extension_id: Uuid,
    tenant_id: Uuid,
    permission: Permission,
) -> Result<bool, PermissionError> {
    let result = sqlx::query!(
        r#"
        SELECT ep.permission
        FROM extension_permissions ep
        JOIN extension_installations ei ON ep.installation_id = ei.id
        WHERE ei.extension_id = $1 
          AND ei.tenant_id = $2 
          AND ei.enabled = true
          AND ep.permission = $3
        "#,
        extension_id,
        tenant_id,
        permission.as_str()
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

    Ok(result.is_some())
}

/// Require a permission, returning an error if not granted
pub async fn require_permission(
    pool: &PgPool,
    extension_id: Uuid,
    tenant_id: Uuid,
    permission: Permission,
) -> Result<(), PermissionError> {
    // First check if extension is installed and enabled
    let installation = sqlx::query!(
        r#"
        SELECT id, enabled FROM extension_installations
        WHERE extension_id = $1 AND tenant_id = $2
        "#,
        extension_id,
        tenant_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

    let installation = installation.ok_or(PermissionError::NotInstalled)?;

    if !installation.enabled {
        return Err(PermissionError::Disabled);
    }

    // Check the specific permission
    let has_perm = check_permission(pool, installation.id, permission).await?;

    if !has_perm {
        return Err(PermissionError::Denied(format!(
            "Extension does not have '{}' permission",
            permission.as_str()
        )));
    }

    Ok(())
}

/// Get all permissions for an installation
pub async fn get_installation_permissions(
    pool: &PgPool,
    installation_id: Uuid,
) -> Result<Vec<String>, PermissionError> {
    let permissions = sqlx::query!(
        r#"
        SELECT permission FROM extension_permissions
        WHERE installation_id = $1
        "#,
        installation_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

    Ok(permissions.into_iter().map(|p| p.permission).collect())
}

/// Grant permissions to an installation
pub async fn grant_permissions(
    pool: &PgPool,
    installation_id: Uuid,
    permissions: &[String],
) -> Result<(), PermissionError> {
    for perm in permissions {
        // Validate permission string
        if Permission::from_str(perm).is_none() {
            return Err(PermissionError::Denied(format!(
                "Invalid permission: {}",
                perm
            )));
        }

        sqlx::query!(
            r#"
            INSERT INTO extension_permissions (installation_id, permission)
            VALUES ($1, $2)
            ON CONFLICT (installation_id, permission) DO NOTHING
            "#,
            installation_id,
            perm
        )
        .execute(pool)
        .await
        .map_err(|e| PermissionError::DatabaseError(e.to_string()))?;
    }

    Ok(())
}

/// Revoke a permission from an installation
pub async fn revoke_permission(
    pool: &PgPool,
    installation_id: Uuid,
    permission: Permission,
) -> Result<(), PermissionError> {
    sqlx::query!(
        r#"
        DELETE FROM extension_permissions
        WHERE installation_id = $1 AND permission = $2
        "#,
        installation_id,
        permission.as_str()
    )
    .execute(pool)
    .await
    .map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Revoke all permissions from an installation
pub async fn revoke_all_permissions(
    pool: &PgPool,
    installation_id: Uuid,
) -> Result<(), PermissionError> {
    sqlx::query!(
        r#"
        DELETE FROM extension_permissions
        WHERE installation_id = $1
        "#,
        installation_id
    )
    .execute(pool)
    .await
    .map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

    Ok(())
}

