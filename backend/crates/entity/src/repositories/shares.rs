use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{entities::file_shares, DataResult};

#[derive(Debug, Clone, Serialize, sea_orm::FromQueryResult)]
pub struct ShareableUserRow {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub department_id: Option<Uuid>,
    pub department_name: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SharedFileRow {
    pub id: Uuid,
    pub name: String,
    pub size: i64,
    pub content_type: Option<String>,
    pub folder_path: Option<String>,
    pub shared_by_id: Uuid,
    pub shared_by_name: String,
    pub shared_at: DateTime<Utc>,
    pub share_token: String,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct ShareRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ShareRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_shareable_users(
        &self,
        tenant_id: Uuid,
        exclude_user_id: Uuid,
        accessible_dept_ids: Option<&[Uuid]>,
        search: Option<&str>,
    ) -> DataResult<Vec<ShareableUserRow>> {
        let mut where_clauses = vec![
            "u.tenant_id = $1".to_string(),
            "u.id != $2".to_string(),
            "u.status = 'active'".to_string(),
        ];
        let mut values: Vec<sea_orm::Value> = vec![tenant_id.into(), exclude_user_id.into()];
        let mut param_idx = 3;

        if let Some(depts) = accessible_dept_ids {
            if depts.is_empty() {
                return Ok(Vec::new());
            }
            where_clauses.push(format!("u.department_id = ANY(${})", param_idx));
            values.push(depts.to_vec().into());
            param_idx += 1;
        }

        if let Some(s) = search {
            let pattern = format!("%{}%", s.to_lowercase());
            where_clauses.push(format!(
                "(LOWER(u.name) LIKE ${} OR LOWER(u.email) LIKE ${})",
                param_idx, param_idx
            ));
            values.push(pattern.into());
        }

        let sql = format!(
            r#"
            SELECT 
                u.id,
                u.name,
                u.email,
                u.department_id,
                d.name as department_name,
                u.role
            FROM users u
            LEFT JOIN departments d ON u.department_id = d.id
            WHERE {}
            ORDER BY u.name
            LIMIT 50
            "#,
            where_clauses.join(" AND ")
        );

        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), &sql, values);
        let rows = self.db.query_all(stmt).await?;
        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            list.push(ShareableUserRow {
                id: row.try_get_by_index(0)?,
                name: row.try_get_by_index(1)?,
                email: row.try_get_by_index(2)?,
                department_id: row.try_get_by_index(3).ok(),
                department_name: row.try_get_by_index(4).ok(),
                role: row.try_get_by_index(5)?,
            });
        }
        Ok(list)
    }

    pub async fn list_shared_with_me(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> DataResult<(Vec<SharedFileRow>, i64)> {
        let count_sql = r#"
            SELECT COUNT(*) 
            FROM file_shares fs
            JOIN files_metadata fm ON fs.file_id = fm.id
            WHERE fs.shared_with_user_id = $1 
              AND fs.tenant_id = $2
              AND (fs.expires_at IS NULL OR fs.expires_at > NOW())
        "#;
        let count_stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            count_sql,
            vec![user_id.into(), tenant_id.into()],
        );
        let count_row = self.db.query_one(count_stmt).await?;
        let total: i64 = match count_row {
            Some(r) => r.try_get_by_index(0).unwrap_or(0),
            None => 0,
        };

        let data_sql = r#"
            SELECT 
                fm.id,
                fm.name,
                fm.size_bytes,
                fm.content_type,
                fm.parent_path,
                u.id as shared_by_id,
                u.name as shared_by_name,
                fs.created_at as shared_at,
                fs.token,
                fs.expires_at
            FROM file_shares fs
            JOIN files_metadata fm ON fs.file_id = fm.id
            JOIN users u ON fs.created_by = u.id
            WHERE fs.shared_with_user_id = $1 
              AND fs.tenant_id = $2
              AND (fs.expires_at IS NULL OR fs.expires_at > NOW())
            ORDER BY fs.created_at DESC
            LIMIT $3 OFFSET $4
        "#;
        let data_stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            data_sql,
            vec![
                user_id.into(),
                tenant_id.into(),
                (limit as i64).into(),
                (offset as i64).into(),
            ],
        );
        let rows = self.db.query_all(data_stmt).await?;
        let mut files = Vec::with_capacity(rows.len());
        for row in rows {
            let id: Uuid = row.try_get_by_index(0)?;
            let name: String = row.try_get_by_index(1)?;
            let size: i64 = row.try_get_by_index(2)?;
            let content_type: Option<String> = row.try_get_by_index(3).ok();
            let folder_path: Option<String> = row.try_get_by_index(4).ok();
            let shared_by_id: Uuid = row.try_get_by_index(5)?;
            let shared_by_name: String = row.try_get_by_index(6)?;
            let shared_at: DateTime<FixedOffset> = row.try_get_by_index(7)?;
            let share_token: String = row.try_get_by_index(8)?;
            let expires_at: Option<DateTime<FixedOffset>> = row.try_get_by_index(9).ok();

            files.push(SharedFileRow {
                id,
                name,
                size,
                content_type,
                folder_path,
                shared_by_id,
                shared_by_name,
                shared_at: shared_at.with_timezone(&Utc),
                share_token,
                expires_at: expires_at.map(|e| e.with_timezone(&Utc)),
            });
        }

        Ok((files, total))
    }

    pub async fn get_user_share(
        &self,
        token: &str,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<Option<file_shares::Model>> {
        Ok(file_shares::Entity::find()
            .filter(file_shares::Column::Token.eq(token))
            .filter(file_shares::Column::SharedWithUserId.eq(Some(user_id)))
            .filter(file_shares::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?)
    }

    pub async fn by_token(&self, token: &str) -> DataResult<Option<file_shares::Model>> {
        Ok(file_shares::Entity::find()
            .filter(file_shares::Column::Token.eq(token))
            .one(self.db)
            .await?)
    }

    pub async fn increment_download_count(&self, token: &str) -> DataResult<()> {
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            "UPDATE file_shares SET download_count = download_count + 1 WHERE token = $1",
            vec![token.into()],
        );
        self.db.execute(stmt).await?;
        Ok(())
    }

    pub async fn create_share(
        &self,
        file_id: Uuid,
        tenant_id: Uuid,
        token: &str,
        created_by: Uuid,
        is_public: bool,
        expires_at: Option<DateTime<Utc>>,
        is_directory: bool,
        share_policy: Option<&str>,
        shared_with_user_id: Option<Uuid>,
    ) -> DataResult<Uuid> {
        let id = Uuid::new_v4();
        let expires_tz = expires_at.map(|e| e.into());
        let active = file_shares::ActiveModel {
            id: sea_orm::ActiveValue::Set(id),
            file_id: sea_orm::ActiveValue::Set(file_id),
            tenant_id: sea_orm::ActiveValue::Set(tenant_id),
            token: sea_orm::ActiveValue::Set(token.to_string()),
            created_by: sea_orm::ActiveValue::Set(created_by),
            is_public: sea_orm::ActiveValue::Set(is_public),
            is_directory: sea_orm::ActiveValue::Set(is_directory),
            share_policy: sea_orm::ActiveValue::Set(share_policy.map(str::to_string)),
            expires_at: sea_orm::ActiveValue::Set(expires_tz),
            download_count: sea_orm::ActiveValue::Set(0),
            created_at: sea_orm::ActiveValue::Set(Utc::now().into()),
            shared_with_user_id: sea_orm::ActiveValue::Set(shared_with_user_id),
        };
        file_shares::Entity::insert(active).exec(self.db).await?;
        Ok(id)
    }
}
