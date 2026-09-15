use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Statement,
};
use uuid::Uuid;

use crate::{
    entities::{deletion_requests, file_exports, files_metadata, user_consent, user_preferences},
    DataResult,
};

pub struct ComplianceRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ComplianceRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn record_consent(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        consent_type: String,
        metadata: Option<serde_json::Value>,
    ) -> DataResult<user_consent::Model> {
        let now = chrono::Utc::now().into();
        let existing = user_consent::Entity::find()
            .filter(user_consent::Column::UserId.eq(user_id))
            .filter(user_consent::Column::TenantId.eq(tenant_id))
            .filter(user_consent::Column::ConsentType.eq(&consent_type))
            .one(self.db)
            .await?;

        if let Some(model) = existing {
            let mut active: user_consent::ActiveModel = model.into();
            active.granted_at = Set(now);
            active.revoked_at = Set(None);
            active.metadata = Set(metadata);
            active.updated_at = Set(now);
            Ok(active.update(self.db).await?)
        } else {
            let active = user_consent::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(user_id),
                tenant_id: Set(tenant_id),
                consent_type: Set(consent_type),
                granted_at: Set(now),
                revoked_at: Set(None),
                metadata: Set(metadata),
                created_at: Set(now),
                updated_at: Set(now),
            };
            Ok(active.insert(self.db).await?)
        }
    }

    pub async fn list_consents(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<Vec<user_consent::Model>> {
        Ok(user_consent::Entity::find()
            .filter(user_consent::Column::UserId.eq(user_id))
            .filter(user_consent::Column::TenantId.eq(tenant_id))
            .order_by_desc(user_consent::Column::GrantedAt)
            .all(self.db)
            .await?)
    }

    pub async fn revoke_consent(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        consent_type: &str,
    ) -> DataResult<bool> {
        let existing = user_consent::Entity::find()
            .filter(user_consent::Column::UserId.eq(user_id))
            .filter(user_consent::Column::TenantId.eq(tenant_id))
            .filter(user_consent::Column::ConsentType.eq(consent_type))
            .filter(user_consent::Column::RevokedAt.is_null())
            .one(self.db)
            .await?;

        if let Some(model) = existing {
            let now = chrono::Utc::now().into();
            let mut active: user_consent::ActiveModel = model.into();
            active.revoked_at = Set(Some(now));
            active.updated_at = Set(now);
            active.update(self.db).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn create_deletion_request(
        &self,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        requested_by: Uuid,
        request_type: String,
        resource_id: Option<Uuid>,
        reason: Option<String>,
    ) -> DataResult<deletion_requests::Model> {
        let now = chrono::Utc::now().into();
        let active = deletion_requests::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            user_id: Set(user_id),
            requested_by: Set(requested_by),
            request_type: Set(request_type),
            resource_id: Set(resource_id),
            reason: Set(reason),
            requested_at: Set(now),
            processed_at: Set(None),
            completed_at: Set(None),
            status: Set("pending".to_string()),
            rejection_reason: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn list_deletion_requests(
        &self,
        tenant_id: Uuid,
        status: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> DataResult<(Vec<deletion_requests::Model>, i64)> {
        let mut query = deletion_requests::Entity::find()
            .filter(deletion_requests::Column::TenantId.eq(tenant_id));

        if let Some(s) = status {
            query = query.filter(deletion_requests::Column::Status.eq(s));
        }

        let total = query.clone().count(self.db).await? as i64;
        let items = query
            .order_by_desc(deletion_requests::Column::RequestedAt)
            .offset(offset)
            .limit(limit)
            .all(self.db)
            .await?;

        Ok((items, total))
    }

    pub async fn get_deletion_request(
        &self,
        request_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<Option<deletion_requests::Model>> {
        Ok(deletion_requests::Entity::find_by_id(request_id)
            .filter(deletion_requests::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?)
    }

    pub async fn process_and_execute_deletion(
        &self,
        request_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<Option<deletion_requests::Model>> {
        let Some(req) = self.get_deletion_request(request_id, tenant_id).await? else {
            return Ok(None);
        };

        if req.status != "pending" {
            return Ok(None);
        }

        let now = chrono::Utc::now().into();

        // Mark as processing
        let mut active: deletion_requests::ActiveModel = req.clone().into();
        active.status = Set("processing".to_string());
        active.processed_at = Set(Some(now));
        active.updated_at = Set(now);
        active.update(self.db).await?;

        // Execute actions
        match req.request_type.as_str() {
            "user_data" => {
                if let Some(user_id) = req.user_id {
                    // Soft delete user files
                    files_metadata::Entity::update_many()
                        .col_expr(files_metadata::Column::IsDeleted, sea_orm::sea_query::Expr::value(true))
                        .col_expr(files_metadata::Column::DeletedAt, sea_orm::sea_query::Expr::value(Some(now)))
                        .filter(files_metadata::Column::OwnerId.eq(Some(user_id)))
                        .filter(files_metadata::Column::TenantId.eq(tenant_id))
                        .exec(self.db)
                        .await?;

                    // Delete user preferences
                    user_preferences::Entity::delete_many()
                        .filter(user_preferences::Column::UserId.eq(user_id))
                        .exec(self.db)
                        .await?;
                }
            }
            "file" => {
                if let Some(file_id) = req.resource_id {
                    files_metadata::Entity::update_many()
                        .col_expr(files_metadata::Column::IsDeleted, sea_orm::sea_query::Expr::value(true))
                        .col_expr(files_metadata::Column::DeletedAt, sea_orm::sea_query::Expr::value(Some(now)))
                        .filter(files_metadata::Column::Id.eq(file_id))
                        .filter(files_metadata::Column::TenantId.eq(tenant_id))
                        .exec(self.db)
                        .await?;
                }
            }
            _ => {}
        }

        // Mark completed
        let mut completed: deletion_requests::ActiveModel = self
            .get_deletion_request(request_id, tenant_id)
            .await?
            .expect("request exists")
            .into();
        completed.status = Set("completed".to_string());
        completed.completed_at = Set(Some(now));
        completed.updated_at = Set(now);
        let final_model = completed.update(self.db).await?;

        Ok(Some(final_model))
    }

    pub async fn log_file_export(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        file_id: Option<Uuid>,
        export_type: &str,
        file_count: i32,
        total_size: Option<i64>,
        ip_address: Option<String>,
    ) -> DataResult<file_exports::Model> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"INSERT INTO file_exports (id, tenant_id, user_id, file_id, export_type, file_count, total_size_bytes, exported_at, ip_address, metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10)"#,
            vec![
                id.into(),
                tenant_id.into(),
                user_id.into(),
                file_id.into(),
                export_type.to_string().into(),
                file_count.into(),
                total_size.into(),
                now.into(),
                ip_address.clone().into(),
                None::<sea_orm::JsonValue>.into(),
            ],
        );
        self.db.execute(stmt).await?;
        Ok(file_exports::Model {
            id,
            tenant_id,
            user_id,
            file_id,
            export_type: export_type.to_string(),
            file_count: Some(file_count),
            total_size_bytes: total_size,
            exported_at: now.into(),
            ip_address,
            metadata: None,
        })
    }
}
