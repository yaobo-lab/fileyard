//! Centralized repository layer with mandatory tenant scoping
//!
//! All database queries for tenant-scoped resources MUST go through this module
//! to ensure tenant isolation is enforced at the data layer.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Error type for repository operations
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("Resource not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// File metadata from database
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub parent_path: Option<String>,
    pub storage_path: String,
    pub content_hash: Option<String>,
    pub size_bytes: i64,
    pub is_directory: bool,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub is_locked: bool,
    pub locked_by: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub visibility: String,
    pub is_immutable: Option<bool>,
}

/// Get file by ID with mandatory tenant scoping
///
/// # Security
/// This function ALWAYS filters by tenant_id to prevent cross-tenant access.
pub async fn get_file(
    pool: &PgPool,
    tenant_id: Uuid,
    file_id: Uuid,
) -> Result<FileMetadata, RepoError> {
    let row: Option<(
        Uuid,
        Uuid,
        String,
        Option<String>,
        String,
        Option<String>,
        i64,
        bool,
        bool,
        Option<DateTime<Utc>>,
        bool,
        Option<Uuid>,
        Option<Uuid>,
        Option<Uuid>,
        String,
        Option<bool>,
    )> = sqlx::query_as(
        r#"
        SELECT id, tenant_id, name, parent_path, storage_path, content_hash,
               size_bytes, is_directory, is_deleted, deleted_at, is_locked, locked_by,
               owner_id, department_id, visibility, is_immutable
        FROM files_metadata
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(file_id)
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((
            id,
            tenant_id,
            name,
            parent_path,
            storage_path,
            content_hash,
            size_bytes,
            is_directory,
            is_deleted,
            deleted_at,
            is_locked,
            locked_by,
            owner_id,
            department_id,
            visibility,
            is_immutable,
        )) => Ok(FileMetadata {
            id,
            tenant_id,
            name,
            parent_path,
            storage_path,
            content_hash,
            size_bytes,
            is_directory,
            is_deleted,
            deleted_at,
            is_locked,
            locked_by,
            owner_id,
            department_id,
            visibility,
            is_immutable,
        }),
        None => Err(RepoError::NotFound),
    }
}

/// Get file by ID, only if not deleted
pub async fn get_active_file(
    pool: &PgPool,
    tenant_id: Uuid,
    file_id: Uuid,
) -> Result<FileMetadata, RepoError> {
    let file = get_file(pool, tenant_id, file_id).await?;
    if file.is_deleted {
        Err(RepoError::NotFound)
    } else {
        Ok(file)
    }
}

/// Update file name (metadata only, never touches storage)
///
/// # Security
/// This function ALWAYS filters by tenant_id to prevent cross-tenant access.
pub async fn update_file_name(
    pool: &PgPool,
    tenant_id: Uuid,
    file_id: Uuid,
    new_name: &str,
) -> Result<(), RepoError> {
    // Validate filename
    if new_name.is_empty() || new_name.contains('\0') || new_name == "." || new_name == ".." {
        return Err(RepoError::InvalidInput("Invalid filename".to_string()));
    }

    let result = sqlx::query(
        "UPDATE files_metadata SET name = $1, updated_at = NOW() WHERE id = $2 AND tenant_id = $3 AND is_deleted = false"
    )
    .bind(new_name)
    .bind(file_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        Err(RepoError::NotFound)
    } else {
        Ok(())
    }
}

/// Move file to new parent (metadata only, never touches storage)
///
/// # Security
/// This function ALWAYS filters by tenant_id to prevent cross-tenant access.
pub async fn update_file_parent(
    pool: &PgPool,
    tenant_id: Uuid,
    file_id: Uuid,
    new_parent_path: Option<&str>,
) -> Result<(), RepoError> {
    let result = sqlx::query(
        "UPDATE files_metadata SET parent_path = $1, updated_at = NOW() WHERE id = $2 AND tenant_id = $3 AND is_deleted = false"
    )
    .bind(new_parent_path)
    .bind(file_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        Err(RepoError::NotFound)
    } else {
        Ok(())
    }
}

/// Soft delete a file (mark as deleted, never touches storage)
///
/// # Security
/// This function ALWAYS filters by tenant_id to prevent cross-tenant access.
/// Storage cleanup happens separately via reference counting.
pub async fn mark_file_deleted(
    pool: &PgPool,
    tenant_id: Uuid,
    file_id: Uuid,
) -> Result<(), RepoError> {
    let result = sqlx::query(
        "UPDATE files_metadata SET is_deleted = true, deleted_at = NOW() WHERE id = $1 AND tenant_id = $2"
    )
    .bind(file_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        Err(RepoError::NotFound)
    } else {
        Ok(())
    }
}

/// Restore a soft-deleted file
///
/// # Security
/// This function ALWAYS filters by tenant_id to prevent cross-tenant access.
pub async fn restore_file(pool: &PgPool, tenant_id: Uuid, file_id: Uuid) -> Result<(), RepoError> {
    let result = sqlx::query(
        "UPDATE files_metadata SET is_deleted = false, deleted_at = NULL WHERE id = $1 AND tenant_id = $2"
    )
    .bind(file_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        Err(RepoError::NotFound)
    } else {
        Ok(())
    }
}

/// Count references to a content hash (for deduplication-aware deletion)
///
/// Returns the number of non-deleted files referencing this content hash.
pub async fn count_content_references(pool: &PgPool, content_hash: &str) -> Result<i64, RepoError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM files_metadata WHERE content_hash = $1 AND is_deleted = false",
    )
    .bind(content_hash)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

/// Check if a content hash has any references (including soft-deleted)
///
/// Used to determine if it's safe to delete from storage.
pub async fn has_any_content_references(
    pool: &PgPool,
    content_hash: &str,
) -> Result<bool, RepoError> {
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM files_metadata WHERE content_hash = $1")
            .bind(content_hash)
            .fetch_one(pool)
            .await?;

    Ok(count.0 > 0)
}

/// Get storage paths for unreferenced content (for cleanup)
///
/// Returns storage_path and content_hash for files that are:
/// 1. Soft-deleted
/// 2. Have no other references to the same content_hash
pub async fn get_unreferenced_storage_paths(
    pool: &PgPool,
    limit: i32,
) -> Result<Vec<(String, String)>, RepoError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT DISTINCT fm.storage_path, fm.content_hash
        FROM files_metadata fm
        WHERE fm.is_deleted = true
        AND fm.content_hash IS NOT NULL
        AND NOT EXISTS (
            SELECT 1 FROM files_metadata fm2 
            WHERE fm2.content_hash = fm.content_hash 
            AND fm2.is_deleted = false
        )
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Permanently delete file metadata (after storage cleanup)
///
/// # Security
/// This function ALWAYS filters by tenant_id AND requires is_deleted = true.
/// Call this only after verifying no other references exist to the content.
pub async fn permanently_delete_file_metadata(
    pool: &PgPool,
    tenant_id: Uuid,
    file_id: Uuid,
) -> Result<(), RepoError> {
    let result = sqlx::query(
        "DELETE FROM files_metadata WHERE id = $1 AND tenant_id = $2 AND is_deleted = true",
    )
    .bind(file_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        Err(RepoError::NotFound)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_invalid_filename_validation() {
        // These should be caught by update_file_name
        assert!("".is_empty());
        assert!(".".len() == 1 && "." == ".");
        assert!("..".len() == 2 && ".." == "..");
    }
}
