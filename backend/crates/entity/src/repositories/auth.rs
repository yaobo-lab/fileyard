use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter,
};
use uuid::Uuid;

use crate::entities::{
    role_permissions, roles, security_alerts, tenant_oidc_providers, tenant_saml_providers,
    tenants, user_sessions, users,
};
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
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn user_status(&self, id: Uuid) -> DataResult<Option<AuthUserStatus>> {
        Ok(users::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .map(|user| AuthUserStatus {
                status: user.status,
                suspended: user.suspended_at.is_some(),
                email: user.email,
            }))
    }

    pub async fn active_user_by_email(&self, email: &str) -> DataResult<Option<users::Model>> {
        Ok(users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .filter(users::Column::Status.eq("active"))
            .one(self.db)
            .await?)
    }
    pub async fn user(&self, id: Uuid) -> DataResult<Option<users::Model>> {
        Ok(users::Entity::find_by_id(id).one(self.db).await?)
    }
    pub async fn tenant(&self, id: Uuid) -> DataResult<Option<tenants::Model>> {
        Ok(tenants::Entity::find_by_id(id).one(self.db).await?)
    }
    pub async fn active_tenant(&self, id: Uuid) -> DataResult<Option<tenants::Model>> {
        Ok(tenants::Entity::find_by_id(id)
            .filter(tenants::Column::Status.eq("active"))
            .one(self.db)
            .await?)
    }
    pub async fn active_tenant_id(&self) -> DataResult<Option<Uuid>> {
        Ok(tenants::Entity::find()
            .filter(tenants::Column::Status.eq("active"))
            .one(self.db)
            .await?
            .map(|t| t.id))
    }
    pub async fn tenant_id_by_domain_or_name(&self, value: &str) -> DataResult<Option<Uuid>> {
        Ok(tenants::Entity::find()
            .filter(
                Condition::any()
                    .add(tenants::Column::Domain.eq(value))
                    .add(tenants::Column::Name.eq(value)),
            )
            .one(self.db)
            .await?
            .map(|t| t.id))
    }
    pub async fn clear_suspension(&self, id: Uuid) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.suspended_at = Set(None);
        a.suspended_until = Set(None);
        a.suspension_reason = Set(None);
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn touch_user(&self, id: Uuid) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.last_active_at = Set(Some(chrono::Utc::now().into()));
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn enabled_sso_providers(
        &self,
        tenant: Uuid,
    ) -> DataResult<Vec<(Uuid, String, String, String)>> {
        let mut out = Vec::new();
        for p in tenant_oidc_providers::Entity::find()
            .filter(tenant_oidc_providers::Column::TenantId.eq(tenant))
            .filter(tenant_oidc_providers::Column::Enabled.eq(true))
            .all(self.db)
            .await?
        {
            out.push((p.id, p.name, p.slug, "oidc".into()))
        }
        for p in tenant_saml_providers::Entity::find()
            .filter(tenant_saml_providers::Column::TenantId.eq(tenant))
            .filter(tenant_saml_providers::Column::Enabled.eq(true))
            .all(self.db)
            .await?
        {
            out.push((p.id, p.name, p.slug, "saml".into()))
        }
        Ok(out)
    }
    pub async fn set_recovery_token(
        &self,
        id: Uuid,
        token: &str,
        expires: chrono::DateTime<chrono::Utc>,
    ) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.recovery_token = Set(Some(token.into()));
        a.recovery_token_expires_at = Set(Some(expires.into()));
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn user_by_valid_recovery_token(
        &self,
        token: &str,
    ) -> DataResult<Option<users::Model>> {
        Ok(users::Entity::find()
            .filter(users::Column::RecoveryToken.eq(token))
            .filter(users::Column::RecoveryTokenExpiresAt.gt(chrono::Utc::now().fixed_offset()))
            .one(self.db)
            .await?)
    }
    pub async fn reset_password(&self, id: Uuid, password_hash: String) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.password_hash = Set(Some(password_hash));
        a.recovery_token = Set(None);
        a.recovery_token_expires_at = Set(None);
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn password_policy(&self, id: Uuid) -> DataResult<Option<serde_json::Value>> {
        Ok(self.tenant(id).await?.and_then(|t| t.password_policy))
    }
    pub async fn set_totp_secret(&self, id: Uuid, secret: String) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.totp_secret = Set(Some(secret));
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn create_local_user(
        &self,
        tenant: Uuid,
        email: String,
        name: String,
        password_hash: String,
        role: String,
    ) -> DataResult<users::Model> {
        let now = chrono::Utc::now().into();
        Ok(users::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant),
            department_id: Set(None),
            custom_role_id: Set(None),
            email: Set(email),
            name: Set(name),
            password_hash: Set(Some(password_hash)),
            role: Set(role),
            status: Set("active".into()),
            avatar_url: Set(None),
            allowed_tenant_ids: Set(None),
            allowed_department_ids: Set(None),
            totp_secret: Set(None),
            recovery_token: Set(None),
            recovery_token_expires_at: Set(None),
            password_changed_at: Set(None),
            suspended_at: Set(None),
            suspended_until: Set(None),
            suspension_reason: Set(None),
            dashboard_layout: Set(None),
            widget_config: Set(None),
            last_active_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            identity_provider: Set("local".into()),
        }
        .insert(self.db)
        .await?)
    }
    pub async fn role_permissions(
        &self,
        tenant: Uuid,
        name: &str,
    ) -> DataResult<Option<(String, Vec<(String, bool)>)>> {
        let role = if let Some(r) = roles::Entity::find()
            .filter(roles::Column::TenantId.eq(tenant))
            .filter(roles::Column::Name.eq(name))
            .one(self.db)
            .await?
        {
            Some(r)
        } else {
            roles::Entity::find()
                .filter(roles::Column::TenantId.is_null())
                .filter(roles::Column::Name.eq(name))
                .one(self.db)
                .await?
        };
        let Some(role) = role else { return Ok(None) };
        let perms = role_permissions::Entity::find()
            .filter(role_permissions::Column::RoleId.eq(role.id))
            .all(self.db)
            .await?
            .into_iter()
            .map(|p| (p.permission, p.granted.unwrap_or(false)))
            .collect();
        Ok(Some((role.base_role, perms)))
    }

    pub async fn session_is_revoked(
        &self,
        token_hash: &str,
        user_id: Uuid,
    ) -> DataResult<Option<bool>> {
        Ok(user_sessions::Entity::find()
            .filter(user_sessions::Column::TokenHash.eq(token_hash))
            .filter(user_sessions::Column::UserId.eq(user_id))
            .filter(user_sessions::Column::ExpiresAt.gt(chrono::Utc::now().fixed_offset()))
            .one(self.db)
            .await?
            .map(|session| session.is_revoked.unwrap_or(false)))
    }

    pub async fn tenant_ip_restrictions(
        &self,
        id: Uuid,
    ) -> DataResult<Option<TenantIpRestrictions>> {
        Ok(tenants::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .map(|tenant| TenantIpRestrictions {
                mode: tenant
                    .ip_restriction_mode
                    .unwrap_or_else(|| "disabled".into()),
                allowlist: tenant.ip_allowlist.unwrap_or_default(),
                blocklist: tenant.ip_blocklist.unwrap_or_default(),
            }))
    }

    pub async fn record_suspended_access_attempt(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        email: &str,
        action: &str,
        ip: Option<&str>,
    ) -> DataResult<()> {
        let recent = security_alerts::Entity::find()
            .filter(security_alerts::Column::AlertType.eq("suspended_access_attempt"))
            .filter(security_alerts::Column::UserId.eq(user_id))
            .filter(
                security_alerts::Column::CreatedAt
                    .gt(chrono::Utc::now().fixed_offset() - chrono::Duration::hours(1)),
            )
            .count(self.db)
            .await?;
        if recent > 0 {
            return Ok(());
        }
        security_alerts::ActiveModel {
            tenant_id: Set(Some(tenant_id)),
            user_id: Set(Some(user_id)),
            alert_type: Set("suspended_access_attempt".into()),
            severity: Set("high".into()),
            title: Set(format!("Suspended user {email} attempted access")),
            description: Set(Some(format!("Suspended user attempted to {action}"))),
            metadata: Set(Some(
                serde_json::json!({"email": email, "attempted_action": action}),
            )),
            ip_address: Set(ip.map(str::to_owned)),
            ..Default::default()
        }
        .insert(self.db)
        .await?;
        Ok(())
    }
}
