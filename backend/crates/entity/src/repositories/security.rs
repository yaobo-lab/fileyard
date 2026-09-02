use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement};
use uuid::Uuid;

use crate::{entities::{tenants, users}, DataResult};

pub struct NewSecurityAlert<'a> {
    pub tenant_id: Option<Uuid>, pub user_id: Option<Uuid>, pub alert_type: &'a str,
    pub severity: &'a str, pub title: &'a str, pub description: &'a str,
    pub metadata: serde_json::Value, pub ip_address: Option<&'a str>,
}

pub struct SecurityRepository<'a> { db: &'a DatabaseConnection }

impl<'a> SecurityRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self { Self { db } }
    fn stmt(&self, sql: &str, values: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), sql, values)
    }
    async fn count(&self, sql: &str, values: Vec<sea_orm::Value>) -> DataResult<i64> {
        let row = self.db.query_one(self.stmt(sql, values)).await?
            .ok_or(sea_orm::DbErr::RecordNotFound("count".into()))?;
        Ok(row.try_get("", "count")?)
    }
    pub async fn create_alert(&self, value: NewSecurityAlert<'_>) -> DataResult<Uuid> {
        let row = self.db.query_one(self.stmt(
            "INSERT INTO security_alerts (tenant_id,user_id,alert_type,severity,title,description,metadata,ip_address) VALUES ($1,$2,$3,$4,$5,$6,$7,$8::inet) RETURNING id",
            vec![value.tenant_id.into(), value.user_id.into(), value.alert_type.into(), value.severity.into(),
                 value.title.into(), value.description.into(), value.metadata.into(), value.ip_address.into()])).await?
            .ok_or(sea_orm::DbErr::RecordNotFound("security alert".into()))?;
        Ok(row.try_get("", "id")?)
    }
    pub async fn tenant(&self, id: Uuid) -> DataResult<Option<tenants::Model>> { Ok(tenants::Entity::find_by_id(id).one(self.db).await?) }
    pub async fn record_failed_login(&self, email: &str, ip: Option<&str>, reason: &str) -> DataResult<()> {
        self.db.execute(self.stmt("INSERT INTO failed_login_attempts (email,ip_address,reason) VALUES ($1,$2::inet,$3)", vec![email.into(),ip.into(),reason.into()])).await?; Ok(())
    }
    pub async fn failed_login_count(&self, email: &str, since: DateTime<Utc>) -> DataResult<i64> {
        self.count("SELECT COUNT(*) AS count FROM failed_login_attempts WHERE email=$1 AND attempted_at>$2", vec![email.into(), since.into()]).await
    }
    pub async fn recent_alert_count(&self, kind: &str, user_id: Option<Uuid>, email: Option<&str>, since: DateTime<Utc>) -> DataResult<i64> {
        self.count("SELECT COUNT(*) AS count FROM security_alerts WHERE alert_type=$1 AND ($2::uuid IS NULL OR user_id=$2) AND ($3::text IS NULL OR metadata->>'email'=$3) AND created_at>$4",
            vec![kind.into(),user_id.into(),email.into(),since.into()]).await
    }
    pub async fn user_identity_by_email(&self, email: &str) -> DataResult<Option<(Uuid,Uuid)>> {
        Ok(users::Entity::find().filter(users::Column::Email.eq(email)).one(self.db).await?.map(|u|(u.id,u.tenant_id)))
    }
    pub async fn record_login_ip(&self, user_id: Uuid, ip: &str, agent: Option<&str>) -> DataResult<bool> {
        let row=self.db.query_one(self.stmt("INSERT INTO user_login_history (user_id,ip_address,user_agent,login_count) VALUES ($1,$2::inet,$3,1) ON CONFLICT (user_id,ip_address) DO UPDATE SET last_seen_at=NOW(),login_count=user_login_history.login_count+1,user_agent=COALESCE($3,user_login_history.user_agent) RETURNING (xmax=0) AS is_new", vec![user_id.into(),ip.into(),agent.into()])).await?.unwrap();
        Ok(row.try_get("","is_new")?)
    }
    pub async fn login_ip_count(&self,user_id:Uuid)->DataResult<i64>{self.count("SELECT COUNT(*) AS count FROM user_login_history WHERE user_id=$1",vec![user_id.into()]).await}
    pub async fn recent_download_count(&self,tenant_id:Uuid,user_id:Uuid,since:DateTime<Utc>)->DataResult<i64>{self.count("SELECT COUNT(*) AS count FROM audit_logs WHERE tenant_id=$1 AND user_id=$2 AND action IN ('file_download','folder_download') AND created_at>$3",vec![tenant_id.into(),user_id.into(),since.into()]).await}
    pub async fn recent_share_count(&self,tenant_id:Uuid,user_id:Uuid,since:DateTime<Utc>)->DataResult<i64>{self.count("SELECT COUNT(*) AS count FROM shares WHERE tenant_id=$1 AND created_by=$2 AND created_at>$3",vec![tenant_id.into(),user_id.into(),since.into()]).await}
    pub async fn cleanup_failed_logins(&self,before:DateTime<Utc>)->DataResult<u64>{Ok(self.db.execute(self.stmt("DELETE FROM failed_login_attempts WHERE attempted_at<$1",vec![before.into()])).await?.rows_affected())}
    pub async fn user_email(&self,id:Uuid)->DataResult<Option<String>>{Ok(users::Entity::find_by_id(id).one(self.db).await?.map(|u|u.email))}
}
