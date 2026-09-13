use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use uuid::Uuid;

use crate::{
    entities::{departments, tenants},
    DataResult,
};

#[derive(Debug, Clone)]
pub struct ListTenantsFilter {
    pub status: Option<String>,
    pub plan: Option<String>,
    pub search: Option<String>,
    pub limit: u64,
    pub offset: u64,
}

pub struct TenantRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> TenantRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn by_id(&self, id: Uuid) -> DataResult<Option<tenants::Model>> {
        Ok(tenants::Entity::find_by_id(id).one(self.db).await?)
    }

    pub async fn tenant(&self, id: Uuid) -> DataResult<Option<tenants::Model>> {
        self.by_id(id).await
    }

    pub async fn compliance_mode(&self, id: Uuid) -> DataResult<Option<String>> {
        let item = tenants::Entity::find_by_id(id)
            .select_only()
            .column(tenants::Column::ComplianceMode)
            .into_tuple::<String>()
            .one(self.db)
            .await?;
        Ok(item)
    }

    pub async fn list_active(&self) -> DataResult<Vec<tenants::Model>> {
        Ok(tenants::Entity::find()
            .filter(tenants::Column::Status.eq("active"))
            .order_by_asc(tenants::Column::Name)
            .all(self.db)
            .await?)
    }

    pub async fn list_by_ids(
        &self,
        ids: &[Uuid],
        status: Option<&str>,
    ) -> DataResult<Vec<tenants::Model>> {
        let mut q = tenants::Entity::find().filter(tenants::Column::Id.is_in(ids.to_vec()));
        if let Some(s) = status {
            q = q.filter(tenants::Column::Status.eq(s));
        }
        q = q.order_by_asc(tenants::Column::Name);
        Ok(q.all(self.db).await?)
    }

    pub async fn list_filtered(
        &self,
        filter: ListTenantsFilter,
    ) -> DataResult<Vec<tenants::Model>> {
        let mut q = tenants::Entity::find();
        if let Some(status) = filter.status {
            q = q.filter(tenants::Column::Status.eq(status));
        }
        if let Some(plan) = filter.plan {
            q = q.filter(tenants::Column::Plan.eq(plan));
        }
        if let Some(search) = filter.search {
            q = q.filter(
                sea_orm::Condition::any()
                    .add(tenants::Column::Name.contains(&search))
                    .add(tenants::Column::Domain.contains(&search)),
            );
        }
        q = q
            .order_by_desc(tenants::Column::CreatedAt)
            .limit(filter.limit)
            .offset(filter.offset);
        Ok(q.all(self.db).await?)
    }

    pub async fn count_users(&self, tenant_id: Uuid) -> DataResult<i64> {
        let sql = "SELECT COUNT(*) AS count FROM users WHERE tenant_id = $1 OR $1 = ANY(allowed_tenant_ids)";
        let stmt = sea_orm::Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![tenant_id.into()],
        );
        let row = self
            .db
            .query_one(stmt)
            .await?
            .ok_or(sea_orm::DbErr::RecordNotFound("count".into()))?;
        Ok(row.try_get("", "count")?)
    }

    pub async fn list_retention_policies(&self) -> DataResult<Vec<(Uuid, i32)>> {
        let items = tenants::Entity::find()
            .select_only()
            .column(tenants::Column::Id)
            .column(tenants::Column::RetentionPolicyDays)
            .into_tuple::<(Uuid, i32)>()
            .all(self.db)
            .await?;
        Ok(items)
    }

    pub async fn list_with_storage_quota(&self) -> DataResult<Vec<tenants::Model>> {
        Ok(tenants::Entity::find()
            .filter(tenants::Column::Status.eq("active"))
            .filter(tenants::Column::StorageQuotaBytes.is_not_null())
            .all(self.db)
            .await?)
    }

    pub async fn update_storage_used(&self, id: Uuid, bytes: i64) -> DataResult<bool> {
        let Some(model) = self.by_id(id).await? else {
            return Ok(false);
        };
        let mut active: tenants::ActiveModel = model.into();
        active.storage_used_bytes = Set(Some(bytes));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn create_tenant(
        &self,
        name: String,
        domain: String,
        plan: String,
        storage_quota_bytes: Option<i64>,
        departments_list: Option<Vec<String>>,
    ) -> DataResult<tenants::Model> {
        let txn = self.db.begin().await?;
        let now = chrono::Utc::now().into();
        let new_id = Uuid::new_v4();

        let active = tenants::ActiveModel {
            id: Set(new_id),
            name: Set(name),
            domain: Set(domain),
            plan: Set(plan),
            storage_quota_bytes: Set(storage_quota_bytes),
            status: Set("active".to_string()),
            compliance_mode: Set("Standard".to_string()),
            encryption_standard: Set("AES-256-GCM".to_string()),
            retention_policy_days: Set(90),
            storage_used_bytes: Set(Some(0)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let tenant_model = active.insert(&txn).await?;

        if let Some(depts) = departments_list {
            for dept_name in depts {
                let dept_active = departments::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    tenant_id: Set(tenant_model.id),
                    name: Set(dept_name),
                    created_at: Set(chrono::Utc::now().into()),
                    updated_at: Set(chrono::Utc::now().into()),
                    ..Default::default()
                };
                dept_active.insert(&txn).await?;
            }
        }

        txn.commit().await?;
        Ok(tenant_model)
    }

    pub async fn suspend(&self, id: Uuid) -> DataResult<bool> {
        let Some(model) = self.by_id(id).await? else {
            return Ok(false);
        };
        let mut active: tenants::ActiveModel = model.into();
        active.status = Set("suspended".to_string());
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn unsuspend(&self, id: Uuid) -> DataResult<bool> {
        let Some(model) = self.by_id(id).await? else {
            return Ok(false);
        };
        let mut active: tenants::ActiveModel = model.into();
        active.status = Set("active".to_string());
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn delete_cascade(&self, id: Uuid) -> DataResult<(i64, i64)> {
        let user_count_sql = "SELECT COUNT(*) AS count FROM users WHERE tenant_id = $1";
        let file_count_sql = "SELECT COUNT(*) AS count FROM files_metadata WHERE tenant_id = $1";

        let u_stmt = sea_orm::Statement::from_sql_and_values(
            self.db.get_database_backend(),
            user_count_sql,
            vec![id.into()],
        );
        let u_row = self.db.query_one(u_stmt).await?.unwrap();
        let users_count: i64 = u_row.try_get("", "count")?;

        let f_stmt = sea_orm::Statement::from_sql_and_values(
            self.db.get_database_backend(),
            file_count_sql,
            vec![id.into()],
        );
        let f_row = self.db.query_one(f_stmt).await?.unwrap();
        let files_count: i64 = f_row.try_get("", "count")?;

        let txn = self.db.begin().await?;

        // 1. Delete file shares
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM file_shares WHERE file_id IN (SELECT id FROM files_metadata WHERE tenant_id = $1)",
            vec![id.into()],
        )).await?;

        // 2. Delete file requests
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM file_requests WHERE tenant_id = $1",
            vec![id.into()],
        )).await?;

        // 3. Delete files metadata
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM files_metadata WHERE tenant_id = $1",
            vec![id.into()],
        )).await?;

        // 4. Delete notifications
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM notifications WHERE tenant_id = $1",
            vec![id.into()],
        )).await?;

        // 5. Delete audit logs
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM audit_logs WHERE tenant_id = $1",
            vec![id.into()],
        )).await?;

        // 6. Delete password reset tokens
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM password_reset_tokens WHERE user_id IN (SELECT id FROM users WHERE tenant_id = $1)",
            vec![id.into()],
        )).await?;

        // 7. Delete users
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM users WHERE tenant_id = $1",
            vec![id.into()],
        )).await?;

        // 8. Delete departments
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM departments WHERE tenant_id = $1",
            vec![id.into()],
        )).await?;

        // 9. Delete tenant
        txn.execute(sea_orm::Statement::from_sql_and_values(
            txn.get_database_backend(),
            "DELETE FROM tenants WHERE id = $1",
            vec![id.into()],
        )).await?;

        txn.commit().await?;

        Ok((users_count, files_count))
    }

    pub async fn update_compliance_settings(
        &self,
        id: Uuid,
        compliance_mode: String,
        retention_days: i32,
        mfa_required: bool,
        public_sharing_enabled: bool,
        session_timeout: Option<i32>,
        data_export_enabled: bool,
    ) -> DataResult<bool> {
        let Some(model) = self.by_id(id).await? else {
            return Ok(false);
        };
        let need_enable_totp = mfa_required && !model.enable_totp.unwrap_or(false);
        let mut active: tenants::ActiveModel = model.into();
        active.compliance_mode = Set(compliance_mode);
        active.retention_policy_days = Set(retention_days);
        active.mfa_required = Set(Some(mfa_required));
        active.public_sharing_enabled = Set(Some(public_sharing_enabled));
        if session_timeout.is_some() {
            active.session_timeout_minutes = Set(session_timeout);
        }
        active.data_export_enabled = Set(Some(data_export_enabled));
        if need_enable_totp {
            active.enable_totp = Set(Some(true));
        }
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn update_blocked_extensions(
        &self,
        id: Uuid,
        extensions: Vec<String>,
    ) -> DataResult<bool> {
        let Some(model) = self.by_id(id).await? else {
            return Ok(false);
        };
        let mut active: tenants::ActiveModel = model.into();
        active.blocked_extensions = Set(Some(extensions));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn update_password_policy(
        &self,
        id: Uuid,
        policy: serde_json::Value,
    ) -> DataResult<bool> {
        let Some(model) = self.by_id(id).await? else {
            return Ok(false);
        };
        let mut active: tenants::ActiveModel = model.into();
        active.password_policy = Set(Some(policy));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn update_ip_restrictions(
        &self,
        id: Uuid,
        mode: String,
        allowlist: Vec<String>,
        blocklist: Vec<String>,
    ) -> DataResult<bool> {
        let Some(model) = self.by_id(id).await? else {
            return Ok(false);
        };
        let mut active: tenants::ActiveModel = model.into();
        active.ip_restriction_mode = Set(Some(mode));
        active.ip_allowlist = Set(Some(allowlist));
        active.ip_blocklist = Set(Some(blocklist));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn update(&self, active: tenants::ActiveModel) -> DataResult<tenants::Model> {
        Ok(active.update(self.db).await?)
    }
}

