use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder,
};
use uuid::Uuid;

use crate::{
    entities::{email_templates, tenant_email_templates},
    DataResult,
};

pub struct EmailTemplateRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> EmailTemplateRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_global(&self) -> DataResult<Vec<email_templates::Model>> {
        Ok(email_templates::Entity::find()
            .order_by_asc(email_templates::Column::Name)
            .all(self.db)
            .await?)
    }

    pub async fn get_global(&self, key: &str) -> DataResult<Option<email_templates::Model>> {
        Ok(email_templates::Entity::find()
            .filter(email_templates::Column::TemplateKey.eq(key))
            .one(self.db)
            .await?)
    }

    pub async fn update_global(
        &self,
        key: &str,
        subject: String,
        body_html: String,
        body_text: Option<String>,
    ) -> DataResult<Option<email_templates::Model>> {
        let Some(model) = self.get_global(key).await? else {
            return Ok(None);
        };
        let mut active: email_templates::ActiveModel = model.into();
        active.subject = Set(subject);
        active.body_html = Set(body_html);
        active.body_text = Set(body_text);
        active.updated_at = Set(Some(Utc::now().into()));
        Ok(Some(active.update(self.db).await?))
    }

    pub async fn list_tenant(
        &self,
        tenant_id: Uuid,
    ) -> DataResult<Vec<tenant_email_templates::Model>> {
        Ok(tenant_email_templates::Entity::find()
            .filter(tenant_email_templates::Column::TenantId.eq(tenant_id))
            .all(self.db)
            .await?)
    }

    pub async fn get_tenant(
        &self,
        tenant_id: Uuid,
        key: &str,
    ) -> DataResult<Option<tenant_email_templates::Model>> {
        Ok(tenant_email_templates::Entity::find()
            .filter(tenant_email_templates::Column::TenantId.eq(tenant_id))
            .filter(tenant_email_templates::Column::TemplateKey.eq(key))
            .one(self.db)
            .await?)
    }

    pub async fn upsert_tenant(
        &self,
        tenant_id: Uuid,
        key: &str,
        subject: String,
        body_html: String,
        body_text: Option<String>,
    ) -> DataResult<tenant_email_templates::Model> {
        let existing = self.get_tenant(tenant_id, key).await?;
        let now = Utc::now();
        if let Some(model) = existing {
            let mut active: tenant_email_templates::ActiveModel = model.into();
            active.subject = Set(subject);
            active.body_html = Set(body_html);
            active.body_text = Set(body_text);
            active.updated_at = Set(Some(now.into()));
            Ok(active.update(self.db).await?)
        } else {
            let active = tenant_email_templates::ActiveModel {
                id: Set(Uuid::new_v4()),
                tenant_id: Set(tenant_id),
                template_key: Set(key.to_string()),
                subject: Set(subject),
                body_html: Set(body_html),
                body_text: Set(body_text),
                created_at: Set(Some(now.into())),
                updated_at: Set(Some(now.into())),
            };
            Ok(active.insert(self.db).await?)
        }
    }

    pub async fn reset_tenant(&self, tenant_id: Uuid, key: &str) -> DataResult<bool> {
        let res = tenant_email_templates::Entity::delete_many()
            .filter(tenant_email_templates::Column::TenantId.eq(tenant_id))
            .filter(tenant_email_templates::Column::TemplateKey.eq(key))
            .exec(self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn effective_template(
        &self,
        tenant_id: Option<Uuid>,
        key: &str,
    ) -> DataResult<Option<(String, String, Option<String>)>> {
        if let Some(tid) = tenant_id {
            if let Some(t) = self.get_tenant(tid, key).await? {
                return Ok(Some((t.subject, t.body_html, t.body_text)));
            }
        }
        if let Some(g) = self.get_global(key).await? {
            return Ok(Some((g.subject, g.body_html, g.body_text)));
        }
        Ok(None)
    }
}
