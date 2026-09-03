use crate::{
    entities::{
        audit_logs, password_reset_tokens, tenants, user_preferences, user_sessions, users,
    },
    DataResult,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, Statement, TransactionTrait,
};
use serde_json::Value;
use uuid::Uuid;

pub struct UserListFilter {
    pub tenant_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub manager_departments: Option<Vec<Uuid>>,
    pub employee_only: bool,
    pub role: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}
pub struct UserUpdatePatch {
    pub name: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub department_id: Option<Uuid>,
    pub dashboard_layout: Option<Value>,
    pub widget_config: Option<Value>,
    pub allowed_tenant_ids: Option<Vec<Uuid>>,
    pub allowed_department_ids: Option<Vec<Uuid>>,
}
#[derive(Debug, Clone)]
pub struct ActivityRow {
    pub action: String,
    pub resource_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
pub struct UserRepository<'a> {
    db: &'a DatabaseConnection,
}
impl<'a> UserRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }
    fn stmt(&self, sql: &str, v: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), sql, v)
    }
    pub async fn user(&self, id: Uuid) -> DataResult<Option<users::Model>> {
        Ok(users::Entity::find_by_id(id).one(self.db).await?)
    }
    pub async fn tenant(&self, id: Uuid) -> DataResult<Option<tenants::Model>> {
        Ok(tenants::Entity::find_by_id(id).one(self.db).await?)
    }
    pub async fn departments(
        &self,
        id: Uuid,
    ) -> DataResult<Option<(Option<Uuid>, Option<Vec<Uuid>>)>> {
        Ok(self
            .user(id)
            .await?
            .map(|u| (u.department_id, u.allowed_department_ids)))
    }
    pub async fn list(&self, f: UserListFilter) -> DataResult<Vec<users::Model>> {
        let mut sql = "SELECT * FROM users WHERE 1=1".to_string();
        let mut v = Vec::new();
        let mut n = 1;
        if let Some(x) = f.tenant_id {
            sql += &format!(" AND (tenant_id=${n} OR ${n}=ANY(allowed_tenant_ids))");
            v.push(x.into());
            n += 1
        }
        if let Some(x) = f.department_id {
            sql += &format!(" AND department_id=${n}");
            v.push(x.into());
            n += 1
        } else if let Some(x) = f.manager_departments {
            sql += &format!(" AND department_id=ANY(${n})");
            v.push(x.into());
            n += 1
        }
        if f.employee_only {
            sql += " AND role='Employee'"
        }
        if let Some(x) = f.role {
            sql += &format!(" AND role=${n}");
            v.push(x.into());
            n += 1
        }
        if let Some(x) = f.status {
            sql += &format!(" AND status=${n}");
            v.push(x.into());
            n += 1
        }
        if let Some(x) = f.search {
            sql += &format!(" AND (name ILIKE ${n} OR email ILIKE ${n})");
            v.push(format!("%{x}%").into());
            n += 1
        }
        sql += &format!(" ORDER BY created_at DESC LIMIT ${n} OFFSET ${}", n + 1);
        v.push(f.limit.into());
        v.push(f.offset.into());
        Ok(users::Entity::find()
            .from_raw_sql(self.stmt(&sql, v))
            .all(self.db)
            .await?)
    }
    pub async fn create(
        &self,
        tenant: Uuid,
        email: String,
        name: String,
        password_hash: Option<String>,
        role: String,
        department: Option<Uuid>,
        provider: String,
    ) -> DataResult<users::Model> {
        let now = chrono::Utc::now().into();
        Ok(users::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant),
            department_id: Set(department),
            custom_role_id: Set(None),
            email: Set(email),
            name: Set(name),
            password_hash: Set(password_hash),
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
            identity_provider: Set(provider),
        }
        .insert(self.db)
        .await?)
    }
    pub async fn update(
        &self,
        id: Uuid,
        tenant: Uuid,
        p: UserUpdatePatch,
    ) -> DataResult<Option<users::Model>> {
        let Some(row) = users::Entity::find_by_id(id)
            .filter(users::Column::TenantId.eq(tenant))
            .one(self.db)
            .await?
        else {
            return Ok(None);
        };
        let mut a: users::ActiveModel = row.into();
        if let Some(v) = p.name {
            a.name = Set(v)
        }
        if let Some(v) = p.role {
            a.role = Set(v)
        }
        if let Some(v) = p.status {
            a.status = Set(v)
        }
        if let Some(v) = p.department_id {
            a.department_id = Set(Some(v))
        }
        if let Some(v) = p.dashboard_layout {
            a.dashboard_layout = Set(Some(v))
        }
        if let Some(v) = p.widget_config {
            a.widget_config = Set(Some(v))
        }
        if let Some(v) = p.allowed_tenant_ids {
            a.allowed_tenant_ids = Set(Some(v))
        }
        if let Some(v) = p.allowed_department_ids {
            a.allowed_department_ids = Set(Some(v))
        }
        a.updated_at = Set(chrono::Utc::now().into());
        Ok(Some(a.update(self.db).await?))
    }
    pub async fn deactivate(&self, id: Uuid) -> DataResult<bool> {
        let Some(row) = self.user(id).await? else {
            return Ok(false);
        };
        let mut a: users::ActiveModel = row.into();
        a.status = Set("inactive".into());
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(true)
    }
    pub async fn permanently_delete(&self, id: Uuid, email: &str, name: &str) -> DataResult<bool> {
        let tx = self.db.begin().await?;
        let meta = serde_json::json!({"email":email,"name":name});
        for (sql,vals) in [("DELETE FROM user_sessions WHERE user_id=$1",vec![id.into()]),("DELETE FROM user_preferences WHERE user_id=$1",vec![id.into()]),("UPDATE audit_logs SET user_id=NULL,metadata=jsonb_set(COALESCE(metadata,'{}'::jsonb),'{deleted_user}',$1::jsonb) WHERE user_id=$2",vec![meta.into(),id.into()]),("UPDATE file_requests SET created_by=NULL WHERE created_by=$1",vec![id.into()]),("UPDATE files SET owner_id=NULL WHERE owner_id=$1",vec![id.into()])]{tx.execute(Statement::from_sql_and_values(self.db.get_database_backend(),sql,vals)).await?;}
        let r = users::Entity::delete_by_id(id).exec(&tx).await?;
        tx.commit().await?;
        Ok(r.rows_affected > 0)
    }
    pub async fn activities(&self, id: Uuid) -> DataResult<Vec<ActivityRow>> {
        let rows = audit_logs::Entity::find()
            .filter(audit_logs::Column::UserId.eq(id))
            .all(self.db)
            .await?;
        Ok(rows
            .into_iter()
            .rev()
            .take(50)
            .map(|r| ActivityRow {
                action: r.action,
                resource_type: r.resource_type,
                created_at: r.created_at.with_timezone(&chrono::Utc),
            })
            .collect())
    }
    pub async fn update_profile(
        &self,
        id: Uuid,
        name: Option<String>,
        email: Option<String>,
    ) -> DataResult<Option<users::Model>> {
        let Some(row) = self.user(id).await? else {
            return Ok(None);
        };
        let mut a: users::ActiveModel = row.into();
        if let Some(v) = name {
            a.name = Set(v)
        }
        if let Some(v) = email {
            a.email = Set(v)
        }
        a.updated_at = Set(chrono::Utc::now().into());
        Ok(Some(a.update(self.db).await?))
    }
    pub async fn set_password(&self, id: Uuid, hash: String) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.password_hash = Set(Some(hash));
        a.password_changed_at = Set(Some(chrono::Utc::now().into()));
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn set_avatar(&self, id: Uuid, url: String) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.avatar_url = Set(Some(url));
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn sessions(&self, user: Uuid) -> DataResult<Vec<user_sessions::Model>> {
        Ok(user_sessions::Entity::find()
            .filter(user_sessions::Column::UserId.eq(user))
            .filter(user_sessions::Column::IsRevoked.eq(false))
            .filter(user_sessions::Column::ExpiresAt.gt(chrono::Utc::now().fixed_offset()))
            .order_by_desc(user_sessions::Column::LastActiveAt)
            .all(self.db)
            .await?)
    }
    pub async fn revoke_session(&self, user: Uuid, id: Uuid) -> DataResult<bool> {
        let Some(row) = user_sessions::Entity::find_by_id(id)
            .filter(user_sessions::Column::UserId.eq(user))
            .one(self.db)
            .await?
        else {
            return Ok(false);
        };
        let mut a: user_sessions::ActiveModel = row.into();
        a.is_revoked = Set(Some(true));
        a.update(self.db).await?;
        Ok(true)
    }
    pub async fn revoke_all_sessions(&self, user: Uuid) -> DataResult<()> {
        self.db
            .execute(self.stmt(
                "UPDATE user_sessions SET is_revoked=true WHERE user_id=$1",
                vec![user.into()],
            ))
            .await?;
        Ok(())
    }
    pub async fn preferences(&self, user: Uuid) -> DataResult<Option<Value>> {
        Ok(user_preferences::Entity::find()
            .filter(user_preferences::Column::UserId.eq(user))
            .one(self.db)
            .await?
            .and_then(|p| p.settings))
    }
    pub async fn update_preferences(&self, user: Uuid, settings: Value) -> DataResult<Value> {
        let now = chrono::Utc::now().into();
        if let Some(row) = user_preferences::Entity::find()
            .filter(user_preferences::Column::UserId.eq(user))
            .one(self.db)
            .await?
        {
            let mut current = row
                .settings
                .clone()
                .unwrap_or_else(|| serde_json::json!({}));
            if let (Some(dst), Some(src)) = (current.as_object_mut(), settings.as_object()) {
                dst.extend(src.clone())
            } else {
                current = settings
            }
            let mut a: user_preferences::ActiveModel = row.into();
            a.settings = Set(Some(current.clone()));
            a.updated_at = Set(now);
            a.update(self.db).await?;
            Ok(current)
        } else {
            user_preferences::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(user),
                starred_files: Set(None),
                settings: Set(Some(settings.clone())),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(self.db)
            .await?;
            Ok(settings)
        }
    }
    pub async fn suspend(
        &self,
        id: Uuid,
        until: Option<chrono::DateTime<chrono::Utc>>,
        reason: Option<String>,
    ) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.suspended_at = Set(Some(chrono::Utc::now().into()));
        a.suspended_until = Set(until.map(Into::into));
        a.suspension_reason = Set(reason);
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        self.revoke_all_sessions(id).await?;
        Ok(())
    }
    pub async fn unsuspend(&self, id: Uuid) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.suspended_at = Set(None);
        a.suspended_until = Set(None);
        a.suspension_reason = Set(None);
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn email_in_use_except(&self, email: &str, id: Uuid) -> DataResult<bool> {
        Ok(users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .filter(users::Column::Id.ne(id))
            .one(self.db)
            .await?
            .is_some())
    }
    pub async fn set_email(&self, id: Uuid, email: String) -> DataResult<()> {
        let Some(row) = self.user(id).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = row.into();
        a.email = Set(email);
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn create_password_reset_token(
        &self,
        user: Uuid,
        token_hash: String,
        expires: chrono::DateTime<chrono::Utc>,
        created_by: Uuid,
    ) -> DataResult<()> {
        let now = chrono::Utc::now().into();
        self.db.execute(self.stmt("UPDATE password_reset_tokens SET used_at=NOW() WHERE user_id=$1 AND used_at IS NULL",vec![user.into()])).await?;
        password_reset_tokens::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user),
            token_hash: Set(token_hash),
            expires_at: Set(expires.into()),
            used_at: Set(None),
            created_by: Set(Some(created_by)),
            created_at: Set(now),
        }
        .insert(self.db)
        .await?;
        Ok(())
    }
}
