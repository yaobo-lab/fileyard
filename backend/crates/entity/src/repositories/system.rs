use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, Statement,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{entities::files_metadata, DataResult};

#[derive(Debug, Clone)]
pub struct StorageFile {
    pub id: Uuid,
    pub name: String,
    pub storage_path: String,
}

#[derive(Debug, Clone, Copy)]
pub struct DatabasePoolStats {
    pub size: u32,
    pub idle: u32,
    pub in_use: u32,
}

pub struct SystemRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> SystemRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn ping(&self) -> DataResult<()> {
        self.db
            .execute(Statement::from_string(
                self.db.get_database_backend(),
                "SELECT 1".to_owned(),
            ))
            .await?;
        Ok(())
    }

    pub fn pool_stats(&self) -> DatabasePoolStats {
        let pool = self.db.get_postgres_connection_pool();
        let size = pool.size();
        let idle = pool.num_idle() as u32;
        DatabasePoolStats {
            size,
            idle,
            in_use: size.saturating_sub(idle),
        }
    }

    pub async fn active_storage_files(&self) -> DataResult<Vec<StorageFile>> {
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .filter(files_metadata::Column::IsDirectory.eq(false))
            .filter(files_metadata::Column::StoragePath.ne(""))
            .order_by_desc(files_metadata::Column::CreatedAt)
            .all(self.db)
            .await?
            .into_iter()
            .map(|f| StorageFile {
                id: f.id,
                name: f.name,
                storage_path: f.storage_path,
            })
            .collect())
    }

    pub async fn mark_file_deleted(&self, id: Uuid) -> DataResult<()> {
        let Some(file) = files_metadata::Entity::find_by_id(id).one(self.db).await? else {
            return Ok(());
        };
        let mut active: files_metadata::ActiveModel = file.into();
        active.is_deleted = Set(true);
        active.deleted_at = Set(Some(chrono::Utc::now().into()));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(())
    }

    pub async fn audit(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        action: &str,
        resource_type: &str,
        metadata: Value,
        ip: Option<&str>,
    ) -> DataResult<()> {
        self.db.execute(Statement::from_sql_and_values(self.db.get_database_backend(),
            "INSERT INTO audit_logs (tenant_id,user_id,action,resource_type,metadata,ip_address) VALUES ($1,$2,$3,$4,$5,$6::inet)",
            vec![tenant_id.into(),user_id.into(),action.into(),resource_type.into(),metadata.into(),ip.into()])).await?;
        Ok(())
    }
    pub async fn audit_resource(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        action: &str,
        resource_type: &str,
        resource_id: Uuid,
        metadata: Value,
        ip: Option<&str>,
    ) -> DataResult<()> {
        self.db.execute(Statement::from_sql_and_values(self.db.get_database_backend(),"INSERT INTO audit_logs (tenant_id,user_id,action,resource_type,resource_id,metadata,ip_address) VALUES ($1,$2,$3,$4,$5,$6,$7::inet)",vec![tenant_id.into(),user_id.into(),action.into(),resource_type.into(),resource_id.into(),metadata.into(),ip.into()])).await?;
        Ok(())
    }
}
