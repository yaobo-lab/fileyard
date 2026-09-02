use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect};
use uuid::Uuid;

use crate::{entities::files_metadata, DataResult};

pub struct FileRepository<'a> { db: &'a DatabaseConnection }

impl<'a> FileRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    pub async fn by_tenant_id(&self, tenant_id: Uuid, file_id: Uuid) -> DataResult<Option<files_metadata::Model>> {
        Ok(files_metadata::Entity::find_by_id(file_id)
            .filter(files_metadata::Column::TenantId.eq(tenant_id)).one(self.db).await?)
    }

    async fn update(&self, tenant_id: Uuid, file_id: Uuid, apply: impl FnOnce(&mut files_metadata::ActiveModel)) -> DataResult<bool> {
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else { return Ok(false) };
        let mut active: files_metadata::ActiveModel = model.into();
        apply(&mut active);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn rename(&self, tenant_id: Uuid, file_id: Uuid, name: String) -> DataResult<bool> {
        if name.is_empty() || name.contains('\0') || name == "." || name == ".." {
            return Err(crate::DataError::InvalidQuery("invalid filename".into()));
        }
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else { return Ok(false) };
        if model.is_deleted { return Ok(false) }
        self.update(tenant_id, file_id, |active| active.name = Set(name)).await
    }

    pub async fn move_to(&self, tenant_id: Uuid, file_id: Uuid, parent: Option<String>) -> DataResult<bool> {
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else { return Ok(false) };
        if model.is_deleted { return Ok(false) }
        self.update(tenant_id, file_id, |active| active.parent_path = Set(parent)).await
    }

    pub async fn set_deleted(&self, tenant_id: Uuid, file_id: Uuid, deleted: bool) -> DataResult<bool> {
        self.update(tenant_id, file_id, |active| {
            active.is_deleted = Set(deleted);
            active.deleted_at = Set(deleted.then(|| chrono::Utc::now().into()));
        }).await
    }

    pub async fn count_active_content_references(&self, hash: &str) -> DataResult<u64> {
        Ok(files_metadata::Entity::find().filter(files_metadata::Column::ContentHash.eq(hash))
            .filter(files_metadata::Column::IsDeleted.eq(false)).count(self.db).await?)
    }

    pub async fn has_content_references(&self, hash: &str) -> DataResult<bool> {
        Ok(files_metadata::Entity::find().filter(files_metadata::Column::ContentHash.eq(hash)).one(self.db).await?.is_some())
    }

    pub async fn unreferenced_storage_paths(&self, limit: u64) -> DataResult<Vec<(String, String)>> {
        let deleted = files_metadata::Entity::find().filter(files_metadata::Column::IsDeleted.eq(true))
            .filter(files_metadata::Column::ContentHash.is_not_null()).limit(limit).all(self.db).await?;
        let mut result = Vec::new();
        for file in deleted {
            if let Some(hash) = file.content_hash {
                if self.count_active_content_references(&hash).await? == 0 && !result.iter().any(|(_, h)| h == &hash) {
                    result.push((file.storage_path, hash));
                }
            }
        }
        Ok(result)
    }

    pub async fn permanently_delete(&self, tenant_id: Uuid, file_id: Uuid) -> DataResult<bool> {
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else { return Ok(false) };
        if !model.is_deleted { return Ok(false) }
        Ok(files_metadata::Entity::delete_by_id(file_id).exec(self.db).await?.rows_affected > 0)
    }
}
