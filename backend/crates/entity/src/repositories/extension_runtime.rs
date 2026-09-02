use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, TransactionTrait,
};
use uuid::Uuid;

use crate::{
    entities::{
        automation_jobs, extension_event_triggers, extension_installations, extension_versions,
        extension_webhook_logs, extensions,
    },
    DataResult,
};

pub struct NewExtension {
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub extension_type: String,
    pub manifest_url: String,
    pub webhook_url: Option<String>,
    pub public_key: String,
    pub signature_algorithm: String,
    pub allowed_tenant_ids: Option<Vec<Uuid>>,
    pub version: String,
    pub manifest: serde_json::Value,
    pub trigger_filter: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstalledExtensionRow {
    pub installation: extension_installations::Model,
    pub extension: extensions::Model,
    pub version: extension_versions::Model,
}

pub struct NewWebhookLog {
    pub extension_id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub payload: Option<serde_json::Value>,
    pub request_headers: Option<serde_json::Value>,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub duration_ms: Option<i32>,
    pub error_message: Option<String>,
}

pub struct ExtensionRuntimeRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ExtensionRuntimeRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn active_file_processors(
        &self,
        tenant_id: Uuid,
    ) -> DataResult<Vec<extensions::Model>> {
        let installations = extension_installations::Entity::find()
            .filter(extension_installations::Column::TenantId.eq(tenant_id))
            .filter(extension_installations::Column::Enabled.eq(true))
            .all(self.db)
            .await?;
        let mut result = Vec::new();
        for installation in installations {
            if let Some(extension) = extensions::Entity::find_by_id(installation.extension_id)
                .one(self.db)
                .await?
            {
                if extension.status == "active" && extension.extension_type == "file_processor" {
                    result.push(extension);
                }
            }
        }
        Ok(result)
    }

