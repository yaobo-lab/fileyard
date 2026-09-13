use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, Statement,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    entities::{audit_logs, audit_settings, users},
    DataResult,
};

#[derive(Debug, Clone)]
pub struct NewAuditLog {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub metadata: Option<Value>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditSettingsUpdate {
    pub log_logins: Option<bool>,
    pub log_file_operations: Option<bool>,
    pub log_user_changes: Option<bool>,
    pub log_settings_changes: Option<bool>,
    pub log_role_changes: Option<bool>,
    pub retention_days: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ActivityLogFilter {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub action: Option<String>,
    pub user_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AuditLogRecord {
    pub id: Uuid,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub metadata: Option<Value>,
    pub ip_address: Option<String>,
}

pub struct AuditRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> AuditRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn get_settings(&self, tenant_id: Uuid) -> DataResult<Option<audit_settings::Model>> {
        Ok(audit_settings::Entity::find()
            .filter(audit_settings::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?)
    }

    pub async fn upsert_settings(
        &self,
        tenant_id: Uuid,
        patch: AuditSettingsUpdate,
    ) -> DataResult<audit_settings::Model> {
        let existing = self.get_settings(tenant_id).await?;
        let now = chrono::Utc::now();
        if let Some(model) = existing {
            let mut active: audit_settings::ActiveModel = model.into();
            if let Some(v) = patch.log_logins {
                active.log_logins = Set(Some(v));
            }
            if let Some(v) = patch.log_file_operations {
                active.log_file_operations = Set(Some(v));
            }
            if let Some(v) = patch.log_user_changes {
                active.log_user_changes = Set(Some(v));
            }
            if let Some(v) = patch.log_settings_changes {
                active.log_settings_changes = Set(Some(v));
            }
            if let Some(v) = patch.log_role_changes {
                active.log_role_changes = Set(Some(v));
            }
            if let Some(v) = patch.retention_days {
                active.retention_days = Set(Some(v));
            }
            active.updated_at = Set(now.into());
            Ok(active.update(self.db).await?)
        } else {
            let active = audit_settings::ActiveModel {
                id: Set(Uuid::new_v4()),
                tenant_id: Set(tenant_id),
                log_logins: Set(Some(patch.log_logins.unwrap_or(true))),
                log_file_operations: Set(Some(patch.log_file_operations.unwrap_or(true))),
                log_user_changes: Set(Some(patch.log_user_changes.unwrap_or(true))),
                log_settings_changes: Set(Some(patch.log_settings_changes.unwrap_or(true))),
                log_role_changes: Set(Some(patch.log_role_changes.unwrap_or(true))),
                retention_days: Set(Some(patch.retention_days.unwrap_or(90))),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
            };
            Ok(active.insert(self.db).await?)
        }
    }

    pub async fn distinct_actions(&self, tenant_id: Uuid) -> DataResult<Vec<String>> {
        let items = audit_logs::Entity::find()
            .filter(audit_logs::Column::TenantId.eq(tenant_id))
            .select_only()
            .column(audit_logs::Column::Action)
            .distinct()
            .order_by_asc(audit_logs::Column::Action)
            .into_tuple::<String>()
            .all(self.db)
            .await?;
        Ok(items)
    }

    pub async fn distinct_resource_types(&self, tenant_id: Uuid) -> DataResult<Vec<String>> {
        let items = audit_logs::Entity::find()
            .filter(audit_logs::Column::TenantId.eq(tenant_id))
            .select_only()
            .column(audit_logs::Column::ResourceType)
            .distinct()
            .order_by_asc(audit_logs::Column::ResourceType)
            .into_tuple::<String>()
            .all(self.db)
            .await?;
        Ok(items)
    }

    pub async fn user_exists_in_tenant(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        is_superadmin: bool,
    ) -> DataResult<bool> {
        let mut query = users::Entity::find_by_id(user_id);
        if !is_superadmin {
            query = query.filter(users::Column::TenantId.eq(tenant_id));
        }
        let res = query.one(self.db).await?;
        Ok(res.is_some())
    }

    pub async fn log_activity(&self, item: NewAuditLog) -> DataResult<audit_logs::Model> {
        let active = audit_logs::ActiveModel {
            id: Set(item.id),
            tenant_id: Set(item.tenant_id),
            user_id: Set(item.user_id),
            action: Set(item.action),
            resource_type: Set(item.resource_type),
            resource_id: Set(item.resource_id),
            metadata: Set(item.metadata),
            ip_address: Set(item.ip_address),
            created_at: Set(chrono::Utc::now().into()),
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn log(
        &self,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        action: impl Into<String>,
        resource_type: impl Into<String>,
        resource_id: Option<Uuid>,
        metadata: Option<serde_json::Value>,
        ip_address: Option<String>,
    ) -> DataResult<audit_logs::Model> {
        self.log_activity(NewAuditLog {
            id: Uuid::new_v4(),
            tenant_id,
            user_id,
            action: action.into(),
            resource_type: resource_type.into(),
            resource_id,
            metadata,
            ip_address,
        })
        .await
    }

    pub async fn query_activity_logs(
        &self,
        tenant_id: Uuid,
        filter: &ActivityLogFilter,
    ) -> DataResult<(Vec<AuditLogRecord>, i64)> {
        // We can execute parameterized Raw SQL or SeaORM join query
        let mut where_clauses = vec!["a.tenant_id = $1".to_string()];
        let mut values: Vec<sea_orm::Value> = vec![tenant_id.into()];
        let mut param_idx = 2;

        if let Some(start) = filter.start_date {
            where_clauses.push(format!("a.created_at >= ${}", param_idx));
            values.push(start.into());
            param_idx += 1;
        }
        if let Some(end) = filter.end_date {
            where_clauses.push(format!("a.created_at < ${} + INTERVAL '1 day'", param_idx));
            values.push(end.into());
            param_idx += 1;
        }
        if let Some(ref action) = filter.action {
            where_clauses.push(format!("a.action = ${}", param_idx));
            values.push(action.clone().into());
            param_idx += 1;
        }
        if let Some(user_id) = filter.user_id {
            where_clauses.push(format!("a.user_id = ${}", param_idx));
            values.push(user_id.into());
            param_idx += 1;
        }
        if let Some(ref res_type) = filter.resource_type {
            where_clauses.push(format!("a.resource_type = ${}", param_idx));
            values.push(res_type.clone().into());
            let _ = param_idx;
        }

        let where_sql = where_clauses.join(" AND ");

        // Count query
        let count_sql = format!("SELECT COUNT(*) FROM audit_logs a WHERE {}", where_sql);
        let count_stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &count_sql,
            values.clone(),
        );
        let count_row = self.db.query_one(count_stmt).await?;
        let total: i64 = match count_row {
            Some(row) => row.try_get_by_index(0).unwrap_or(0),
            None => 0,
        };

        // Data query
        let mut data_sql = format!(
            r#"
            SELECT a.id, a.action, a.resource_type, a.resource_id, a.created_at,
                   a.user_id, u.name as user_name, a.metadata, a.ip_address::text
            FROM audit_logs a
            LEFT JOIN users u ON a.user_id = u.id
            WHERE {}
            ORDER BY a.created_at DESC
            "#,
            where_sql
        );

        let mut data_values = values;
        if let Some(limit) = filter.limit {
            data_sql.push_str(&format!(" LIMIT ${}", data_values.len() + 1));
            data_values.push(limit.into());
        }
        if let Some(offset) = filter.offset {
            data_sql.push_str(&format!(" OFFSET ${}", data_values.len() + 1));
            data_values.push(offset.into());
        }

        let data_stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &data_sql,
            data_values,
        );

        let rows = self.db.query_all(data_stmt).await?;
        let mut records = Vec::with_capacity(rows.len());

        for row in rows {
            let id: Uuid = row.try_get_by_index(0)?;
            let action: String = row.try_get_by_index(1)?;
            let resource_type: String = row.try_get_by_index(2)?;
            let resource_id: Option<Uuid> = row.try_get_by_index(3).ok();
            let created_at: chrono::DateTime<chrono::FixedOffset> = row.try_get_by_index(4)?;
            let user_id: Option<Uuid> = row.try_get_by_index(5).ok();
            let user_name: Option<String> = row.try_get_by_index(6).ok();
            let metadata: Option<Value> = row.try_get_by_index(7).ok();
            let ip_address: Option<String> = row.try_get_by_index(8).ok();

            records.push(AuditLogRecord {
                id,
                action,
                resource_type,
                resource_id,
                created_at: created_at.with_timezone(&Utc),
                user_id,
                user_name,
                metadata,
                ip_address,
            });
        }

        Ok((records, total))
    }

    pub async fn query_user_activity_logs(
        &self,
        user_id: Uuid,
        filter: &ActivityLogFilter,
    ) -> DataResult<(Vec<AuditLogRecord>, i64)> {
        let mut where_clauses = vec!["a.user_id = $1".to_string()];
        let mut values: Vec<sea_orm::Value> = vec![user_id.into()];
        let mut param_idx = 2;

        if let Some(start) = filter.start_date {
            where_clauses.push(format!("a.created_at >= ${}", param_idx));
            values.push(start.into());
            param_idx += 1;
        }
        if let Some(end) = filter.end_date {
            where_clauses.push(format!("a.created_at < ${} + INTERVAL '1 day'", param_idx));
            values.push(end.into());
            param_idx += 1;
        }
        if let Some(ref action) = filter.action {
            where_clauses.push(format!("a.action = ${}", param_idx));
            values.push(action.clone().into());
            param_idx += 1;
        }
        if let Some(ref res_type) = filter.resource_type {
            where_clauses.push(format!("a.resource_type = ${}", param_idx));
            values.push(res_type.clone().into());
        }

        let where_sql = where_clauses.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM audit_logs a WHERE {}", where_sql);
        let count_stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &count_sql,
            values.clone(),
        );
        let count_row = self.db.query_one(count_stmt).await?;
        let total: i64 = match count_row {
            Some(row) => row.try_get_by_index(0).unwrap_or(0),
            None => 0,
        };

        let mut data_sql = format!(
            r#"
            SELECT a.id, a.action, a.resource_type, a.resource_id, a.created_at,
                   a.user_id, u.name as user_name, a.metadata, a.ip_address::text
            FROM audit_logs a
            LEFT JOIN users u ON a.user_id = u.id
            WHERE {}
            ORDER BY a.created_at DESC
            "#,
            where_sql
        );

        let mut data_values = values;
        if let Some(limit) = filter.limit {
            data_sql.push_str(&format!(" LIMIT ${}", data_values.len() + 1));
            data_values.push(limit.into());
        }
        if let Some(offset) = filter.offset {
            data_sql.push_str(&format!(" OFFSET ${}", data_values.len() + 1));
            data_values.push(offset.into());
        }

        let data_stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &data_sql,
            data_values,
        );

        let rows = self.db.query_all(data_stmt).await?;
        let mut records = Vec::with_capacity(rows.len());

        for row in rows {
            let id: Uuid = row.try_get_by_index(0)?;
            let action: String = row.try_get_by_index(1)?;
            let resource_type: String = row.try_get_by_index(2)?;
            let resource_id: Option<Uuid> = row.try_get_by_index(3).ok();
            let created_at: chrono::DateTime<chrono::FixedOffset> = row.try_get_by_index(4)?;
            let user_id: Option<Uuid> = row.try_get_by_index(5).ok();
            let user_name: Option<String> = row.try_get_by_index(6).ok();
            let metadata: Option<Value> = row.try_get_by_index(7).ok();
            let ip_address: Option<String> = row.try_get_by_index(8).ok();

            records.push(AuditLogRecord {
                id,
                action,
                resource_type,
                resource_id,
                created_at: created_at.with_timezone(&Utc),
                user_id,
                user_name,
                metadata,
                ip_address,
            });
        }

        Ok((records, total))
    }

