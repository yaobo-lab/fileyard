use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use uuid::Uuid;

use crate::entities::departments;
use crate::DataResult;

pub struct DepartmentRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> DepartmentRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list(&self, tenant_id: Uuid) -> DataResult<Vec<departments::Model>> {
        departments::Entity::find()
            .filter(departments::Column::TenantId.eq(tenant_id))
            .order_by_asc(departments::Column::Name)
            .all(self.db)
            .await
            .map_err(Into::into)
    }

    pub async fn create(
        &self,
        tenant_id: Uuid,
        name: String,
        description: Option<String>,
    ) -> DataResult<departments::Model> {
        departments::ActiveModel {
            tenant_id: Set(tenant_id),
            name: Set(name),
            description: Set(description),
            ..Default::default()
        }
        .insert(self.db)
        .await
        .map_err(Into::into)
    }

    pub async fn update(
        &self,
        id: Uuid,
        tenant_id: Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> DataResult<Option<departments::Model>> {
        let Some(model) = departments::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .filter(|row| row.tenant_id == tenant_id)
        else {
            return Ok(None);
        };
        let mut active: departments::ActiveModel = model.into();
        if let Some(value) = name {
            active.name = Set(value);
        }
        if let Some(value) = description {
            active.description = Set(Some(value));
        }
        active.updated_at = Set(chrono::Utc::now().fixed_offset());
        Ok(Some(active.update(self.db).await?))
    }

    pub async fn delete(&self, id: Uuid, tenant_id: Uuid) -> DataResult<bool> {
        Ok(departments::Entity::delete_many()
            .filter(departments::Column::Id.eq(id))
            .filter(departments::Column::TenantId.eq(tenant_id))
            .exec(self.db)
            .await?
            .rows_affected
            > 0)
    }
}
