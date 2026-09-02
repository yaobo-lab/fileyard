use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{entities::{automation_jobs, extension_installations, extension_webhook_logs, extensions}, DataResult};

pub struct NewWebhookLog {
    pub extension_id: Uuid, pub tenant_id: Uuid, pub event_type: String,
    pub payload: Option<serde_json::Value>, pub request_headers: Option<serde_json::Value>,
    pub response_status: Option<i32>, pub response_body: Option<String>,
    pub duration_ms: Option<i32>, pub error_message: Option<String>,
}

pub struct ExtensionRuntimeRepository<'a> { db: &'a DatabaseConnection }

impl<'a> ExtensionRuntimeRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    pub async fn active_file_processors(&self, tenant_id: Uuid) -> DataResult<Vec<extensions::Model>> {
        let installations = extension_installations::Entity::find()
            .filter(extension_installations::Column::TenantId.eq(tenant_id))
            .filter(extension_installations::Column::Enabled.eq(true)).all(self.db).await?;
        let mut result = Vec::new();
        for installation in installations {
            if let Some(extension) = extensions::Entity::find_by_id(installation.extension_id).one(self.db).await? {
                if extension.status == "active" && extension.extension_type == "file_processor" { result.push(extension); }
            }
        }
        Ok(result)
    }

    pub async fn log_webhook(&self, value: NewWebhookLog) -> DataResult<()> {
        extension_webhook_logs::ActiveModel {
            extension_id: Set(value.extension_id), tenant_id: Set(value.tenant_id), event_type: Set(value.event_type),
            payload: Set(value.payload), request_headers: Set(value.request_headers), response_status: Set(value.response_status),
            response_body: Set(value.response_body), duration_ms: Set(value.duration_ms), error_message: Set(value.error_message),
            ..Default::default()
        }.insert(self.db).await?;
        Ok(())
    }

    pub async fn due_jobs(&self, now: chrono::DateTime<chrono::FixedOffset>, limit: u64) -> DataResult<Vec<automation_jobs::Model>> {
        use sea_orm::{QueryOrder, QuerySelect};
        Ok(automation_jobs::Entity::find().filter(automation_jobs::Column::Enabled.eq(true))
            .filter(automation_jobs::Column::NextRunAt.lte(now)).order_by_asc(automation_jobs::Column::NextRunAt)
            .limit(limit).all(self.db).await?)
    }

    pub async fn active_extension(&self, id: Uuid) -> DataResult<Option<extensions::Model>> {
        Ok(extensions::Entity::find_by_id(id).filter(extensions::Column::Status.eq("active")).one(self.db).await?)
    }

    pub async fn job(&self, id: Uuid) -> DataResult<Option<automation_jobs::Model>> {
        Ok(automation_jobs::Entity::find_by_id(id).one(self.db).await?)
    }

    pub async fn jobs(&self, extension_id: Uuid, tenant_id: Uuid) -> DataResult<Vec<automation_jobs::Model>> {
        use sea_orm::QueryOrder;
        Ok(automation_jobs::Entity::find().filter(automation_jobs::Column::ExtensionId.eq(extension_id))
            .filter(automation_jobs::Column::TenantId.eq(tenant_id)).order_by_desc(automation_jobs::Column::CreatedAt)
            .all(self.db).await?)
    }

    pub async fn create_job(&self, extension_id: Uuid, tenant_id: Uuid, name: String, cron: String,
        next_run: chrono::DateTime<chrono::FixedOffset>, config: serde_json::Value) -> DataResult<automation_jobs::Model> {
        Ok(automation_jobs::ActiveModel { extension_id:Set(extension_id), tenant_id:Set(tenant_id), name:Set(name),
            cron_expression:Set(Some(cron)), next_run_at:Set(next_run), config:Set(config), ..Default::default() }
            .insert(self.db).await?)
    }

    pub async fn update_job_result(&self, id: Uuid, status: String, error: Option<String>) -> DataResult<()> {
        let Some(model)=automation_jobs::Entity::find_by_id(id).one(self.db).await? else { return Ok(()) };
        let mut active: automation_jobs::ActiveModel=model.into(); active.last_run_at=Set(Some(chrono::Utc::now().fixed_offset()));
        active.last_status=Set(Some(status)); active.last_error=Set(error); active.update(self.db).await?; Ok(())
    }

    pub async fn schedule_job(&self, id: Uuid, next_run: chrono::DateTime<chrono::FixedOffset>) -> DataResult<()> {
        let Some(model)=automation_jobs::Entity::find_by_id(id).one(self.db).await? else { return Ok(()) };
        let mut active: automation_jobs::ActiveModel=model.into(); active.next_run_at=Set(next_run); active.update(self.db).await?; Ok(())
    }

    pub async fn set_job_enabled(&self, id: Uuid, enabled: bool) -> DataResult<()> {
        let Some(model)=automation_jobs::Entity::find_by_id(id).one(self.db).await? else { return Ok(()) };
        let mut active: automation_jobs::ActiveModel=model.into(); active.enabled=Set(enabled); active.update(self.db).await?; Ok(())
    }

    pub async fn delete_job(&self, id: Uuid) -> DataResult<()> {
        automation_jobs::Entity::delete_by_id(id).exec(self.db).await?; Ok(())
    }
}
