use crate::DataResult;
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use uuid::Uuid;

pub type StorageDistribution = (Uuid, String, i64, Option<i64>);
pub type ActiveFileRequest = (Uuid, String, i64, DateTime<Utc>);

pub struct DashboardRepository<'a> {
    db: &'a DatabaseConnection,
}
impl<'a> DashboardRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }
    fn stmt(&self, sql: &str, values: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), sql, values)
    }
    pub async fn storage_distribution(
        &self,
        tenant: Uuid,
        all: bool,
    ) -> DataResult<Vec<StorageDistribution>> {
        let (sql, values) = if all {
            ("SELECT t.id,t.name,COALESCE((SELECT SUM(size_bytes) FROM files_metadata WHERE tenant_id=t.id AND is_deleted=false AND is_directory=false),0)::bigint storage_used,t.storage_quota_bytes FROM tenants t WHERE t.status='active' ORDER BY storage_used DESC LIMIT 10", vec![])
        } else {
            ("SELECT t.id,t.name,COALESCE((SELECT SUM(size_bytes) FROM files_metadata WHERE tenant_id=t.id AND is_deleted=false AND is_directory=false),0)::bigint storage_used,t.storage_quota_bytes FROM tenants t WHERE t.id=$1", vec![tenant.into()])
        };
        let rows = self.db.query_all(self.stmt(sql, values)).await?;
        rows.into_iter()
            .map(|r| {
                Ok((
                    r.try_get("", "id")?,
                    r.try_get("", "name")?,
                    r.try_get("", "storage_used")?,
                    r.try_get("", "storage_quota_bytes")?,
                ))
            })
            .collect()
    }
    pub async fn active_requests(&self, t: Uuid) -> DataResult<Vec<ActiveFileRequest>> {
        let rows=self.db.query_all(self.stmt("SELECT fr.id,fr.name,COALESCE((SELECT COUNT(*) FROM file_request_uploads WHERE file_request_id=fr.id),0)::bigint upload_count,fr.expires_at FROM file_requests fr WHERE fr.tenant_id=$1 AND fr.status='active' AND fr.expires_at>NOW() ORDER BY fr.expires_at LIMIT 5",vec![t.into()])).await?;
        rows.into_iter()
            .map(|r| {
                Ok((
                    r.try_get("", "id")?,
                    r.try_get("", "name")?,
                    r.try_get("", "upload_count")?,
                    r.try_get("", "expires_at")?,
                ))
            })
            .collect()
    }
    async fn count(&self, sql: &str, values: Vec<sea_orm::Value>) -> DataResult<i64> {
        Ok(self
            .db
            .query_one(self.stmt(sql, values))
            .await?
            .map(|r| r.try_get("", "count"))
            .transpose()?
            .unwrap_or(0))
    }
    pub async fn active_request_count(&self, t: Uuid) -> DataResult<i64> {
        self.count("SELECT COUNT(*)::bigint count FROM file_requests WHERE tenant_id=$1 AND status='active' AND expires_at>NOW()",vec![t.into()]).await
    }
    pub async fn file_count(&self, t: Uuid) -> DataResult<i64> {
        self.count("SELECT COUNT(*)::bigint count FROM files_metadata WHERE tenant_id=$1 AND is_deleted=false",vec![t.into()]).await
    }
    pub async fn user_count(&self, t: Uuid) -> DataResult<i64> {
        self.count(
            "SELECT COUNT(*)::bigint count FROM users WHERE tenant_id=$1 AND status='active'",
            vec![t.into()],
        )
        .await
    }
    pub async fn company_count(&self) -> DataResult<i64> {
        self.count(
            "SELECT COUNT(*)::bigint count FROM tenants WHERE status='active'",
            vec![],
        )
        .await
    }
    pub async fn tenant_storage(&self, t: Uuid) -> DataResult<i64> {
        self.count("SELECT COALESCE(SUM(size_bytes),0)::bigint count FROM files_metadata WHERE tenant_id=$1 AND is_deleted=false AND is_directory=false",vec![t.into()]).await
    }
    pub async fn tenant_quota(&self, t: Uuid) -> DataResult<Option<i64>> {
        Ok(self
            .db
            .query_one(self.stmt(
                "SELECT storage_quota_bytes FROM tenants WHERE id=$1",
                vec![t.into()],
            ))
            .await?
            .map(|r| r.try_get("", "storage_quota_bytes"))
            .transpose()?
            .flatten())
    }
    pub async fn file_types(&self, t: Uuid) -> DataResult<Vec<(String, i64)>> {
        let rows=self.db.query_all(self.stmt("SELECT COALESCE(content_type,'application/octet-stream') content_type,COUNT(*)::bigint count FROM files_metadata WHERE tenant_id=$1 AND is_deleted=false AND is_directory=false GROUP BY content_type ORDER BY count DESC LIMIT 10",vec![t.into()])).await?;
        rows.into_iter()
            .map(|r| Ok((r.try_get("", "content_type")?, r.try_get("", "count")?)))
            .collect()
    }
    pub async fn non_directory_file_count(&self, t: Uuid) -> DataResult<i64> {
        self.count("SELECT COUNT(*)::bigint count FROM files_metadata WHERE tenant_id=$1 AND is_deleted=false AND is_directory=false",vec![t.into()]).await
    }
}
