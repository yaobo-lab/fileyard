use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, Statement};
use uuid::Uuid;

use crate::{entities::replication_jobs, DataResult};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReplicationStats {
    pub pending_jobs: i64,
    pub processing_jobs: i64,
    pub failed_jobs: i64,
    pub completed_last_hour: i64,
    pub oldest_pending_age_seconds: Option<i64>,
}

pub struct ReplicationRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ReplicationRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    fn statement(&self, sql: &str, values: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), sql, values)
    }

    pub async fn enqueue(
        &self,
        storage_path: &str,
        tenant_id: Uuid,
        operation: &str,
        size_bytes: Option<i64>,
    ) -> DataResult<Uuid> {
        let id = Uuid::new_v4();
        let row = self.db.query_one(self.statement(
            r#"INSERT INTO replication_jobs (id, storage_path, tenant_id, operation, source_size_bytes)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (storage_path, operation) WHERE status IN ('pending', 'processing')
               DO UPDATE SET next_retry_at = NOW()
               RETURNING id"#,
            vec![id.into(), storage_path.into(), tenant_id.into(), operation.into(), size_bytes.into()],
        )).await?.ok_or(sea_orm::DbErr::RecordNotFound("replication job".into()))?;
        Ok(row.try_get("", "id")?)
    }

    pub async fn fetch_next(&self) -> DataResult<Option<replication_jobs::Model>> {
        Ok(replication_jobs::Model::find_by_statement(self.statement(
            r#"UPDATE replication_jobs SET status = 'processing', started_at = NOW()
               WHERE id = (SELECT id FROM replication_jobs
                 WHERE status = 'pending' AND next_retry_at <= NOW()
                 ORDER BY created_at ASC LIMIT 1 FOR UPDATE SKIP LOCKED)
               RETURNING *"#,
            vec![],
        ))
        .one(self.db)
        .await?)
    }

    pub async fn complete(&self, id: Uuid) -> DataResult<()> {
        self.db.execute(self.statement(
            "UPDATE replication_jobs SET status = 'completed', completed_at = NOW(), error_message = NULL WHERE id = $1",
            vec![id.into()],
        )).await?;
        Ok(())
    }

    pub async fn fail(&self, id: Uuid, error: &str, retry_seconds: u64) -> DataResult<bool> {
        let row = self
            .db
            .query_one(self.statement(
                r#"UPDATE replication_jobs SET retry_count = retry_count + 1, error_message = $2,
               status = CASE WHEN retry_count + 1 >= max_retries THEN 'failed' ELSE 'pending' END,
               next_retry_at = CASE WHEN retry_count + 1 >= max_retries THEN NULL
                 ELSE NOW() + (($3 * POWER(2, retry_count)) || ' seconds')::INTERVAL END,
               completed_at = CASE WHEN retry_count + 1 >= max_retries THEN NOW() ELSE NULL END
               WHERE id = $1 RETURNING (status = 'failed') AS permanently_failed"#,
                vec![id.into(), error.into(), (retry_seconds as i64).into()],
            ))
            .await?
            .ok_or(sea_orm::DbErr::RecordNotFound("replication job".into()))?;
        Ok(row.try_get("", "permanently_failed")?)
    }

    pub async fn stats(&self) -> DataResult<ReplicationStats> {
        let row = self.db.query_one(self.statement(
            r#"SELECT COUNT(*) FILTER (WHERE status = 'pending') AS pending_jobs,
               COUNT(*) FILTER (WHERE status = 'processing') AS processing_jobs,
               COUNT(*) FILTER (WHERE status = 'failed') AS failed_jobs,
               COUNT(*) FILTER (WHERE status = 'completed' AND completed_at > NOW() - INTERVAL '1 hour') AS completed_last_hour,
               EXTRACT(EPOCH FROM (NOW() - MIN(created_at) FILTER (WHERE status = 'pending')))::BIGINT AS oldest_pending_age_seconds
               FROM replication_jobs"#,
            vec![],
        )).await?.ok_or(sea_orm::DbErr::RecordNotFound("replication stats".into()))?;
        Ok(ReplicationStats {
            pending_jobs: row.try_get("", "pending_jobs")?,
            processing_jobs: row.try_get("", "processing_jobs")?,
            failed_jobs: row.try_get("", "failed_jobs")?,
            completed_last_hour: row.try_get("", "completed_last_hour")?,
            oldest_pending_age_seconds: row.try_get("", "oldest_pending_age_seconds")?,
        })
    }

    pub async fn jobs(
        &self,
        status: &str,
        limit: i64,
        offset: i64,
    ) -> DataResult<Vec<replication_jobs::Model>> {
        Ok(replication_jobs::Model::find_by_statement(self.statement(
            "SELECT * FROM replication_jobs WHERE status = $1 ORDER BY created_at ASC LIMIT $2 OFFSET $3",
            vec![status.into(), limit.into(), offset.into()],
        )).all(self.db).await?)
    }

    pub async fn retry_failed(&self) -> DataResult<i64> {
        let result = self.db.execute(self.statement(
            "UPDATE replication_jobs SET status = 'pending', retry_count = 0, next_retry_at = NOW(), error_message = NULL, completed_at = NULL WHERE status = 'failed'",
            vec![],
        )).await?;
        Ok(result.rows_affected() as i64)
    }
}
