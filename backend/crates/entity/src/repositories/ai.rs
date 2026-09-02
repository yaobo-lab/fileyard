use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

use crate::entities::{ai_usage_logs, tenant_ai_settings, users};
use crate::DataResult;

#[derive(Default)]
pub struct AiSettingsPatch {
    pub enabled: Option<bool>,
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub allowed_roles: Option<Vec<String>>,
    pub hipaa_approved_only: Option<bool>,
    pub sox_read_only: Option<bool>,
    pub monthly_token_limit: Option<i32>,
    pub daily_request_limit: Option<i32>,
    pub maintenance_mode: Option<bool>,
    pub maintenance_message: Option<String>,
    pub custom_endpoint: Option<String>,
    pub custom_model: Option<String>,
}

pub struct NewAiUsage<'a> {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub file_id: Option<Uuid>,
    pub file_name: Option<&'a str>,
    pub action: &'a str,
    pub provider: &'a str,
    pub model: Option<&'a str>,
    pub tokens_used: i32,
    pub status: &'a str,
    pub error_message: Option<&'a str>,
}

pub struct AiUsagePage {
    pub tokens_today: i32,
    pub total: u64,
    pub rows: Vec<(ai_usage_logs::Model, Option<String>)>,
}

pub struct AiRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> AiRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_or_create_settings(
        &self,
        tenant_id: Uuid,
    ) -> DataResult<tenant_ai_settings::Model> {
        if let Some(model) = tenant_ai_settings::Entity::find_by_id(tenant_id)
            .one(self.db)
            .await?
        {
            return Ok(model);
        }
        Ok(tenant_ai_settings::ActiveModel {
            tenant_id: Set(tenant_id),
            ..Default::default()
        }
        .insert(self.db)
        .await?)
    }

    pub async fn update_settings(
        &self,
        tenant_id: Uuid,
        patch: AiSettingsPatch,
    ) -> DataResult<tenant_ai_settings::Model> {
        let model = self.get_or_create_settings(tenant_id).await?;
        let mut active: tenant_ai_settings::ActiveModel = model.into();
        if let Some(v) = patch.enabled {
            active.enabled = Set(v)
        }
        if let Some(v) = patch.provider {
            active.provider = Set(v)
        }
        if let Some(v) = patch.api_key {
            active.api_key_encrypted = Set(Some(v))
        }
        if let Some(v) = patch.allowed_roles {
            active.allowed_roles = Set(v)
        }
        if let Some(v) = patch.hipaa_approved_only {
            active.hipaa_approved_only = Set(v)
        }
        if let Some(v) = patch.sox_read_only {
            active.sox_read_only = Set(v)
        }
        if let Some(v) = patch.monthly_token_limit {
            active.monthly_token_limit = Set(v)
        }
        if let Some(v) = patch.daily_request_limit {
            active.daily_request_limit = Set(v)
        }
        if let Some(v) = patch.maintenance_mode {
            active.maintenance_mode = Set(v)
        }
        if let Some(v) = patch.maintenance_message {
            active.maintenance_message = Set(Some(v))
        }
        if let Some(v) = patch.custom_endpoint {
            active.custom_endpoint = Set(Some(v))
        }
        if let Some(v) = patch.custom_model {
            active.custom_model = Set(Some(v))
        }
        active.updated_at = Set(chrono::Utc::now().fixed_offset());
        Ok(active.update(self.db).await?)
    }

    pub async fn reset_daily(&self, tenant_id: Uuid, date: chrono::NaiveDate) -> DataResult<()> {
        let model = self.get_or_create_settings(tenant_id).await?;
        let mut active: tenant_ai_settings::ActiveModel = model.into();
        active.requests_today = Set(0);
        active.last_usage_reset = Set(Some(date));
        active.update(self.db).await?;
        Ok(())
    }

    pub async fn increment_usage(&self, tenant_id: Uuid, tokens: i32) -> DataResult<()> {
        let model = self.get_or_create_settings(tenant_id).await?;
        let mut active: tenant_ai_settings::ActiveModel = model.clone().into();
        active.tokens_used_this_month = Set(model.tokens_used_this_month + tokens);
        active.requests_today = Set(model.requests_today + 1);
        active.updated_at = Set(chrono::Utc::now().fixed_offset());
        active.update(self.db).await?;
        Ok(())
    }

    pub async fn log_usage(&self, value: NewAiUsage<'_>) -> DataResult<()> {
        ai_usage_logs::ActiveModel {
            tenant_id: Set(value.tenant_id),
            user_id: Set(Some(value.user_id)),
            file_id: Set(value.file_id),
            file_name: Set(value.file_name.map(str::to_owned)),
            action: Set(value.action.to_owned()),
            provider: Set(value.provider.to_owned()),
            model: Set(value.model.map(str::to_owned)),
            tokens_used: Set(value.tokens_used),
            status: Set(value.status.to_owned()),
            error_message: Set(value.error_message.map(str::to_owned)),
            ..Default::default()
        }
        .insert(self.db)
        .await?;
        Ok(())
    }

    pub async fn usage_page(
        &self,
        tenant_id: Uuid,
        offset: u64,
        limit: u64,
    ) -> DataResult<AiUsagePage> {
        let today = chrono::Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight")
            .and_utc()
            .fixed_offset();
        let today_rows = ai_usage_logs::Entity::find()
            .filter(ai_usage_logs::Column::TenantId.eq(tenant_id))
            .filter(ai_usage_logs::Column::CreatedAt.gte(today))
            .all(self.db)
            .await?;
        let total = ai_usage_logs::Entity::find()
            .filter(ai_usage_logs::Column::TenantId.eq(tenant_id))
            .count(self.db)
            .await?;
        let models = ai_usage_logs::Entity::find()
            .filter(ai_usage_logs::Column::TenantId.eq(tenant_id))
            .order_by_desc(ai_usage_logs::Column::CreatedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db)
            .await?;
        let mut rows = Vec::with_capacity(models.len());
        for model in models {
            let name = match model.user_id {
                Some(id) => users::Entity::find_by_id(id)
                    .one(self.db)
                    .await?
                    .map(|u| u.name),
                None => None,
            };
            rows.push((model, name));
        }
        Ok(AiUsagePage {
            tokens_today: today_rows.iter().map(|r| r.tokens_used).sum(),
            total,
            rows,
        })
    }
}
