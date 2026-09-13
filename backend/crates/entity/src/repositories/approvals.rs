use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, Statement,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities::{approval_policies, approval_requests},
    DataResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApprovalRow {
    pub id: Uuid,
    pub file_id: Uuid,
    pub tenant_id: Uuid,
    pub policy_id: Option<Uuid>,
    pub requested_by: Uuid,
    pub status: String,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub file_name: String,
    pub file_size: i64,
    pub content_type: Option<String>,
    pub department_id: Option<Uuid>,
    pub uploader_email: String,
    pub uploader_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalHistoryRow {
    pub id: Uuid,
    pub file_id: Uuid,
    pub tenant_id: Uuid,
    pub requested_by: Uuid,
    pub status: String,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub file_name: String,
    pub file_size: i64,
    pub content_type: Option<String>,
    pub uploader_email: String,
    pub uploader_name: Option<String>,
    pub decider_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPendingApprovalRow {
    pub id: Uuid,
    pub file_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub file_name: String,
    pub file_size: i64,
    pub content_type: Option<String>,
    pub rejection_reason: Option<String>,
}

pub struct ApprovalRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ApprovalRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn list_active_policies(&self, tenant_id: Uuid) -> DataResult<Vec<approval_policies::Model>> {
        Ok(approval_policies::Entity::find()
            .filter(approval_policies::Column::TenantId.eq(tenant_id))
            .filter(approval_policies::Column::IsActive.eq(true))
            .order_by_asc(approval_policies::Column::Scope)
            .all(self.db)
            .await?)
    }

    pub async fn list_policies(&self, tenant_id: Uuid) -> DataResult<Vec<approval_policies::Model>> {
        Ok(approval_policies::Entity::find()
            .filter(approval_policies::Column::TenantId.eq(tenant_id))
            .order_by_asc(approval_policies::Column::CreatedAt)
            .all(self.db)
            .await?)
    }

    pub async fn by_policy_id(&self, tenant_id: Uuid, id: Uuid) -> DataResult<Option<approval_policies::Model>> {
        Ok(approval_policies::Entity::find_by_id(id)
            .filter(approval_policies::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?)
    }

    pub async fn create_policy(
        &self,
        tenant_id: Uuid,
        name: String,
        scope: String,
        scope_value: Option<String>,
    ) -> DataResult<approval_policies::Model> {
        let now = chrono::Utc::now().into();
        let active = approval_policies::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            name: Set(name),
            scope: Set(scope),
            scope_value: Set(scope_value),
            required_approvals: Set(1),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn update_policy(&self, active: approval_policies::ActiveModel) -> DataResult<approval_policies::Model> {
        Ok(active.update(self.db).await?)
    }

    pub async fn delete_policy(&self, tenant_id: Uuid, id: Uuid) -> DataResult<bool> {
        let res = approval_policies::Entity::delete_many()
            .filter(approval_policies::Column::Id.eq(id))
            .filter(approval_policies::Column::TenantId.eq(tenant_id))
            .exec(self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn list_pending(
        &self,
        company_id: Uuid,
        limit: u64,
        offset: u64,
        department_id: Option<Uuid>,
    ) -> DataResult<Vec<PendingApprovalRow>> {
        let (sql, values) = if let Some(dept_id) = department_id {
            (
                r#"SELECT ar.id, ar.file_id, ar.tenant_id, ar.policy_id, ar.requested_by, ar.status,
                          ar.decided_by, ar.decided_at, ar.rejection_reason, ar.created_at,
                          fm.name as file_name, fm.size_bytes, fm.content_type, fm.department_id,
                          u.email as uploader_email, u.name as uploader_name
                   FROM approval_requests ar
                   JOIN files_metadata fm ON fm.id = ar.file_id
                   JOIN users u ON u.id = ar.requested_by
                   WHERE ar.tenant_id = $1 AND ar.status = 'pending' AND fm.department_id = $4
                   ORDER BY ar.created_at DESC
                   LIMIT $2 OFFSET $3"#,
                vec![
                    company_id.into(),
                    (limit as i64).into(),
                    (offset as i64).into(),
                    dept_id.into(),
                ],
            )
        } else {
            (
                r#"SELECT ar.id, ar.file_id, ar.tenant_id, ar.policy_id, ar.requested_by, ar.status,
                          ar.decided_by, ar.decided_at, ar.rejection_reason, ar.created_at,
                          fm.name as file_name, fm.size_bytes, fm.content_type, fm.department_id,
                          u.email as uploader_email, u.name as uploader_name
                   FROM approval_requests ar
                   JOIN files_metadata fm ON fm.id = ar.file_id
                   JOIN users u ON u.id = ar.requested_by
                   WHERE ar.tenant_id = $1 AND ar.status = 'pending'
                   ORDER BY ar.created_at DESC
                   LIMIT $2 OFFSET $3"#,
                vec![
                    company_id.into(),
                    (limit as i64).into(),
                    (offset as i64).into(),
                ],
            )
        };

        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), sql, values);
        let rows = self.db.query_all(stmt).await?;
        let mut list = Vec::new();
        for r in rows {
            list.push(PendingApprovalRow {
                id: r.try_get("", "id")?,
                file_id: r.try_get("", "file_id")?,
                tenant_id: r.try_get("", "tenant_id")?,
                policy_id: r.try_get("", "policy_id")?,
                requested_by: r.try_get("", "requested_by")?,
                status: r.try_get("", "status")?,
                decided_by: r.try_get("", "decided_by")?,
                decided_at: r.try_get("", "decided_at")?,
                rejection_reason: r.try_get("", "rejection_reason")?,
                created_at: r.try_get("", "created_at")?,
                file_name: r.try_get("", "file_name")?,
                file_size: r.try_get("", "size_bytes")?,
                content_type: r.try_get("", "content_type")?,
                department_id: r.try_get("", "department_id")?,
                uploader_email: r.try_get("", "uploader_email")?,
                uploader_name: r.try_get("", "uploader_name")?,
            });
        }
        Ok(list)
    }

    pub async fn list_history(
        &self,
        company_id: Uuid,
        limit: u64,
        offset: u64,
    ) -> DataResult<Vec<ApprovalHistoryRow>> {
        let sql = r#"SELECT ar.id, ar.file_id, ar.tenant_id, ar.requested_by, ar.status,
                      ar.decided_by, ar.decided_at, ar.rejection_reason, ar.created_at,
                      fm.name as file_name, fm.size_bytes, fm.content_type,
                      uploader.email as uploader_email, uploader.name as uploader_name,
                      decider.email as decider_email
               FROM approval_requests ar
               JOIN files_metadata fm ON fm.id = ar.file_id
               JOIN users uploader ON uploader.id = ar.requested_by
               LEFT JOIN users decider ON decider.id = ar.decided_by
               WHERE ar.tenant_id = $1 AND ar.status != 'pending'
               ORDER BY ar.decided_at DESC NULLS LAST
               LIMIT $2 OFFSET $3"#;

        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![
                company_id.into(),
                (limit as i64).into(),
                (offset as i64).into(),
            ],
        );
        let rows = self.db.query_all(stmt).await?;
        let mut list = Vec::new();
        for r in rows {
            list.push(ApprovalHistoryRow {
                id: r.try_get("", "id")?,
                file_id: r.try_get("", "file_id")?,
                tenant_id: r.try_get("", "tenant_id")?,
                requested_by: r.try_get("", "requested_by")?,
                status: r.try_get("", "status")?,
                decided_by: r.try_get("", "decided_by")?,
                decided_at: r.try_get("", "decided_at")?,
                rejection_reason: r.try_get("", "rejection_reason")?,
                created_at: r.try_get("", "created_at")?,
                file_name: r.try_get("", "file_name")?,
                file_size: r.try_get("", "size_bytes")?,
                content_type: r.try_get("", "content_type")?,
                uploader_email: r.try_get("", "uploader_email")?,
                uploader_name: r.try_get("", "uploader_name")?,
                decider_email: r.try_get("", "decider_email")?,
            });
        }
        Ok(list)
    }

    pub async fn list_my_pending(
        &self,
        company_id: Uuid,
        user_id: Uuid,
    ) -> DataResult<Vec<MyPendingApprovalRow>> {
        let sql = r#"SELECT ar.id, ar.file_id, ar.status, ar.created_at,
                      fm.name as file_name, fm.size_bytes, fm.content_type, ar.rejection_reason
               FROM approval_requests ar
               JOIN files_metadata fm ON fm.id = ar.file_id
               WHERE ar.tenant_id = $1 AND ar.requested_by = $2
               AND ar.status IN ('pending', 'rejected')
               ORDER BY ar.created_at DESC"#;

        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![company_id.into(), user_id.into()],
        );
        let rows = self.db.query_all(stmt).await?;
        let mut list = Vec::new();
        for r in rows {
            list.push(MyPendingApprovalRow {
                id: r.try_get("", "id")?,
                file_id: r.try_get("", "file_id")?,
                status: r.try_get("", "status")?,
                created_at: r.try_get("", "created_at")?,
                file_name: r.try_get("", "file_name")?,
                file_size: r.try_get("", "size_bytes")?,
                content_type: r.try_get("", "content_type")?,
                rejection_reason: r.try_get("", "rejection_reason")?,
            });
        }
        Ok(list)
    }

    pub async fn get_stats(&self, company_id: Uuid) -> DataResult<(i64, i64, i64)> {
        let sql = r#"SELECT 
            COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
            COUNT(*) FILTER (WHERE status = 'approved') as approved_count,
            COUNT(*) FILTER (WHERE status = 'rejected') as rejected_count
            FROM approval_requests WHERE tenant_id = $1"#;

        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![company_id.into()],
        );
        let row = self.db.query_one(stmt).await?.unwrap();
        let pending: i64 = row.try_get("", "pending_count")?;
        let approved: i64 = row.try_get("", "approved_count")?;
        let rejected: i64 = row.try_get("", "rejected_count")?;
        Ok((pending, approved, rejected))
    }

    pub async fn approve_request(
        &self,
        company_id: Uuid,
        request_id: Uuid,
        decided_by: Uuid,
    ) -> DataResult<Option<(Uuid, Uuid)>> {
        let sql = r#"UPDATE approval_requests
               SET status = 'approved', decided_by = $1, decided_at = NOW(), updated_at = NOW()
               WHERE id = $2 AND tenant_id = $3 AND status = 'pending'
               RETURNING file_id, requested_by"#;
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![decided_by.into(), request_id.into(), company_id.into()],
        );
        let row = self.db.query_one(stmt).await?;
        if let Some(r) = row {
            let file_id: Uuid = r.try_get("", "file_id")?;
            let requested_by: Uuid = r.try_get("", "requested_by")?;
            Ok(Some((file_id, requested_by)))
        } else {
            Ok(None)
        }
    }

    pub async fn reject_request(
        &self,
        company_id: Uuid,
        request_id: Uuid,
        decided_by: Uuid,
        reason: &str,
    ) -> DataResult<Option<(Uuid, Uuid)>> {
        let sql = r#"UPDATE approval_requests
               SET status = 'rejected', decided_by = $1, decided_at = NOW(),
                   rejection_reason = $4, updated_at = NOW()
               WHERE id = $2 AND tenant_id = $3 AND status = 'pending'
               RETURNING file_id, requested_by"#;
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![
                decided_by.into(),
                request_id.into(),
                company_id.into(),
                reason.into(),
            ],
        );
        let row = self.db.query_one(stmt).await?;
        if let Some(r) = row {
            let file_id: Uuid = r.try_get("", "file_id")?;
            let requested_by: Uuid = r.try_get("", "requested_by")?;
            Ok(Some((file_id, requested_by)))
        } else {
            Ok(None)
        }
    }

    pub async fn create_request(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        policy_id: Option<Uuid>,
        requested_by: Uuid,
    ) -> DataResult<Uuid> {
        let now = chrono::Utc::now().into();
        let new_id = Uuid::new_v4();
        let active = approval_requests::ActiveModel {
            id: Set(new_id),
            tenant_id: Set(tenant_id),
            file_id: Set(file_id),
            policy_id: Set(policy_id),
            requested_by: Set(requested_by),
            status: Set("pending".to_string()),
            step: Set(1),
            decided_by: Set(None),
            decided_at: Set(None),
            rejection_reason: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        active.insert(self.db).await?;
        Ok(new_id)
    }

    pub async fn update_file_approval_status(
        &self,
        file_id: Uuid,
        status: &str,
    ) -> DataResult<bool> {
        let sql = "UPDATE files_metadata SET approval_status = $1, updated_at = NOW() WHERE id = $2";
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![status.into(), file_id.into()],
        );
        let res = self.db.execute(stmt).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn get_file_info_for_approval(
        &self,
        company_id: Uuid,
        file_id: Uuid,
        check_not_deleted: bool,
    ) -> DataResult<Option<(Uuid, String, Option<Uuid>, bool)>> {
        let sql = if check_not_deleted {
            "SELECT owner_id, approval_status, department_id, is_company_folder FROM files_metadata WHERE id = $1 AND tenant_id = $2 AND is_deleted = false"
        } else {
            "SELECT owner_id, approval_status, department_id, is_company_folder FROM files_metadata WHERE id = $1 AND tenant_id = $2"
        };
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![file_id.into(), company_id.into()],
        );
        let row = self.db.query_one(stmt).await?;
        if let Some(r) = row {
            let owner_id: Uuid = r.try_get("", "owner_id")?;
            let approval_status: String = r.try_get("", "approval_status")?;
            let department_id: Option<Uuid> = r.try_get("", "department_id")?;
            let is_company_folder: bool = r.try_get("", "is_company_folder")?;
            Ok(Some((
                owner_id,
                approval_status,
                department_id,
                is_company_folder,
            )))
        } else {
            Ok(None)
        }
    }

    pub async fn get_file_and_uploader(
        &self,
        file_id: Uuid,
        user_id: Uuid,
    ) -> DataResult<Option<(String, String, String)>> {
        let sql = "SELECT fm.name, u.email, u.role FROM files_metadata fm JOIN users u ON u.id = $2 WHERE fm.id = $1";
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![file_id.into(), user_id.into()],
        );
        let row = self.db.query_one(stmt).await?;
        if let Some(r) = row {
            let name: String = r.try_get("", "name")?;
            let email: String = r.try_get("", "email")?;
            let role: String = r.try_get("", "role")?;
            Ok(Some((name, email, role)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_file_name(&self, file_id: Uuid) -> DataResult<Option<String>> {
        let sql = "SELECT name FROM files_metadata WHERE id = $1";
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![file_id.into()],
        );
        let row = self.db.query_one(stmt).await?;
        if let Some(r) = row {
            let name: String = r.try_get("", "name")?;
            Ok(Some(name))
        } else {
            Ok(None)
        }
    }
}
