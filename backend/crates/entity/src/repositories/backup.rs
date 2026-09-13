use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::Value;

use crate::DataResult;

pub struct BackupRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> BackupRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
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
