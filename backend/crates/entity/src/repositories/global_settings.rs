use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Statement,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{entities::global_settings, DataResult};

pub struct GlobalSettingsRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> GlobalSettingsRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn all(&self) -> DataResult<Vec<global_settings::Model>> {
        Ok(global_settings::Entity::find()
            .order_by_asc(global_settings::Column::Key)
            .all(self.db)
            .await?)
    }

    pub async fn get(&self, key: &str) -> DataResult<Option<global_settings::Model>> {
        Ok(global_settings::Entity::find_by_id(key.to_owned())
            .one(self.db)
            .await?)
    }

    pub async fn upsert(&self, key: &str, value: Value, updated_by: Uuid) -> DataResult<()> {
        let model = global_settings::ActiveModel {
            key: Set(key.to_owned()),
            value: Set(value),
            updated_at: Set(Some(chrono::Utc::now().into())),
            updated_by: Set(Some(updated_by)),
        };
        global_settings::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(global_settings::Column::Key)
                    .update_columns([
                        global_settings::Column::Value,
                        global_settings::Column::UpdatedAt,
                        global_settings::Column::UpdatedBy,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> DataResult<bool> {
        Ok(global_settings::Entity::delete_many()
            .filter(global_settings::Column::Key.eq(key))
            .exec(self.db)
            .await?
            .rows_affected
            > 0)
    }

    pub async fn audit(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        action: &str,
        metadata: Value,
        ip_address: Option<&str>,
    ) -> DataResult<()> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(),
            "INSERT INTO audit_logs (tenant_id,user_id,action,resource_type,metadata,ip_address) VALUES ($1,$2,$3,'global_settings',$4,$5::inet)",
            vec![tenant_id.into(), user_id.into(), action.into(), metadata.into(), ip_address.into()]);
        self.db.execute(stmt).await?;
        Ok(())
    }
}
