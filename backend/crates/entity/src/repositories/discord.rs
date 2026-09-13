use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QuerySelect,
};
use uuid::Uuid;

use crate::{
    entities::{
        discord_notification_logs, discord_oauth_states, tenant_discord_settings,
        user_discord_connections,
    },
    DataResult,
};

#[derive(Debug, Clone, Default)]
pub struct DiscordPreferencePatch {
    pub dm_notifications_enabled: Option<bool>,
    pub notify_file_shared: Option<bool>,
    pub notify_file_uploaded: Option<bool>,
    pub notify_comments: Option<bool>,
    pub notify_file_requests: Option<bool>,
}

pub struct DiscordRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> DiscordRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn is_enabled(&self, tenant_id: Uuid) -> DataResult<bool> {
        let res = tenant_discord_settings::Entity::find_by_id(tenant_id)
            .select_only()
            .column(tenant_discord_settings::Column::Enabled)
            .into_tuple::<bool>()
            .one(self.db)
            .await?;
        Ok(res.unwrap_or(false))
    }

    pub async fn set_enabled(&self, tenant_id: Uuid, enabled: bool) -> DataResult<()> {
        let existing = tenant_discord_settings::Entity::find_by_id(tenant_id)
            .one(self.db)
            .await?;
        let now = Utc::now();
        if let Some(model) = existing {
            let mut active: tenant_discord_settings::ActiveModel = model.into();
            active.enabled = Set(enabled);
            active.updated_at = Set(now.into());
            active.update(self.db).await?;
        } else {
            let active = tenant_discord_settings::ActiveModel {
                tenant_id: Set(tenant_id),
                enabled: Set(enabled),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
            };
            active.insert(self.db).await?;
        }
        Ok(())
    }

    pub async fn create_oauth_state(
        &self,
        state: &str,
        user_id: Uuid,
        tenant_id: Uuid,
        expires_at: chrono::DateTime<Utc>,
    ) -> DataResult<()> {
        let active = discord_oauth_states::ActiveModel {
            state: Set(state.to_string()),
            user_id: Set(user_id),
            tenant_id: Set(tenant_id),
            expires_at: Set(expires_at.into()),
            created_at: Set(Utc::now().into()),
        };
        active.insert(self.db).await?;
        Ok(())
    }

    pub async fn consume_oauth_state(&self, state: &str) -> DataResult<Option<(Uuid, Uuid)>> {
        let item = discord_oauth_states::Entity::find_by_id(state.to_string())
            .filter(discord_oauth_states::Column::ExpiresAt.gt(Utc::now()))
            .one(self.db)
            .await?;
        if let Some(record) = item {
            let _ = discord_oauth_states::Entity::delete_by_id(state.to_string())
                .exec(self.db)
                .await?;
            Ok(Some((record.user_id, record.tenant_id)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_connection(
        &self,
        user_id: Uuid,
    ) -> DataResult<Option<user_discord_connections::Model>> {
        Ok(user_discord_connections::Entity::find()
            .filter(user_discord_connections::Column::UserId.eq(user_id))
            .one(self.db)
            .await?)
    }

    pub async fn upsert_connection(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        discord_user_id: &str,
        discord_username: &str,
        discord_avatar: Option<&str>,
        access_token: &str,
        refresh_token: &str,
        token_expires_at: chrono::DateTime<Utc>,
    ) -> DataResult<()> {
        let existing = self.get_connection(user_id).await?;
        let now = Utc::now();
        if let Some(model) = existing {
            let mut active: user_discord_connections::ActiveModel = model.into();
            active.discord_user_id = Set(discord_user_id.to_string());
            active.discord_username = Set(Some(discord_username.to_string()));
            active.discord_avatar = Set(discord_avatar.map(String::from));
            active.access_token_encrypted = Set(access_token.to_string());
            active.refresh_token_encrypted = Set(Some(refresh_token.to_string()));
            active.token_expires_at = Set(Some(token_expires_at.into()));
            active.updated_at = Set(now.into());
            active.update(self.db).await?;
        } else {
            let active = user_discord_connections::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(user_id),
                tenant_id: Set(tenant_id),
                discord_user_id: Set(discord_user_id.to_string()),
                discord_username: Set(Some(discord_username.to_string())),
                discord_discriminator: Set(None),
                discord_avatar: Set(discord_avatar.map(String::from)),
                access_token_encrypted: Set(access_token.to_string()),
                refresh_token_encrypted: Set(Some(refresh_token.to_string())),
                token_expires_at: Set(Some(token_expires_at.into())),
                dm_notifications_enabled: Set(true),
                notify_file_shared: Set(true),
                notify_file_uploaded: Set(true),
                notify_comments: Set(true),
                notify_file_requests: Set(true),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
            };
            active.insert(self.db).await?;
        }
        Ok(())
    }

    pub async fn disconnect(&self, user_id: Uuid) -> DataResult<bool> {
        let res = user_discord_connections::Entity::delete_many()
            .filter(user_discord_connections::Column::UserId.eq(user_id))
            .exec(self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn update_preferences(
        &self,
        user_id: Uuid,
        patch: DiscordPreferencePatch,
    ) -> DataResult<bool> {
        let Some(model) = self.get_connection(user_id).await? else {
            return Ok(false);
        };
        let mut active: user_discord_connections::ActiveModel = model.into();
        if let Some(v) = patch.dm_notifications_enabled {
            active.dm_notifications_enabled = Set(v);
        }
        if let Some(v) = patch.notify_file_shared {
            active.notify_file_shared = Set(v);
        }
        if let Some(v) = patch.notify_file_uploaded {
            active.notify_file_uploaded = Set(v);
        }
        if let Some(v) = patch.notify_comments {
            active.notify_comments = Set(v);
        }
        if let Some(v) = patch.notify_file_requests {
            active.notify_file_requests = Set(v);
        }
        active.updated_at = Set(Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn log_notification(
        &self,
        user_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        event_type: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> DataResult<()> {
        let active = discord_notification_logs::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            tenant_id: Set(tenant_id),
            event_type: Set(event_type.to_string()),
            status: Set(status.to_string()),
            error_message: Set(error_message.map(String::from)),
            created_at: Set(Utc::now().into()),
        };
        active.insert(self.db).await?;
        Ok(())
    }
}
