use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, Statement,
};
use uuid::Uuid;

use crate::{entities::api_usage, DataResult};

#[derive(Debug, Clone)]
pub struct ApiMetricItem {
    pub tenant_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub response_time_ms: u32,
    pub request_size_bytes: i64,
    pub response_size_bytes: i64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UsageStatsRaw {
    pub total_requests: i64,
    pub total_errors: i64,
    pub avg_response_time: f64,
    pub total_request_bytes: i64,
    pub total_response_bytes: i64,
    pub unique_users: i64,
    pub unique_tenants: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TenantUsageRow {
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub category: String,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_response_time_ms: f64,
    pub total_bytes: i64,
}

pub struct ApiUsageRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ApiUsageRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn insert_metric(&self, metric: &ApiMetricItem) -> DataResult<()> {
        let active = api_usage::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(metric.tenant_id),
            user_id: Set(metric.user_id),
            endpoint: Set(metric.endpoint.clone()),
            method: Set(metric.method.clone()),
            status_code: Set(metric.status_code as i32),
            response_time_ms: Set(metric.response_time_ms as i32),
            request_size_bytes: Set(Some(metric.request_size_bytes)),
            response_size_bytes: Set(Some(metric.response_size_bytes)),
            ip_address: Set(metric.ip_address.clone()),
            user_agent: Set(metric.user_agent.clone()),
            error_message: Set(metric.error_message.clone()),
            created_at: Set(Some(Utc::now().into())),
        };
        active.insert(self.db).await?;
        Ok(())
    }

    pub async fn flush_metrics(&self, metrics: &[ApiMetricItem]) -> DataResult<()> {
        if metrics.is_empty() {
            return Ok(());
        }

        let models: Vec<api_usage::ActiveModel> = metrics
            .iter()
            .map(|m| api_usage::ActiveModel {
                id: Set(Uuid::new_v4()),
                tenant_id: Set(m.tenant_id),
                user_id: Set(m.user_id),
                endpoint: Set(m.endpoint.clone()),
                method: Set(m.method.clone()),
                status_code: Set(m.status_code as i32),
                response_time_ms: Set(m.response_time_ms as i32),
                request_size_bytes: Set(Some(m.request_size_bytes)),
                response_size_bytes: Set(Some(m.response_size_bytes)),
                ip_address: Set(m.ip_address.clone()),
                user_agent: Set(m.user_agent.clone()),
                error_message: Set(m.error_message.clone()),
                created_at: Set(Some(Utc::now().into())),
            })
            .collect();

        use sea_orm::EntityTrait;
        api_usage::Entity::insert_many(models).exec(self.db).await?;
        Ok(())
    }

