use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{entities::{extension_installations, extension_permissions}, DataResult};

#[derive(Debug, Clone)]
pub struct ExtensionAccess { pub installation_id: Uuid, pub enabled: bool }

pub struct ExtensionPermissionRepository<'a> { db: &'a DatabaseConnection }

impl<'a> ExtensionPermissionRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    pub async fn installation(&self, extension_id: Uuid, tenant_id: Uuid) -> DataResult<Option<ExtensionAccess>> {
        Ok(extension_installations::Entity::find()
            .filter(extension_installations::Column::ExtensionId.eq(extension_id))
            .filter(extension_installations::Column::TenantId.eq(tenant_id))
            .one(self.db).await?.map(|row| ExtensionAccess { installation_id: row.id, enabled: row.enabled }))
    }

    pub async fn has(&self, installation_id: Uuid, permission: &str) -> DataResult<bool> {
        Ok(extension_permissions::Entity::find()
            .filter(extension_permissions::Column::InstallationId.eq(installation_id))
            .filter(extension_permissions::Column::Permission.eq(permission))
            .one(self.db).await?.is_some())
    }

    pub async fn has_for_tenant(&self, extension_id: Uuid, tenant_id: Uuid, permission: &str) -> DataResult<bool> {
        let Some(installation) = self.installation(extension_id, tenant_id).await? else { return Ok(false) };
        if !installation.enabled { return Ok(false); }
        self.has(installation.installation_id, permission).await
    }

    pub async fn list(&self, installation_id: Uuid) -> DataResult<Vec<String>> {
        Ok(extension_permissions::Entity::find()
            .filter(extension_permissions::Column::InstallationId.eq(installation_id))
            .all(self.db).await?.into_iter().map(|row| row.permission).collect())
    }

    pub async fn grant(&self, installation_id: Uuid, permissions: &[String]) -> DataResult<()> {
        for permission in permissions {
            if !self.has(installation_id, permission).await? {
                extension_permissions::ActiveModel { installation_id: Set(installation_id), permission: Set(permission.clone()), ..Default::default() }
                    .insert(self.db).await?;
            }
        }
        Ok(())
    }

    pub async fn revoke(&self, installation_id: Uuid, permission: &str) -> DataResult<()> {
        extension_permissions::Entity::delete_many()
            .filter(extension_permissions::Column::InstallationId.eq(installation_id))
            .filter(extension_permissions::Column::Permission.eq(permission)).exec(self.db).await?;
        Ok(())
    }

    pub async fn revoke_all(&self, installation_id: Uuid) -> DataResult<()> {
        extension_permissions::Entity::delete_many()
            .filter(extension_permissions::Column::InstallationId.eq(installation_id)).exec(self.db).await?;
        Ok(())
    }
}
