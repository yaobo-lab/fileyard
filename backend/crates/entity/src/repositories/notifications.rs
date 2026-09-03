use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

use crate::{
    entities::{
        email_templates, notification_preferences, notifications, tenant_email_templates,
        tenant_notification_settings, users,
    },
    DataResult,
};

pub struct NotificationRepository<'a> {
    db: &'a DatabaseConnection,
}
pub struct PreferencePatch {
    pub event_type: String,
    pub email_enabled: Option<bool>,
    pub in_app_enabled: Option<bool>,
}
pub struct TenantNotificationPatch {
    pub event_type: String,
    pub enabled: Option<bool>,
    pub email_enforced: Option<bool>,
    pub in_app_enforced: Option<bool>,
    pub default_email: Option<bool>,
    pub default_in_app: Option<bool>,
}

impl<'a> NotificationRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn email_template(
        &self,
        tenant_id: Uuid,
        key: &str,
    ) -> DataResult<Option<(String, String, Option<String>)>> {
        if let Some(model) = tenant_email_templates::Entity::find()
            .filter(tenant_email_templates::Column::TenantId.eq(tenant_id))
            .filter(tenant_email_templates::Column::TemplateKey.eq(key))
            .one(self.db)
            .await?
        {
            return Ok(Some((model.subject, model.body_html, model.body_text)));
        }
        Ok(email_templates::Entity::find()
            .filter(email_templates::Column::TemplateKey.eq(key))
            .one(self.db)
            .await?
            .map(|m| (m.subject, m.body_html, m.body_text)))
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        kind: &str,
        title: &str,
        message: &str,
        metadata: serde_json::Value,
    ) -> DataResult<notifications::Model> {
        Ok(notifications::ActiveModel {
            user_id: Set(user_id),
            tenant_id: Set(tenant_id),
            notification_type: Set(kind.into()),
            title: Set(title.into()),
            message: Set(message.into()),
            metadata: Set(Some(metadata)),
            ..Default::default()
        }
        .insert(self.db)
        .await?)
    }

    pub async fn mark_email_sent(&self, id: Uuid) -> DataResult<()> {
        if let Some(model) = notifications::Entity::find_by_id(id).one(self.db).await? {
            let mut active: notifications::ActiveModel = model.into();
            active.email_sent = Set(true);
            active.update(self.db).await?;
        }
        Ok(())
    }

    pub async fn by_id(&self, id: Uuid) -> DataResult<Option<notifications::Model>> {
        Ok(notifications::Entity::find_by_id(id).one(self.db).await?)
    }

    pub async fn tenant_setting(
        &self,
        tenant_id: Uuid,
        event_type: &str,
        role: &str,
    ) -> DataResult<Option<tenant_notification_settings::Model>> {
        let base = tenant_notification_settings::Entity::find()
            .filter(tenant_notification_settings::Column::TenantId.eq(tenant_id))
            .filter(tenant_notification_settings::Column::EventType.eq(event_type));
        if let Some(model) = base
            .clone()
            .filter(tenant_notification_settings::Column::Role.eq(role))
            .one(self.db)
            .await?
        {
            return Ok(Some(model));
        }
        Ok(base
            .filter(tenant_notification_settings::Column::Role.is_null())
            .one(self.db)
            .await?)
    }

    pub async fn user_preference(
        &self,
        user_id: Uuid,
        event_type: &str,
    ) -> DataResult<notification_preferences::Model> {
        if let Some(model) = notification_preferences::Entity::find()
            .filter(notification_preferences::Column::UserId.eq(user_id))
            .filter(notification_preferences::Column::EventType.eq(event_type))
            .one(self.db)
            .await?
        {
            return Ok(model);
        }
        Ok(notification_preferences::ActiveModel {
            user_id: Set(user_id),
            event_type: Set(event_type.into()),
            email_enabled: Set(true),
            in_app_enabled: Set(true),
            ..Default::default()
        }
        .insert(self.db)
        .await?)
    }

    pub async fn tenant_admins(&self, tenant_id: Uuid) -> DataResult<Vec<(Uuid, String, String)>> {
        Ok(users::Entity::find()
            .filter(users::Column::TenantId.eq(tenant_id))
            .filter(users::Column::Status.eq("active"))
            .filter(users::Column::Role.is_in(["SuperAdmin", "Admin"]))
            .all(self.db)
            .await?
            .into_iter()
            .map(|u| (u.id, u.email, u.role))
            .collect())
    }

    pub async fn user_name(&self, id: Uuid) -> DataResult<Option<String>> {
        Ok(users::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .map(|u| u.name))
    }
    pub async fn list(
        &self,
        user_id: Uuid,
        unread_only: bool,
        limit: u64,
        offset: u64,
    ) -> DataResult<(Vec<notifications::Model>, i64, i64)> {
        let mut q = notifications::Entity::find().filter(notifications::Column::UserId.eq(user_id));
        if unread_only {
            q = q.filter(notifications::Column::IsRead.eq(false));
        }
        let total = q.clone().count(self.db).await? as i64;
        let rows = q
            .order_by_desc(notifications::Column::CreatedAt)
            .limit(limit)
            .offset(offset)
            .all(self.db)
            .await?;
        let unread = notifications::Entity::find()
            .filter(notifications::Column::UserId.eq(user_id))
            .filter(notifications::Column::IsRead.eq(false))
            .count(self.db)
            .await? as i64;
        Ok((rows, total, unread))
    }
    pub async fn mark_read(&self, id: Uuid, user_id: Uuid) -> DataResult<bool> {
        let Some(m) = notifications::Entity::find_by_id(id)
            .filter(notifications::Column::UserId.eq(user_id))
            .one(self.db)
            .await?
        else {
            return Ok(false);
        };
        let mut a: notifications::ActiveModel = m.into();
        a.is_read = Set(true);
        a.update(self.db).await?;
        Ok(true)
    }
    pub async fn mark_all_read(&self, user_id: Uuid) -> DataResult<u64> {
        let rows = notifications::Entity::find()
            .filter(notifications::Column::UserId.eq(user_id))
            .filter(notifications::Column::IsRead.eq(false))
            .all(self.db)
            .await?;
        let count = rows.len() as u64;
        for m in rows {
            let mut a: notifications::ActiveModel = m.into();
            a.is_read = Set(true);
            a.update(self.db).await?;
        }
        Ok(count)
    }
    pub async fn delete(&self, id: Uuid, user_id: Uuid) -> DataResult<bool> {
        let Some(m) = notifications::Entity::find_by_id(id)
            .filter(notifications::Column::UserId.eq(user_id))
            .one(self.db)
            .await?
        else {
            return Ok(false);
        };
        Ok(notifications::Entity::delete_by_id(m.id)
            .exec(self.db)
            .await?
            .rows_affected
            > 0)
    }
    pub async fn preferences(
        &self,
        user_id: Uuid,
    ) -> DataResult<Vec<notification_preferences::Model>> {
        Ok(notification_preferences::Entity::find()
            .filter(notification_preferences::Column::UserId.eq(user_id))
            .order_by_asc(notification_preferences::Column::EventType)
            .all(self.db)
            .await?)
    }
    pub async fn ensure_default_preferences(
        &self,
        user_id: Uuid,
        event_types: &[&str],
    ) -> DataResult<()> {
        for event in event_types {
            let _ = self.user_preference(user_id, event).await?;
        }
        Ok(())
    }
    pub async fn update_preference(&self, user_id: Uuid, p: PreferencePatch) -> DataResult<()> {
        let m = self.user_preference(user_id, &p.event_type).await?;
        let mut a: notification_preferences::ActiveModel = m.into();
        if let Some(v) = p.email_enabled {
            a.email_enabled = Set(v)
        }
        if let Some(v) = p.in_app_enabled {
            a.in_app_enabled = Set(v)
        }
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn tenant_settings(
        &self,
        tenant_id: Uuid,
    ) -> DataResult<Vec<tenant_notification_settings::Model>> {
        Ok(tenant_notification_settings::Entity::find()
            .filter(tenant_notification_settings::Column::TenantId.eq(tenant_id))
            .order_by_asc(tenant_notification_settings::Column::Role)
            .order_by_asc(tenant_notification_settings::Column::EventType)
            .all(self.db)
            .await?)
    }
    pub async fn ensure_global_settings(
        &self,
        tenant_id: Uuid,
        event_types: &[&str],
    ) -> DataResult<()> {
        for event in event_types {
            if self.tenant_setting(tenant_id, event, "").await?.is_none() {
                tenant_notification_settings::ActiveModel {
                    tenant_id: Set(tenant_id),
                    event_type: Set((*event).into()),
                    role: Set(None),
                    ..Default::default()
                }
                .insert(self.db)
                .await?;
            }
        }
        Ok(())
    }
    pub async fn update_tenant_setting(
        &self,
        tenant_id: Uuid,
        role: Option<String>,
        p: TenantNotificationPatch,
    ) -> DataResult<()> {
        let m = tenant_notification_settings::Entity::find()
            .filter(tenant_notification_settings::Column::TenantId.eq(tenant_id))
            .filter(tenant_notification_settings::Column::EventType.eq(&p.event_type))
            .filter(match &role {
                Some(r) => tenant_notification_settings::Column::Role.eq(r),
                None => tenant_notification_settings::Column::Role.is_null(),
            })
            .one(self.db)
            .await?;
        let mut a = match m {
            Some(m) => m.into(),
            None => tenant_notification_settings::ActiveModel {
                tenant_id: Set(tenant_id),
                event_type: Set(p.event_type),
                role: Set(role),
                ..Default::default()
            },
        };
        if let Some(v) = p.enabled {
            a.enabled = Set(v)
        }
        if let Some(v) = p.email_enforced {
            a.email_enforced = Set(v)
        }
        if let Some(v) = p.in_app_enforced {
            a.in_app_enforced = Set(v)
        }
        if let Some(v) = p.default_email {
            a.default_email = Set(v)
        }
        if let Some(v) = p.default_in_app {
            a.default_in_app = Set(v)
        }
        a.updated_at = Set(chrono::Utc::now().into());
        a.save(self.db).await?;
        Ok(())
    }
}