    pub async fn register(&self, value: NewExtension) -> DataResult<extensions::Model> {
        let txn = self.db.begin().await?;
        let extension = extensions::ActiveModel {
            tenant_id: Set(value.tenant_id),
            name: Set(value.name),
            slug: Set(value.slug),
            description: Set(value.description),
            extension_type: Set(value.extension_type),
            manifest_url: Set(value.manifest_url),
            webhook_url: Set(value.webhook_url),
            public_key: Set(Some(value.public_key)),
            signature_algorithm: Set(value.signature_algorithm),
            allowed_tenant_ids: Set(value.allowed_tenant_ids),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        extension_versions::ActiveModel {
            extension_id: Set(extension.id),
            version: Set(value.version),
            manifest: Set(value.manifest),
            is_current: Set(true),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        if let Some(filter) = value.trigger_filter {
            extension_event_triggers::ActiveModel {
                extension_id: Set(extension.id),
                event_type: Set("file_uploaded".into()),
                filter_config: Set(filter),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }
        txn.commit().await?;
        Ok(extension)
    }

    pub async fn extension(&self, id: Uuid) -> DataResult<Option<extensions::Model>> {
        Ok(extensions::Entity::find_by_id(id).one(self.db).await?)
    }

    pub async fn current_version(
        &self,
        extension_id: Uuid,
    ) -> DataResult<Option<extension_versions::Model>> {
        Ok(extension_versions::Entity::find()
            .filter(extension_versions::Column::ExtensionId.eq(extension_id))
            .filter(extension_versions::Column::IsCurrent.eq(true))
            .one(self.db)
            .await?)
    }

    async fn installation(
        &self,
        extension_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<Option<extension_installations::Model>> {
        Ok(extension_installations::Entity::find()
            .filter(extension_installations::Column::ExtensionId.eq(extension_id))
            .filter(extension_installations::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?)
    }

    pub async fn create_installation(
        &self,
        extension_id: Uuid,
        tenant_id: Uuid,
        version_id: Uuid,
        settings: serde_json::Value,
        installed_by: Uuid,
    ) -> DataResult<Option<extension_installations::Model>> {
        if self.installation(extension_id, tenant_id).await?.is_some() {
            return Ok(None);
        }
        Ok(Some(
            extension_installations::ActiveModel {
                extension_id: Set(extension_id),
                tenant_id: Set(tenant_id),
                version_id: Set(version_id),
                settings: Set(settings),
                installed_by: Set(Some(installed_by)),
                ..Default::default()
            }
            .insert(self.db)
            .await?,
        ))
    }

    pub async fn accessible_extensions(
        &self,
        tenant_id: Uuid,
    ) -> DataResult<Vec<(extensions::Model, Option<extension_versions::Model>)>> {
        let models = extensions::Entity::find()
            .filter(extensions::Column::Status.eq("active"))
            .order_by_desc(extensions::Column::CreatedAt)
            .all(self.db)
            .await?;
        let mut result = Vec::new();
        for model in models {
            if model.tenant_id == tenant_id
                || model
                    .allowed_tenant_ids
                    .as_ref()
                    .is_some_and(|ids| ids.contains(&tenant_id))
            {
                let version = self.current_version(model.id).await?;
                result.push((model, version));
            }
        }
        Ok(result)
    }

    pub async fn installed_extensions(
        &self,
        tenant_id: Uuid,
    ) -> DataResult<Vec<InstalledExtensionRow>> {
        let installs = extension_installations::Entity::find()
            .filter(extension_installations::Column::TenantId.eq(tenant_id))
            .order_by_desc(extension_installations::Column::InstalledAt)
            .all(self.db)
            .await?;
        let mut result = Vec::new();
        for installation in installs {
            if let (Some(extension), Some(version)) = (
                extensions::Entity::find_by_id(installation.extension_id)
                    .one(self.db)
                    .await?,
                extension_versions::Entity::find_by_id(installation.version_id)
                    .one(self.db)
                    .await?,
            ) {
                result.push(InstalledExtensionRow {
                    installation,
                    extension,
                    version,
                });
            }
        }
        Ok(result)
    }

    pub async fn active_ui_manifests(
        &self,
        tenant_id: Uuid,
    ) -> DataResult<Vec<(Uuid, serde_json::Value)>> {
        Ok(self
            .installed_extensions(tenant_id)
            .await?
            .into_iter()
            .filter(|row| {
                row.installation.enabled
                    && row.extension.status == "active"
                    && row.extension.extension_type == "ui"
            })
            .map(|row| (row.extension.id, row.version.manifest))
            .collect())
    }

    pub async fn update_installation(
        &self,
        extension_id: Uuid,
        tenant_id: Uuid,
        enabled: Option<bool>,
        settings: Option<serde_json::Value>,
    ) -> DataResult<bool> {
        let Some(model) = extension_installations::Entity::find()
            .filter(extension_installations::Column::ExtensionId.eq(extension_id))
            .filter(extension_installations::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?
        else {
            return Ok(false);
        };
        let mut active: extension_installations::ActiveModel = model.clone().into();
        if let Some(v) = enabled {
            active.enabled = Set(v);
        }
        if let Some(value) = settings {
            let mut current = model.settings.as_object().cloned().unwrap_or_default();
            if let Some(new) = value.as_object() {
                for (k, v) in new {
                    current.insert(k.clone(), v.clone());
                }
            }
            active.settings = Set(serde_json::Value::Object(current));
        }
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn update_access(
        &self,
        extension_id: Uuid,
        owner_id: Uuid,
        allowed: Vec<Uuid>,
    ) -> DataResult<Option<bool>> {
        let Some(model) = extensions::Entity::find_by_id(extension_id)
            .one(self.db)
            .await?
        else {
            return Ok(None);
        };
        if model.tenant_id != owner_id {
            return Ok(Some(false));
        }
        let mut active: extensions::ActiveModel = model.into();
        active.allowed_tenant_ids = Set(Some(allowed));
        active.update(self.db).await?;
        Ok(Some(true))
    }

    pub async fn uninstall(&self, extension_id: Uuid, tenant_id: Uuid) -> DataResult<bool> {
        let Some(installation) = extension_installations::Entity::find()
            .filter(extension_installations::Column::ExtensionId.eq(extension_id))
            .filter(extension_installations::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?
        else {
            return Ok(false);
        };
        let txn = self.db.begin().await?;
        automation_jobs::Entity::delete_many()
            .filter(automation_jobs::Column::ExtensionId.eq(extension_id))
            .filter(automation_jobs::Column::TenantId.eq(tenant_id))
            .exec(&txn)
            .await?;
        extension_installations::Entity::delete_by_id(installation.id)
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(true)
    }

    pub async fn tenant_job_extension_name(
        &self,
        job_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<Option<String>> {
        let Some(job) = automation_jobs::Entity::find_by_id(job_id)
            .one(self.db)
            .await?
            .filter(|j| j.tenant_id == tenant_id)
        else {
            return Ok(None);
        };
        Ok(extensions::Entity::find_by_id(job.extension_id)
            .one(self.db)
            .await?
            .map(|e| e.name))
    }

    pub async fn is_enabled_automation(
        &self,
        extension_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<bool> {
        let Some(inst) = extension_installations::Entity::find()
            .filter(extension_installations::Column::ExtensionId.eq(extension_id))
            .filter(extension_installations::Column::TenantId.eq(tenant_id))
            .filter(extension_installations::Column::Enabled.eq(true))
            .one(self.db)
            .await?
        else {
            return Ok(false);
        };
        Ok(extensions::Entity::find_by_id(inst.extension_id)
            .one(self.db)
            .await?
            .is_some_and(|e| e.extension_type == "automation"))
    }

    pub async fn webhook_logs(
        &self,
        extension_id: Uuid,
        tenant_id: Uuid,
        limit: u64,
    ) -> DataResult<Vec<extension_webhook_logs::Model>> {
        Ok(extension_webhook_logs::Entity::find()
            .filter(extension_webhook_logs::Column::ExtensionId.eq(extension_id))
            .filter(extension_webhook_logs::Column::TenantId.eq(tenant_id))
            .order_by_desc(extension_webhook_logs::Column::CreatedAt)
            .limit(limit)
            .all(self.db)
            .await?)
    }

    pub async fn log_webhook(&self, value: NewWebhookLog) -> DataResult<()> {
        extension_webhook_logs::ActiveModel {
            extension_id: Set(value.extension_id),
            tenant_id: Set(value.tenant_id),
            event_type: Set(value.event_type),
            payload: Set(value.payload),
            request_headers: Set(value.request_headers),
            response_status: Set(value.response_status),
            response_body: Set(value.response_body),
            duration_ms: Set(value.duration_ms),
            error_message: Set(value.error_message),
            ..Default::default()
        }
        .insert(self.db)
        .await?;
        Ok(())
    }

    pub async fn due_jobs(
        &self,
        now: chrono::DateTime<chrono::FixedOffset>,
        limit: u64,
    ) -> DataResult<Vec<automation_jobs::Model>> {
        use sea_orm::{QueryOrder, QuerySelect};
        Ok(automation_jobs::Entity::find()
            .filter(automation_jobs::Column::Enabled.eq(true))
            .filter(automation_jobs::Column::NextRunAt.lte(now))
            .order_by_asc(automation_jobs::Column::NextRunAt)
            .limit(limit)
            .all(self.db)
            .await?)
    }

    pub async fn active_extension(&self, id: Uuid) -> DataResult<Option<extensions::Model>> {
        Ok(extensions::Entity::find_by_id(id)
            .filter(extensions::Column::Status.eq("active"))
            .one(self.db)
            .await?)
    }

    pub async fn job(&self, id: Uuid) -> DataResult<Option<automation_jobs::Model>> {
        Ok(automation_jobs::Entity::find_by_id(id).one(self.db).await?)
    }

    pub async fn jobs(
        &self,
        extension_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<Vec<automation_jobs::Model>> {
        use sea_orm::QueryOrder;
        Ok(automation_jobs::Entity::find()
            .filter(automation_jobs::Column::ExtensionId.eq(extension_id))
            .filter(automation_jobs::Column::TenantId.eq(tenant_id))
            .order_by_desc(automation_jobs::Column::CreatedAt)
            .all(self.db)
            .await?)
    }

    pub async fn create_job(
        &self,
        extension_id: Uuid,
        tenant_id: Uuid,
        name: String,
        cron: String,
        next_run: chrono::DateTime<chrono::FixedOffset>,
        config: serde_json::Value,
    ) -> DataResult<automation_jobs::Model> {
        Ok(automation_jobs::ActiveModel {
            extension_id: Set(extension_id),
            tenant_id: Set(tenant_id),
            name: Set(name),
            cron_expression: Set(Some(cron)),
            next_run_at: Set(next_run),
            config: Set(config),
            ..Default::default()
        }
        .insert(self.db)
        .await?)
    }

    pub async fn update_job_result(
        &self,
        id: Uuid,
        status: String,
        error: Option<String>,
    ) -> DataResult<()> {
        let Some(model) = automation_jobs::Entity::find_by_id(id).one(self.db).await? else {
            return Ok(());
        };
        let mut active: automation_jobs::ActiveModel = model.into();
        active.last_run_at = Set(Some(chrono::Utc::now().fixed_offset()));
        active.last_status = Set(Some(status));
        active.last_error = Set(error);
        active.update(self.db).await?;
        Ok(())
    }

    pub async fn schedule_job(
        &self,
        id: Uuid,
        next_run: chrono::DateTime<chrono::FixedOffset>,
    ) -> DataResult<()> {
        let Some(model) = automation_jobs::Entity::find_by_id(id).one(self.db).await? else {
            return Ok(());
        };
        let mut active: automation_jobs::ActiveModel = model.into();
        active.next_run_at = Set(next_run);
        active.update(self.db).await?;
        Ok(())
    }

    pub async fn set_job_enabled(&self, id: Uuid, enabled: bool) -> DataResult<()> {
        let Some(model) = automation_jobs::Entity::find_by_id(id).one(self.db).await? else {
            return Ok(());
        };
        let mut active: automation_jobs::ActiveModel = model.into();
        active.enabled = Set(enabled);
        active.update(self.db).await?;
        Ok(())
    }

    pub async fn delete_job(&self, id: Uuid) -> DataResult<()> {
        automation_jobs::Entity::delete_by_id(id)
            .exec(self.db)
            .await?;
        Ok(())
    }
}