    pub async fn get_stats(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        tenant_id: Option<Uuid>,
    ) -> DataResult<UsageStatsRaw> {
        let sql = if let Some(tid) = tenant_id {
            let stmt = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                r#"
                SELECT 
                    COUNT(*) as total_requests,
                    COUNT(*) FILTER (WHERE status_code >= 400) as total_errors,
                    AVG(response_time_ms)::FLOAT8 as avg_response_time,
                    COALESCE(SUM(request_size_bytes), 0)::BIGINT as total_request_bytes,
                    COALESCE(SUM(response_size_bytes), 0)::BIGINT as total_response_bytes,
                    COUNT(DISTINCT user_id) as unique_users,
                    COUNT(DISTINCT tenant_id) as unique_tenants
                FROM api_usage
                WHERE created_at >= $1 AND created_at <= $2 AND tenant_id = $3
                "#,
                vec![from.into(), to.into(), tid.into()],
            );
            self.db.query_one(stmt).await?
        } else {
            let stmt = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                r#"
                SELECT 
                    COUNT(*) as total_requests,
                    COUNT(*) FILTER (WHERE status_code >= 400) as total_errors,
                    AVG(response_time_ms)::FLOAT8 as avg_response_time,
                    COALESCE(SUM(request_size_bytes), 0)::BIGINT as total_request_bytes,
                    COALESCE(SUM(response_size_bytes), 0)::BIGINT as total_response_bytes,
                    COUNT(DISTINCT user_id) as unique_users,
                    COUNT(DISTINCT tenant_id) as unique_tenants
                FROM api_usage
                WHERE created_at >= $1 AND created_at <= $2
                "#,
                vec![from.into(), to.into()],
            );
            self.db.query_one(stmt).await?
        };

        if let Some(row) = sql {
            Ok(UsageStatsRaw {
                total_requests: row.try_get_by_index(0).unwrap_or(0),
                total_errors: row.try_get_by_index(1).unwrap_or(0),
                avg_response_time: row.try_get_by_index(2).unwrap_or(0.0),
                total_request_bytes: row.try_get_by_index(3).unwrap_or(0),
                total_response_bytes: row.try_get_by_index(4).unwrap_or(0),
                unique_users: row.try_get_by_index(5).unwrap_or(0),
                unique_tenants: row.try_get_by_index(6).unwrap_or(0),
            })
        } else {
            Ok(UsageStatsRaw {
                total_requests: 0,
                total_errors: 0,
                avg_response_time: 0.0,
                total_request_bytes: 0,
                total_response_bytes: 0,
                unique_users: 0,
                unique_tenants: 0,
            })
        }
    }

    pub async fn get_usage_by_tenant(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> DataResult<Vec<TenantUsageRow>> {
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            WITH categorized AS (
                SELECT 
                    u.tenant_id,
                    t.name as tenant_name,
                    CASE 
                        WHEN u.tenant_id IS NOT NULL THEN 'tenant'
                        WHEN u.endpoint LIKE '/api/auth/login%' 
                          OR u.endpoint LIKE '/api/auth/register%'
                          OR u.endpoint LIKE '/api/auth/forgot-password%'
                          OR u.endpoint LIKE '/api/auth/reset-password%'
                          OR u.endpoint LIKE '/health%'
                          OR u.endpoint LIKE '/api/public%'
                          OR u.endpoint = '/'
                        THEN 'unauthenticated'
                        ELSE 'unknown'
                    END as category,
                    u.status_code,
                    u.response_time_ms,
                    u.request_size_bytes,
                    u.response_size_bytes
                FROM api_usage u
                LEFT JOIN tenants t ON t.id = u.tenant_id
                WHERE u.created_at >= $1 AND u.created_at <= $2
            )
            SELECT 
                tenant_id,
                CASE 
                    WHEN category = 'tenant' THEN tenant_name
                    WHEN category = 'unauthenticated' THEN 'Unauthenticated'
                    ELSE 'Unknown'
                END as tenant_name,
                category,
                COUNT(*) as request_count,
                COUNT(*) FILTER (WHERE status_code >= 400) as error_count,
                AVG(response_time_ms)::FLOAT8 as avg_response_time_ms,
                COALESCE(SUM(request_size_bytes + response_size_bytes), 0)::BIGINT as total_bytes
            FROM categorized
            GROUP BY tenant_id, tenant_name, category
            ORDER BY request_count DESC
            LIMIT 50
            "#,
            vec![from.into(), to.into()],
        );

        let rows = self.db.query_all(stmt).await?;
        let mut results = Vec::with_capacity(rows.len());

        for row in rows {
            let tenant_id: Option<Uuid> = row.try_get_by_index(0).ok();
            let tenant_name: Option<String> = row.try_get_by_index(1).ok();
            let category: String = row.try_get_by_index(2)?;
            let request_count: i64 = row.try_get_by_index(3)?;
            let error_count: i64 = row.try_get_by_index(4)?;
            let avg_response_time_ms: f64 = row.try_get_by_index(5)?;
            let total_bytes: i64 = row.try_get_by_index(6)?;

            results.push(TenantUsageRow {
                tenant_id,
                tenant_name,
                category,
                request_count,
                error_count,
                avg_response_time_ms,
                total_bytes,
            });
        }

        Ok(results)
    }

    pub async fn query_raw_sql(
        &self,
        sql: &str,
        values: Vec<sea_orm::Value>,
    ) -> DataResult<Vec<sea_orm::QueryResult>> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        Ok(self.db.query_all(stmt).await?)
    }

    pub async fn query_models<T: sea_orm::FromQueryResult>(
        &self,
        sql: &str,
        values: Vec<sea_orm::Value>,
    ) -> DataResult<Vec<T>> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        Ok(T::find_by_statement(stmt).all(self.db).await?)
    }

    pub async fn query_one_model<T: sea_orm::FromQueryResult>(
        &self,
        sql: &str,
        values: Vec<sea_orm::Value>,
    ) -> DataResult<Option<T>> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        Ok(T::find_by_statement(stmt).one(self.db).await?)
    }

    pub async fn execute_raw_sql(
        &self,
        sql: &str,
        values: Vec<sea_orm::Value>,
    ) -> DataResult<u64> {
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        let res = self.db.execute(stmt).await?;
        Ok(res.rows_affected())
    }
}
