use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Statement,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{entities::backup_history, DataResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantBackupStat {
    pub tenant_name: String,
    pub backup_count: i64,
    pub last_backup: Option<DateTime<Utc>>,
    pub auto_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionCountsData {
    pub users: i64,
    pub departments: i64,
    pub roles: i64,
    pub file_metadata: i64,
    pub audit_logs: i64,
    pub approval_policies: i64,
    pub approval_history: i64,
    pub sso_oidc: i64,
    pub sso_saml: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetricsData {
    pub total: i64,
    pub auto_count: i64,
    pub failed_24h: i64,
    pub total_storage: i64,
    pub last_backup_duration_ms: Option<i32>,
    pub last_backup_at: Option<DateTime<Utc>>,
    pub by_tenant: Vec<TenantBackupStat>,
}

pub struct BackupRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> BackupRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_history(
        &self,
        tenant_id: Option<Uuid>,
        limit: u64,
    ) -> DataResult<Vec<backup_history::Model>> {
        let mut query = backup_history::Entity::find();
        if let Some(tid) = tenant_id {
            query = query.filter(backup_history::Column::TenantId.eq(tid));
        } else {
            query = query.filter(backup_history::Column::TenantId.is_null());
        }
        Ok(query
            .order_by_desc(backup_history::Column::CreatedAt)
            .limit(limit)
            .all(self.db)
            .await?)
    }

    pub async fn find_history_by_id(&self, id: Uuid) -> DataResult<Option<backup_history::Model>> {
        Ok(backup_history::Entity::find_by_id(id).one(self.db).await?)
    }

    pub async fn delete_history_by_id(&self, id: Uuid) -> DataResult<bool> {
        let res = backup_history::Entity::delete_by_id(id)
            .exec(self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn record_backup(
        &self,
        tenant_id: Option<Uuid>,
        filename: String,
        storage_path: String,
        size_bytes: i64,
        sections: Value,
        is_auto: bool,
        duration_ms: i32,
        user_id: Option<Uuid>,
    ) -> DataResult<backup_history::Model> {
        let active = backup_history::ActiveModel {
            id: sea_orm::Set(Uuid::new_v4()),
            tenant_id: sea_orm::Set(tenant_id),
            filename: sea_orm::Set(filename),
            storage_path: sea_orm::Set(storage_path),
            size_bytes: sea_orm::Set(size_bytes),
            sections: sea_orm::Set(sections),
            is_auto_backup: sea_orm::Set(is_auto),
            status: sea_orm::Set("completed".to_string()),
            duration_ms: sea_orm::Set(Some(duration_ms)),
            created_by: sea_orm::Set(user_id),
            created_at: sea_orm::Set(Utc::now().fixed_offset()),
            ..Default::default()
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn get_old_backups(
        &self,
        tenant_id: Option<Uuid>,
        retention_count: u64,
    ) -> DataResult<Vec<(Uuid, String)>> {
        let mut query = backup_history::Entity::find()
            .filter(backup_history::Column::IsAutoBackup.eq(true))
            .filter(backup_history::Column::Status.eq("completed"));
        if let Some(tid) = tenant_id {
            query = query.filter(backup_history::Column::TenantId.eq(tid));
        } else {
            query = query.filter(backup_history::Column::TenantId.is_null());
        }
        let list = query
            .order_by_desc(backup_history::Column::CreatedAt)
            .offset(retention_count)
            .all(self.db)
            .await?;
        Ok(list.into_iter().map(|m| (m.id, m.storage_path)).collect())
    }

    pub async fn get_last_auto_backup_time(
        &self,
        tenant_id: Option<Uuid>,
    ) -> DataResult<Option<DateTime<Utc>>> {
        let mut query = backup_history::Entity::find()
            .filter(backup_history::Column::IsAutoBackup.eq(true))
            .filter(backup_history::Column::Status.eq("completed"));
        if let Some(tid) = tenant_id {
            query = query.filter(backup_history::Column::TenantId.eq(tid));
        } else {
            query = query.filter(backup_history::Column::TenantId.is_null());
        }
        let last = query
            .order_by_desc(backup_history::Column::CreatedAt)
            .one(self.db)
            .await?;
        Ok(last.map(|m| m.created_at.with_timezone(&Utc)))
    }

    pub async fn get_metrics(&self) -> DataResult<BackupMetricsData> {
        let total = backup_history::Entity::find().count(self.db).await? as i64;
        let auto_count = backup_history::Entity::find()
            .filter(backup_history::Column::IsAutoBackup.eq(true))
            .count(self.db)
            .await? as i64;

        let day_ago = Utc::now() - chrono::Duration::hours(24);
        let failed_24h = backup_history::Entity::find()
            .filter(backup_history::Column::Status.eq("failed"))
            .filter(backup_history::Column::CreatedAt.gt(day_ago))
            .count(self.db)
            .await? as i64;

        let total_storage = {
            let stmt = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                "SELECT COALESCE(SUM(size_bytes), 0)::bigint FROM backup_history WHERE status = 'completed'",
                vec![],
            );
            if let Some(row) = self.db.query_one(stmt).await? {
                let bytes: i64 = row.try_get_by_index(0)?;
                bytes
            } else {
                0
            }
        };

        let last_backup = backup_history::Entity::find()
            .order_by_desc(backup_history::Column::CreatedAt)
            .one(self.db)
            .await?;

        let by_tenant_rows = self
            .db
            .query_all(Statement::from_sql_and_values(
                self.db.get_database_backend(),
                r#"
                SELECT t.name, COUNT(bh.id) as count, MAX(bh.created_at) as last_backup,
                       COALESCE(t.auto_backup_enabled, false) as auto_enabled
                FROM tenants t
                LEFT JOIN backup_history bh ON bh.tenant_id = t.id
                WHERE t.status = 'active'
                GROUP BY t.id, t.name, t.auto_backup_enabled
                ORDER BY t.name
                "#,
                vec![],
            ))
            .await?;

        let mut by_tenant = Vec::with_capacity(by_tenant_rows.len());
        for row in by_tenant_rows {
            let tenant_name: String = row.try_get("", "name")?;
            let backup_count: i64 = row.try_get("", "count")?;
            let last_backup: Option<chrono::DateTime<chrono::FixedOffset>> =
                row.try_get("", "last_backup")?;
            let auto_enabled: bool = row.try_get("", "auto_enabled")?;
            by_tenant.push(TenantBackupStat {
                tenant_name,
                backup_count,
                last_backup: last_backup.map(|dt| dt.with_timezone(&Utc)),
                auto_enabled,
            });
        }

        Ok(BackupMetricsData {
            total,
            auto_count,
            failed_24h,
            total_storage,
            last_backup_duration_ms: last_backup.as_ref().and_then(|b| b.duration_ms),
            last_backup_at: last_backup.as_ref().map(|b| b.created_at.with_timezone(&Utc)),
            by_tenant,
        })
    }

    pub async fn section_counts(&self, tenant_id: Uuid) -> DataResult<SectionCountsData> {
        let v = vec![tenant_id.into()];
        let users = self
            .query_count(
                "SELECT COUNT(*) FROM users WHERE tenant_id = $1",
                v.clone(),
            )
            .await?;
        let departments = self
            .query_count(
                "SELECT COUNT(*) FROM departments WHERE tenant_id = $1",
                v.clone(),
            )
            .await?;
        let roles = self
            .query_count(
                "SELECT COUNT(*) FROM roles WHERE tenant_id = $1",
                v.clone(),
            )
            .await?;
        let file_metadata = self
            .query_count(
                "SELECT COUNT(*) FROM files_metadata WHERE tenant_id = $1",
                v.clone(),
            )
            .await?;
        let audit_logs = self
            .query_count(
                "SELECT COUNT(*) FROM audit_logs WHERE tenant_id = $1",
                v.clone(),
            )
            .await?;
        let approval_policies = self
            .query_count(
                "SELECT COUNT(*) FROM approval_policies WHERE tenant_id = $1",
                v.clone(),
            )
            .await?;
        let approval_history = self
            .query_count(
                "SELECT COUNT(*) FROM approval_requests WHERE tenant_id = $1",
                v.clone(),
            )
            .await?;
        let sso_oidc = self
            .query_count(
                "SELECT COUNT(*) FROM tenant_oidc_providers WHERE tenant_id = $1",
                v.clone(),
            )
            .await?;
        let sso_saml = self
            .query_count(
                "SELECT COUNT(*) FROM tenant_saml_providers WHERE tenant_id = $1",
                v,
            )
            .await?;

        Ok(SectionCountsData {
            users,
            departments,
            roles,
            file_metadata,
            audit_logs,
            approval_policies,
            approval_history,
            sso_oidc,
            sso_saml,
        })
    }

    pub async fn query_scalar_json(
        &self,
        sql: &str,
        values: Vec<sea_orm::Value>,
    ) -> DataResult<Option<Value>> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        let res = self.db.query_one(stmt).await?;
        if let Some(row) = res {
            let val: Value = row.try_get_by_index(0)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub async fn query_scalar_json_all(
        &self,
        sql: &str,
        values: Vec<sea_orm::Value>,
    ) -> DataResult<Vec<Value>> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        let rows = self.db.query_all(stmt).await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            let val: Value = row.try_get_by_index(0)?;
            list.push(val);
        }
        Ok(list)
    }

    pub async fn query_count(&self, sql: &str, values: Vec<sea_orm::Value>) -> DataResult<i64> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        if let Some(row) = self.db.query_one(stmt).await? {
            let count: i64 = row.try_get_by_index(0)?;
            Ok(count)
        } else {
            Ok(0)
        }
    }

    pub async fn execute(&self, sql: &str, values: Vec<sea_orm::Value>) -> DataResult<u64> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        let res = self.db.execute(stmt).await?;
        Ok(res.rows_affected())
    }
}
