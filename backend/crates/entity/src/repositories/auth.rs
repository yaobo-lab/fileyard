use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use uuid::Uuid;

use crate::entities::{security_alerts, tenants, user_sessions, users};
use crate::DataResult;

#[derive(Debug, Clone)]
pub struct AuthUserStatus {
    pub status: String,
    pub suspended: bool,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct TenantIpRestrictions {
    pub mode: String,
    pub allowlist: Vec<String>,
    pub blocklist: Vec<String>,
}

pub struct AuthRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> AuthRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    pub async fn user_status(&self, id: Uuid) -> DataResult<Option<AuthUserStatus>> {
        Ok(users::Entity::find_by_id(id).one(self.db).await?.map(|user| AuthUserStatus {
            status: user.status,
            suspended: user.suspended_at.is_some(),
            email: user.email,
        }))
    }

    pub async fn session_is_revoked(&self, token_hash: &str, user_id: Uuid) -> DataResult<Option<bool>> {
        Ok(user_sessions::Entity::find()
            .filter(user_sessions::Column::TokenHash.eq(token_hash))
            .filter(user_sessions::Column::UserId.eq(user_id))
            .filter(user_sessions::Column::ExpiresAt.gt(chrono::Utc::now().fixed_offset()))
            .one(self.db).await?
            .map(|session| session.is_revoked.unwrap_or(false)))
    }

    pub async fn tenant_ip_restrictions(&self, id: Uuid) -> DataResult<Option<TenantIpRestrictions>> {
        Ok(tenants::Entity::find_by_id(id).one(self.db).await?.map(|tenant| TenantIpRestrictions {
            mode: tenant.ip_restriction_mode.unwrap_or_else(|| "disabled".into()),
            allowlist: tenant.ip_allowlist.unwrap_or_default(),
            blocklist: tenant.ip_blocklist.unwrap_or_default(),
        }))
    }

    pub async fn record_suspended_access_attempt(&self, tenant_id: Uuid, user_id: Uuid, email: &str, action: &str, ip: Option<&str>) -> DataResult<()> {
        let recent = security_alerts::Entity::find()
            .filter(security_alerts::Column::AlertType.eq("suspended_access_attempt"))
            .filter(security_alerts::Column::UserId.eq(user_id))
            .filter(security_alerts::Column::CreatedAt.gt(chrono::Utc::now().fixed_offset() - chrono::Duration::hours(1)))
            .count(self.db).await?;
        if recent > 0 { return Ok(()); }
        security_alerts::ActiveModel {
            tenant_id: Set(Some(tenant_id)), user_id: Set(Some(user_id)),
            alert_type: Set("suspended_access_attempt".into()), severity: Set("high".into()),
            title: Set(format!("Suspended user {email} attempted access")),
            description: Set(Some(format!("Suspended user attempted to {action}"))),
            metadata: Set(Some(serde_json::json!({"email": email, "attempted_action": action}))),
            ip_address: Set(ip.map(str::to_owned)), ..Default::default()
        }.insert(self.db).await?;
        Ok(())
    }
}