    pub async fn file_activity(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        limit: u64,
    ) -> DataResult<Vec<(Uuid, String, Option<Uuid>, Option<String>, Option<Value>, DateTime<Utc>)>> {
        let sql = r#"
            SELECT al.id, al.action, al.user_id, u.name as user_name, al.metadata, al.created_at
            FROM audit_logs al
            LEFT JOIN users u ON u.id = al.user_id
            WHERE al.tenant_id = $1 AND al.resource_id = $2 AND al.resource_type = 'file'
            ORDER BY al.created_at DESC
            LIMIT $3
        "#;
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![tenant_id.into(), file_id.into(), (limit as i64).into()],
        );
        let rows = self.db.query_all(stmt).await?;
        let mut result = Vec::new();
        for r in rows {
            let id: Uuid = r.try_get("", "id")?;
            let action: String = r.try_get("", "action")?;
            let user_id: Option<Uuid> = r.try_get("", "user_id")?;
            let user_name: Option<String> = r.try_get("", "user_name")?;
            let metadata: Option<Value> = r.try_get("", "metadata")?;
            let created_at: chrono::DateTime<chrono::FixedOffset> = r.try_get("", "created_at")?;
            result.push((id, action, user_id, user_name, metadata, created_at.with_timezone(&Utc)));
        }
        Ok(result)
    }
}
