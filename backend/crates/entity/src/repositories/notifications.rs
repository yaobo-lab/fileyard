use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{entities::{email_templates, notification_preferences, notifications, tenant_email_templates, tenant_notification_settings, users}, DataResult};

pub struct NotificationRepository<'a> { db: &'a DatabaseConnection }

impl<'a> NotificationRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    pub async fn email_template(&self, tenant_id: Uuid, key: &str) -> DataResult<Option<(String, String, Option<String>)>> {
        if let Some(model) = tenant_email_templates::Entity::find()
            .filter(tenant_email_templates::Column::TenantId.eq(tenant_id))
            .filter(tenant_email_templates::Column::TemplateKey.eq(key)).one(self.db).await? {
            return Ok(Some((model.subject, model.body_html, model.body_text)));
        }
        Ok(email_templates::Entity::find().filter(email_templates::Column::TemplateKey.eq(key))
            .one(self.db).await?.map(|m| (m.subject, m.body_html, m.body_text)))
    }

    pub async fn create(&self, user_id: Uuid, tenant_id: Uuid, kind: &str, title: &str, message: &str, metadata: serde_json::Value) -> DataResult<notifications::Model> {
        Ok(notifications::ActiveModel { user_id: Set(user_id), tenant_id: Set(tenant_id), notification_type: Set(kind.into()),
            title: Set(title.into()), message: Set(message.into()), metadata: Set(Some(metadata)), ..Default::default() }.insert(self.db).await?)
    }

    pub async fn mark_email_sent(&self, id: Uuid) -> DataResult<()> {
        if let Some(model) = notifications::Entity::find_by_id(id).one(self.db).await? {
            let mut active: notifications::ActiveModel = model.into(); active.email_sent = Set(true); active.update(self.db).await?;
        }
        Ok(())
    }

    pub async fn by_id(&self, id: Uuid) -> DataResult<Option<notifications::Model>> { Ok(notifications::Entity::find_by_id(id).one(self.db).await?) }

    pub async fn tenant_setting(&self, tenant_id: Uuid, event_type: &str, role: &str) -> DataResult<Option<tenant_notification_settings::Model>> {
        let base = tenant_notification_settings::Entity::find()
            .filter(tenant_notification_settings::Column::TenantId.eq(tenant_id))
            .filter(tenant_notification_settings::Column::EventType.eq(event_type));
        if let Some(model) = base.clone().filter(tenant_notification_settings::Column::Role.eq(role)).one(self.db).await? { return Ok(Some(model)); }
        Ok(base.filter(tenant_notification_settings::Column::Role.is_null()).one(self.db).await?)
    }

    pub async fn user_preference(&self, user_id: Uuid, event_type: &str) -> DataResult<notification_preferences::Model> {
        if let Some(model) = notification_preferences::Entity::find()
            .filter(notification_preferences::Column::UserId.eq(user_id))
            .filter(notification_preferences::Column::EventType.eq(event_type)).one(self.db).await? { return Ok(model); }
        Ok(notification_preferences::ActiveModel { user_id: Set(user_id), event_type: Set(event_type.into()),
            email_enabled: Set(true), in_app_enabled: Set(true), ..Default::default() }.insert(self.db).await?)
    }

    pub async fn tenant_admins(&self, tenant_id: Uuid) -> DataResult<Vec<(Uuid, String, String)>> {
        Ok(users::Entity::find().filter(users::Column::TenantId.eq(tenant_id)).filter(users::Column::Status.eq("active"))
            .filter(users::Column::Role.is_in(["SuperAdmin", "Admin"])).all(self.db).await?
            .into_iter().map(|u| (u.id, u.email, u.role)).collect())
    }

    pub async fn user_name(&self, id: Uuid) -> DataResult<Option<String>> {
        Ok(users::Entity::find_by_id(id).one(self.db).await?.map(|u| u.name))
    }
}
