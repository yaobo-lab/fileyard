use chrono::{DateTime, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use uuid::Uuid;

use crate::{
    entities::{tenants, users},
    DataResult,
};

pub struct NewSecurityAlert<'a> {
    pub tenant_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub alert_type: &'a str,
    pub severity: &'a str,
    pub title: &'a str,
    pub description: &'a str,
    pub metadata: serde_json::Value,
    pub ip_address: Option<&'a str>,
}

pub struct SecurityRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> SecurityRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }
    fn stmt(&self, sql: &str, values: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), sql, values)
    }
    async fn count(&self, sql: &str, values: Vec<sea_orm::Value>) -> DataResult<i64> {
        let row = self
            .db
            .query_one(self.stmt(sql, values))
            .await?
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
    pub async fn tenant(&self, id: Uuid) -> DataResult<Option<tenants::Model>> {
        Ok(tenants::Entity::find_by_id(id).one(self.db).await?)
    }
    pub async fn record_failed_login(
        &self,
        email: &str,
        ip: Option<&str>,
        reason: &str,
    ) -> DataResult<()> {
        self.db.execute(self.stmt("INSERT INTO failed_login_attempts (email,ip_address,reason) VALUES ($1,$2::inet,$3)", vec![email.into(),ip.into(),reason.into()])).await?;
        Ok(())
    }
    pub async fn failed_login_count(&self, email: &str, since: DateTime<Utc>) -> DataResult<i64> {
        self.count("SELECT COUNT(*) AS count FROM failed_login_attempts WHERE email=$1 AND attempted_at>$2", vec![email.into(), since.into()]).await
    }
    pub async fn recent_alert_count(
        &self,
        kind: &str,
        user_id: Option<Uuid>,
        email: Option<&str>,
        since: DateTime<Utc>,
    ) -> DataResult<i64> {
        self.count("SELECT COUNT(*) AS count FROM security_alerts WHERE alert_type=$1 AND ($2::uuid IS NULL OR user_id=$2) AND ($3::text IS NULL OR metadata->>'email'=$3) AND created_at>$4",
            vec![kind.into(),user_id.into(),email.into(),since.into()]).await
    }
    pub async fn user_identity_by_email(&self, email: &str) -> DataResult<Option<(Uuid, Uuid)>> {
        Ok(users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(self.db)
            .await?
            .map(|u| (u.id, u.tenant_id)))
    }
    pub async fn record_login_ip(
        &self,
        user_id: Uuid,
        ip: &str,
        agent: Option<&str>,
    ) -> DataResult<bool> {
        let row=self.db.query_one(self.stmt("INSERT INTO user_login_history (user_id,ip_address,user_agent,login_count) VALUES ($1,$2::inet,$3,1) ON CONFLICT (user_id,ip_address) DO UPDATE SET last_seen_at=NOW(),login_count=user_login_history.login_count+1,user_agent=COALESCE($3,user_login_history.user_agent) RETURNING (xmax=0) AS is_new", vec![user_id.into(),ip.into(),agent.into()])).await?.unwrap();
        Ok(row.try_get("", "is_new")?)
    }
    pub async fn login_ip_count(&self, user_id: Uuid) -> DataResult<i64> {
        self.count(
            "SELECT COUNT(*) AS count FROM user_login_history WHERE user_id=$1",
            vec![user_id.into()],
        )
        .await
    }
    pub async fn recent_download_count(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        since: DateTime<Utc>,
    ) -> DataResult<i64> {
        self.count("SELECT COUNT(*) AS count FROM audit_logs WHERE tenant_id=$1 AND user_id=$2 AND action IN ('file_download','folder_download') AND created_at>$3",vec![tenant_id.into(),user_id.into(),since.into()]).await
    }
    pub async fn recent_share_count(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        since: DateTime<Utc>,
    ) -> DataResult<i64> {
        self.count("SELECT COUNT(*) AS count FROM shares WHERE tenant_id=$1 AND created_by=$2 AND created_at>$3",vec![tenant_id.into(),user_id.into(),since.into()]).await
    }
    pub async fn cleanup_failed_logins(&self, before: DateTime<Utc>) -> DataResult<u64> {
        Ok(self
            .db
            .execute(self.stmt(
                "DELETE FROM failed_login_attempts WHERE attempted_at<$1",
                vec![before.into()],
            ))
            .await?
            .rows_affected())
    }
    pub async fn user_email(&self, id: Uuid) -> DataResult<Option<String>> {
        Ok(users::Entity::find_by_id(id)
            .one(self.db)
            .await?
            .map(|u| u.email))
    }

    pub async fn list_enriched_alerts(
        &self,
        filter: AlertQueryFilter,
    ) -> DataResult<(Vec<EnrichedSecurityAlert>, i64)> {
        let count_sql = r#"
            SELECT COUNT(*) AS count FROM security_alerts sa
            WHERE ($1::uuid IS NULL OR sa.tenant_id = $1)
            AND ($2::text IS NULL OR sa.severity = $2)
            AND ($3::text IS NULL OR sa.alert_type = $3)
            AND ($4::boolean IS NULL OR sa.resolved = $4)
        "#;
        let count_values = vec![
            filter.tenant_id.into(),
            filter.severity.clone().into(),
            filter.alert_type.clone().into(),
            filter.resolved.into(),
        ];
        let total = self.count(count_sql, count_values).await?;

        let list_sql = r#"
            SELECT 
                sa.id, sa.tenant_id, t.name as tenant_name,
                sa.user_id, u.email as user_email,
                sa.alert_type, sa.severity, sa.title, sa.description, sa.metadata,
                sa.ip_address::text as ip_address,
                sa.resolved, sa.resolved_by, ru.email as resolved_by_email,
                sa.resolved_at, sa.created_at
            FROM security_alerts sa
            LEFT JOIN tenants t ON sa.tenant_id = t.id
            LEFT JOIN users u ON sa.user_id = u.id
            LEFT JOIN users ru ON sa.resolved_by = ru.id
            WHERE ($1::uuid IS NULL OR sa.tenant_id = $1)
            AND ($2::text IS NULL OR sa.severity = $2)
            AND ($3::text IS NULL OR sa.alert_type = $3)
            AND ($4::boolean IS NULL OR sa.resolved = $4)
            ORDER BY 
                CASE sa.severity 
                    WHEN 'critical' THEN 1 
                    WHEN 'high' THEN 2 
                    WHEN 'medium' THEN 3 
                    ELSE 4 
                END,
                sa.created_at DESC
            LIMIT $5 OFFSET $6
        "#;
        let list_values = vec![
            filter.tenant_id.into(),
            filter.severity.into(),
            filter.alert_type.into(),
            filter.resolved.into(),
            (filter.limit as i64).into(),
            (filter.offset as i64).into(),
        ];

        let rows = self.db.query_all(self.stmt(list_sql, list_values)).await?;
        let mut list = Vec::with_capacity(rows.len());

        for row in rows {
            let id: Uuid = row.try_get_by_index(0)?;
            let tenant_id: Option<Uuid> = row.try_get_by_index(1).ok();
            let tenant_name: Option<String> = row.try_get_by_index(2).ok();
            let user_id: Option<Uuid> = row.try_get_by_index(3).ok();
            let user_email: Option<String> = row.try_get_by_index(4).ok();
            let alert_type: String = row.try_get_by_index(5)?;
            let severity: String = row.try_get_by_index(6)?;
            let title: String = row.try_get_by_index(7)?;
            let description: Option<String> = row.try_get_by_index(8).ok();
            let metadata: Option<serde_json::Value> = row.try_get_by_index(9).ok();
            let ip_address: Option<String> = row.try_get_by_index(10).ok();
            let resolved: Option<bool> = row.try_get_by_index(11).ok();
            let resolved_by: Option<Uuid> = row.try_get_by_index(12).ok();
            let resolved_by_email: Option<String> = row.try_get_by_index(13).ok();
            let resolved_at: Option<chrono::DateTime<chrono::FixedOffset>> =
                row.try_get_by_index(14).ok();
            let created_at: Option<chrono::DateTime<chrono::FixedOffset>> =
                row.try_get_by_index(15).ok();

            list.push(EnrichedSecurityAlert {
                id,
                tenant_id,
                tenant_name,
                user_id,
                user_email,
                alert_type,
                severity,
                title,
                description,
                metadata,
                ip_address,
                resolved,
                resolved_by,
                resolved_by_email,
                resolved_at,
                created_at,
            });
        }

        Ok((list, total))
    }

    pub async fn get_stats(&self, tenant_id: Option<Uuid>) -> DataResult<AlertStatsResult> {
        let stats_sql = r#"
            SELECT 
                COUNT(*) AS total,
                COUNT(*) FILTER (WHERE severity = 'critical') AS critical,
                COUNT(*) FILTER (WHERE severity = 'high') AS high,
                COUNT(*) FILTER (WHERE severity = 'medium') AS medium,
                COUNT(*) FILTER (WHERE severity = 'low') AS low,
                COUNT(*) FILTER (WHERE resolved = false) AS unresolved
            FROM security_alerts
            WHERE ($1::uuid IS NULL OR tenant_id = $1)
        "#;
        let stats_row = self
            .db
            .query_one(self.stmt(stats_sql, vec![tenant_id.into()]))
            .await?
            .ok_or(sea_orm::DbErr::RecordNotFound("stats".into()))?;

        let total: i64 = stats_row.try_get_by_index(0).unwrap_or(0);
        let critical: i64 = stats_row.try_get_by_index(1).unwrap_or(0);
        let high: i64 = stats_row.try_get_by_index(2).unwrap_or(0);
        let medium: i64 = stats_row.try_get_by_index(3).unwrap_or(0);
        let low: i64 = stats_row.try_get_by_index(4).unwrap_or(0);
        let unresolved: i64 = stats_row.try_get_by_index(5).unwrap_or(0);

        let type_sql = r#"
            SELECT alert_type, COUNT(*) AS count
            FROM security_alerts
            WHERE resolved = false AND ($1::uuid IS NULL OR tenant_id = $1)
            GROUP BY alert_type
            ORDER BY count DESC
        "#;
        let type_rows = self
            .db
            .query_all(self.stmt(type_sql, vec![tenant_id.into()]))
            .await?;

        let mut by_type = Vec::with_capacity(type_rows.len());
        for row in type_rows {
            let alert_type: String = row.try_get_by_index(0)?;
            let count: i64 = row.try_get_by_index(1)?;
            by_type.push(TypeCountResult { alert_type, count });
        }

        Ok(AlertStatsResult {
            total,
            critical,
            high,
            medium,
            low,
            unresolved,
            by_type,
        })
    }

    pub async fn get_tenant_id(&self, alert_id: Uuid) -> DataResult<Option<Option<Uuid>>> {
        let row = self
            .db
            .query_one(self.stmt(
                "SELECT tenant_id FROM security_alerts WHERE id = $1",
                vec![alert_id.into()],
            ))
            .await?;
        match row {
            Some(r) => Ok(Some(r.try_get_by_index(0).ok())),
            None => Ok(None),
        }
    }

    pub async fn resolve(&self, alert_id: Uuid, user_id: Uuid) -> DataResult<bool> {
        let res = self
            .db
            .execute(self.stmt(
                "UPDATE security_alerts SET resolved = true, resolved_by = $1, resolved_at = NOW() WHERE id = $2",
                vec![user_id.into(), alert_id.into()],
            ))
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn dismiss(&self, alert_id: Uuid) -> DataResult<bool> {
        let res = self
            .db
            .execute(self.stmt(
                "DELETE FROM security_alerts WHERE id = $1",
                vec![alert_id.into()],
            ))
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn count_invalid_tenant_alerts(
        &self,
        ids: &[Uuid],
        tenant_id: Uuid,
    ) -> DataResult<i64> {
        self.count(
            "SELECT COUNT(*) AS count FROM security_alerts WHERE id = ANY($1) AND (tenant_id IS NULL OR tenant_id != $2)",
            vec![ids.to_vec().into(), tenant_id.into()],
        )
        .await
    }

    pub async fn bulk_resolve(
        &self,
        ids: &[Uuid],
        user_id: Uuid,
        tenant_id: Option<Uuid>,
    ) -> DataResult<u64> {
        let sql = r#"
            UPDATE security_alerts 
            SET resolved = true, resolved_by = $1, resolved_at = NOW()
            WHERE id = ANY($2) AND ($3::uuid IS NULL OR tenant_id = $3)
        "#;
        let res = self
            .db
            .execute(self.stmt(sql, vec![user_id.into(), ids.to_vec().into(), tenant_id.into()]))
            .await?;
        Ok(res.rows_affected())
    }

    pub async fn bulk_dismiss(
        &self,
        ids: &[Uuid],
        tenant_id: Option<Uuid>,
    ) -> DataResult<u64> {
        let sql = r#"
            DELETE FROM security_alerts
            WHERE id = ANY($1) AND ($2::uuid IS NULL OR tenant_id = $2)
        "#;
        let res = self
            .db
            .execute(self.stmt(sql, vec![ids.to_vec().into(), tenant_id.into()]))
            .await?;
        Ok(res.rows_affected())
    }

    pub async fn unresolved_badge_count(&self, tenant_id: Option<Uuid>) -> DataResult<i64> {
        self.count(
            r#"
            SELECT COUNT(*) AS count FROM security_alerts
            WHERE resolved = false AND severity IN ('critical', 'high') AND ($1::uuid IS NULL OR tenant_id = $1)
            "#,
            vec![tenant_id.into()],
        )
        .await
    }

    pub async fn count_recent_alerts_by_user(
        &self,
        user_id: Uuid,
        alert_type: &str,
        since: DateTime<Utc>,
    ) -> DataResult<i64> {
        self.count(
            "SELECT COUNT(*) as count FROM security_alerts WHERE alert_type = $1 AND user_id = $2 AND created_at > $3",
            vec![alert_type.into(), user_id.into(), since.into()],
        )
        .await
    }

    pub async fn count_password_confirm_failures(
        &self,
        user_id: Uuid,
        since: DateTime<Utc>,
    ) -> DataResult<i64> {
        self.count_recent_alerts_by_user(user_id, "password_confirm_failed", since).await
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EnrichedSecurityAlert {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub user_id: Option<Uuid>,
    pub user_email: Option<String>,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub resolved: Option<bool>,
    pub resolved_by: Option<Uuid>,
    pub resolved_by_email: Option<String>,
    pub resolved_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_at: Option<chrono::DateTime<chrono::FixedOffset>>,
}

#[derive(Debug, Clone)]
pub struct AlertQueryFilter {
    pub tenant_id: Option<Uuid>,
    pub severity: Option<String>,
    pub alert_type: Option<String>,
    pub resolved: Option<bool>,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AlertStatsResult {
    pub total: i64,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub unresolved: i64,
    pub by_type: Vec<TypeCountResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TypeCountResult {
    pub alert_type: String,
    pub count: i64,
}

