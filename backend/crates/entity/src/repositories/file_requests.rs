use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

use crate::{
    entities::{file_request_uploads, file_requests},
    DataResult,
};

#[derive(Debug, Clone)]
pub struct ListFileRequestsFilter {
    pub tenant_id: Uuid,
    pub visibility: String,
    pub user_id: Uuid,
    pub is_admin: bool,
    pub user_department_id: Option<Uuid>,
    pub department_id_filter: Option<Uuid>,
    pub status: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_before: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub limit: u64,
    pub offset: u64,
}

pub struct FileRequestRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> FileRequestRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn by_id(&self, id: Uuid) -> DataResult<Option<file_requests::Model>> {
        Ok(file_requests::Entity::find_by_id(id).one(self.db).await?)
    }

    pub async fn by_tenant_and_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> DataResult<Option<file_requests::Model>> {
        Ok(file_requests::Entity::find_by_id(id)
            .filter(file_requests::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?)
    }

    pub async fn by_active_token(
        &self,
        token: &str,
    ) -> DataResult<Option<file_requests::Model>> {
        Ok(file_requests::Entity::find()
            .filter(file_requests::Column::Token.eq(token))
            .filter(file_requests::Column::Status.eq("active"))
            .one(self.db)
            .await?)
    }

    pub async fn list_expiring(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        three_days: chrono::DateTime<chrono::Utc>,
    ) -> DataResult<Vec<file_requests::Model>> {
        Ok(file_requests::Entity::find()
            .filter(file_requests::Column::Status.eq("active"))
            .filter(file_requests::Column::ExpiresAt.gt(now))
            .filter(file_requests::Column::ExpiresAt.lte(three_days))
            .all(self.db)
            .await?)
    }

    pub async fn list_filtered(
        &self,
        filter: ListFileRequestsFilter,
    ) -> DataResult<Vec<file_requests::Model>> {
        let mut query = file_requests::Entity::find()
            .filter(file_requests::Column::TenantId.eq(filter.tenant_id));

        if filter.visibility == "private" {
            query = query
                .filter(file_requests::Column::Visibility.eq("private"))
                .filter(file_requests::Column::CreatedBy.eq(filter.user_id));
        } else {
            query = query.filter(file_requests::Column::Visibility.eq("department"));
            if filter.is_admin {
                if let Some(dept_id) = filter.department_id_filter {
                    query = query.filter(file_requests::Column::DepartmentId.eq(dept_id));
                }
            } else if let Some(dept_id) = filter.user_department_id {
                query = query.filter(file_requests::Column::DepartmentId.eq(dept_id));
            } else {
                query = query.filter(file_requests::Column::DepartmentId.is_null());
            }
        }

        if let Some(status) = filter.status {
            query = query.filter(file_requests::Column::Status.eq(status));
        }
        if let Some(created_after) = filter.created_after {
            query = query.filter(file_requests::Column::CreatedAt.gte(created_after));
        }
        if let Some(created_before) = filter.created_before {
            query = query.filter(file_requests::Column::CreatedAt.lte(created_before));
        }

        query = query
            .order_by_desc(file_requests::Column::CreatedAt)
            .limit(filter.limit)
            .offset(filter.offset);

        Ok(query.all(self.db).await?)
    }

    pub async fn create(
        &self,
        tenant_id: Uuid,
        department_id: Option<Uuid>,
        name: String,
        destination_path: String,
        token: String,
        created_by: Uuid,
        expires_at: chrono::DateTime<chrono::Utc>,
        max_uploads: Option<i32>,
        visibility: String,
    ) -> DataResult<file_requests::Model> {
        let now = chrono::Utc::now().into();
        let active = file_requests::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            department_id: Set(department_id),
            name: Set(name),
            destination_path: Set(destination_path),
            token: Set(token),
            created_by: Set(created_by),
            expires_at: Set(expires_at.into()),
            status: Set("active".to_string()),
            visibility: Set(visibility),
            upload_count: Set(0),
            max_uploads: Set(max_uploads),
            created_at: Set(now),
            updated_at: Set(now),
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn revoke(&self, tenant_id: Uuid, id: Uuid) -> DataResult<bool> {
        let Some(model) = self.by_tenant_and_id(tenant_id, id).await? else {
            return Ok(false);
        };
        let mut active: file_requests::ActiveModel = model.into();
        active.status = Set("revoked".to_string());
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn delete_permanent(&self, tenant_id: Uuid, id: Uuid) -> DataResult<bool> {
        let res = file_requests::Entity::delete_many()
            .filter(file_requests::Column::Id.eq(id))
            .filter(file_requests::Column::TenantId.eq(tenant_id))
            .exec(self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn delete_uploads(&self, file_request_id: Uuid) -> DataResult<u64> {
        let res = file_request_uploads::Entity::delete_many()
            .filter(file_request_uploads::Column::FileRequestId.eq(file_request_id))
            .exec(self.db)
            .await?;
        Ok(res.rows_affected)
    }

    pub async fn list_uploads(
        &self,
        file_request_id: Uuid,
    ) -> DataResult<Vec<file_request_uploads::Model>> {
        Ok(file_request_uploads::Entity::find()
            .filter(file_request_uploads::Column::FileRequestId.eq(file_request_id))
            .order_by_desc(file_request_uploads::Column::UploadedAt)
            .all(self.db)
            .await?)
    }

    pub async fn create_upload(
        &self,
        file_request_id: Uuid,
        file_metadata_id: Option<Uuid>,
        filename: String,
        original_filename: String,
        size_bytes: i64,
        content_type: Option<String>,
        storage_path: String,
    ) -> DataResult<file_request_uploads::Model> {
        let active = file_request_uploads::ActiveModel {
            id: Set(Uuid::new_v4()),
            file_request_id: Set(file_request_id),
            file_metadata_id: Set(file_metadata_id),
            filename: Set(filename),
            original_filename: Set(original_filename),
            size_bytes: Set(size_bytes),
            content_type: Set(content_type),
            storage_path: Set(storage_path),
            uploaded_by_email: Set(None),
            uploaded_at: Set(chrono::Utc::now().into()),
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn increment_upload_count(&self, id: Uuid, count: i32) -> DataResult<bool> {
        let Some(model) = self.by_id(id).await? else {
            return Ok(false);
        };
        let mut active: file_requests::ActiveModel = model.into();
        let current_count = active.upload_count.as_ref();
        active.upload_count = Set(current_count + count);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }
}
