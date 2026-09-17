use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Statement,
};
use uuid::Uuid;

use crate::{entities::files_metadata, DataResult};

#[derive(Debug, Clone, serde::Serialize, sea_orm::FromQueryResult)]
pub struct TrashItemRow {
    pub id: Uuid,
    pub name: String,
    pub parent_path: Option<String>,
    pub size_bytes: i64,
    pub is_directory: bool,
    pub deleted_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub owner_id: Option<Uuid>,
    pub visibility: Option<String>,
    pub owner_name: Option<String>,
}

#[derive(Debug, Default)]
pub struct ListTrashFilter {
    pub target_owner: Option<Uuid>,
    pub target_department: Option<Uuid>,
    pub is_admin: bool,
    pub current_user_id: Uuid,
}

pub struct FileRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> FileRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, file_id: Uuid) -> DataResult<Option<files_metadata::Model>> {
        Ok(files_metadata::Entity::find_by_id(file_id).one(self.db).await?)
    }

    pub async fn find_active_by_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> DataResult<Option<files_metadata::Model>> {
        Ok(files_metadata::Entity::find_by_id(file_id)
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .one(self.db)
            .await?)
    }

    pub async fn find_active_by_path(
        &self,
        tenant_id: Uuid,
        name: &str,
        parent_path: Option<&str>,
    ) -> DataResult<Option<files_metadata::Model>> {
        let mut q = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::Name.eq(name))
            .filter(files_metadata::Column::IsDeleted.eq(false));

        if let Some(parent) = parent_path {
            q = q.filter(files_metadata::Column::ParentPath.eq(parent));
        } else {
            q = q.filter(files_metadata::Column::ParentPath.is_null());
        }

        Ok(q.one(self.db).await?)
    }

    pub async fn find_deleted_by_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> DataResult<Option<files_metadata::Model>> {
        Ok(files_metadata::Entity::find_by_id(file_id)
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::IsDeleted.eq(true))
            .one(self.db)
            .await?)
    }

    pub async fn find_deleted_by_name(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> DataResult<Option<files_metadata::Model>> {
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::Name.eq(name))
            .filter(files_metadata::Column::IsDeleted.eq(true))
            .order_by_desc(files_metadata::Column::DeletedAt)
            .one(self.db)
            .await?)
    }

    pub async fn soft_delete_file_and_children(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        is_directory: bool,
        folder_path: &str,
    ) -> DataResult<()> {
        let stmt1 = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            "UPDATE files_metadata SET is_deleted = true, deleted_at = NOW() WHERE id = $1 AND tenant_id = $2",
            vec![file_id.into(), tenant_id.into()],
        );
        self.db.execute(stmt1).await?;

        if is_directory {
            let prefix = format!("{}/%", folder_path);
            let stmt2 = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                "UPDATE files_metadata SET is_deleted = true, deleted_at = NOW() WHERE tenant_id = $1 AND (parent_path = $2 OR parent_path LIKE $3) AND is_deleted = false",
                vec![tenant_id.into(), folder_path.into(), prefix.into()],
            );
            self.db.execute(stmt2).await?;
        }
        Ok(())
    }

    pub async fn list_trash(
        &self,
        tenant_id: Uuid,
        filter: ListTrashFilter,
    ) -> DataResult<Vec<TrashItemRow>> {
        let mut where_clauses = vec![
            "fm.tenant_id = $1".to_string(),
            "fm.is_deleted = true".to_string(),
        ];
        let mut values: Vec<sea_orm::Value> = vec![tenant_id.into()];

        if let Some(owner) = filter.target_owner {
            where_clauses.push(format!("fm.owner_id = ${}", values.len() + 1));
            values.push(owner.into());
        } else if let Some(dept_id) = filter.target_department {
            where_clauses.push(format!("u.department_id = ${}", values.len() + 1));
            values.push(dept_id.into());
        } else if !filter.is_admin {
            where_clauses.push(format!("fm.owner_id = ${}", values.len() + 1));
            values.push(filter.current_user_id.into());
        }

        let sql = format!(
            r#"
            SELECT 
                fm.id, fm.name, fm.parent_path, fm.size_bytes, 
                fm.is_directory, fm.deleted_at, fm.owner_id, 
                fm.visibility, u.name as owner_name
            FROM files_metadata fm
            LEFT JOIN users u ON fm.owner_id = u.id
            WHERE {}
            ORDER BY fm.deleted_at DESC
            "#,
            where_clauses.join(" AND ")
        );

        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), &sql, values);
        Ok(TrashItemRow::find_by_statement(stmt).all(self.db).await?)
    }

    pub async fn restore_file_and_children(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        is_directory: bool,
        folder_path: &str,
    ) -> DataResult<u64> {
        let stmt1 = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            "UPDATE files_metadata SET is_deleted = false, deleted_at = NULL WHERE id = $1 AND tenant_id = $2",
            vec![file_id.into(), tenant_id.into()],
        );
        let res1 = self.db.execute(stmt1).await?;
        let mut total = res1.rows_affected();

        if is_directory {
            let prefix = format!("{}/%", folder_path);
            let stmt2 = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                "UPDATE files_metadata SET is_deleted = false, deleted_at = NULL WHERE tenant_id = $1 AND (parent_path = $2 OR parent_path LIKE $3) AND is_deleted = true",
                vec![tenant_id.into(), folder_path.into(), prefix.into()],
            );
            let res2 = self.db.execute(stmt2).await?;
            total += res2.rows_affected();
        }
        Ok(total)
    }

    pub async fn list_deleted_children(
        &self,
        tenant_id: Uuid,
        folder_path: &str,
    ) -> DataResult<Vec<files_metadata::Model>> {
        let prefix = format!("{}/%", folder_path);
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::IsDeleted.eq(true))
            .filter(
                sea_orm::Condition::any()
                    .add(files_metadata::Column::ParentPath.eq(folder_path))
                    .add(files_metadata::Column::ParentPath.like(&prefix)),
            )
            .all(self.db)
            .await?)
    }

    pub async fn count_content_hash_references_excluding(
        &self,
        hash: &str,
        exclude_file_id: Uuid,
    ) -> DataResult<i64> {
        let count = files_metadata::Entity::find()
            .filter(files_metadata::Column::ContentHash.eq(hash))
            .filter(files_metadata::Column::Id.ne(exclude_file_id))
            .filter(files_metadata::Column::IsDirectory.eq(false))
            .count(self.db)
            .await?;
        Ok(count as i64)
    }

    pub async fn delete_file_metadata(&self, tenant_id: Uuid, file_id: Uuid) -> DataResult<bool> {
        let res = files_metadata::Entity::delete_many()
            .filter(files_metadata::Column::Id.eq(file_id))
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .exec(self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn by_tenant_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> DataResult<Option<files_metadata::Model>> {
        Ok(files_metadata::Entity::find_by_id(file_id)
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?)
    }

    async fn update(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        apply: impl FnOnce(&mut files_metadata::ActiveModel),
    ) -> DataResult<bool> {
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else {
            return Ok(false);
        };
        let mut active: files_metadata::ActiveModel = model.into();
        apply(&mut active);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn rename(&self, tenant_id: Uuid, file_id: Uuid, name: String) -> DataResult<bool> {
        if name.is_empty() || name.contains('\0') || name == "." || name == ".." {
            return Err(crate::DataError::InvalidQuery("invalid filename".into()));
        }
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else {
            return Ok(false);
        };
        if model.is_deleted {
            return Ok(false);
        }
        self.update(tenant_id, file_id, |active| active.name = Set(name))
            .await
    }

    pub async fn move_to(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        parent: Option<String>,
    ) -> DataResult<bool> {
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else {
            return Ok(false);
        };
        if model.is_deleted {
            return Ok(false);
        }
        self.update(tenant_id, file_id, |active| {
            active.parent_path = Set(parent)
        })
        .await
    }

    pub async fn set_deleted(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        deleted: bool,
    ) -> DataResult<bool> {
        self.update(tenant_id, file_id, |active| {
            active.is_deleted = Set(deleted);
            active.deleted_at = Set(deleted.then(|| chrono::Utc::now().into()));
        })
        .await
    }

    pub async fn count_active_content_references(&self, hash: &str) -> DataResult<u64> {
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::ContentHash.eq(hash))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .count(self.db)
            .await?)
    }

    pub async fn has_content_references(&self, hash: &str) -> DataResult<bool> {
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::ContentHash.eq(hash))
            .one(self.db)
            .await?
            .is_some())
    }

    pub async fn unreferenced_storage_paths(
        &self,
        limit: u64,
    ) -> DataResult<Vec<(String, String)>> {
        let deleted = files_metadata::Entity::find()
            .filter(files_metadata::Column::IsDeleted.eq(true))
            .filter(files_metadata::Column::ContentHash.is_not_null())
            .limit(limit)
            .all(self.db)
            .await?;
        let mut result = Vec::new();
        for file in deleted {
            if let Some(hash) = file.content_hash {
                if self.count_active_content_references(&hash).await? == 0
                    && !result.iter().any(|(_, h)| h == &hash)
                {
                    result.push((file.storage_path, hash));
                }
            }
        }
        Ok(result)
    }

    pub async fn permanently_delete(&self, tenant_id: Uuid, file_id: Uuid) -> DataResult<bool> {
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else {
            return Ok(false);
        };
        if !model.is_deleted {
            return Ok(false);
        }
        Ok(files_metadata::Entity::delete_by_id(file_id)
            .exec(self.db)
            .await?
            .rows_affected
            > 0)
    }

    pub async fn list_expired(
        &self,
        tenant_id: Uuid,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> DataResult<Vec<(String, String)>> {
        let items = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::IsDeleted.eq(true))
            .filter(files_metadata::Column::DeletedAt.lt(cutoff))
            .select_only()
            .column(files_metadata::Column::Name)
            .column(files_metadata::Column::StoragePath)
            .into_tuple::<(String, String)>()
            .all(self.db)
            .await?;
        Ok(items)
    }

    pub async fn delete_by_name(&self, tenant_id: Uuid, name: &str) -> DataResult<u64> {
        let res = files_metadata::Entity::delete_many()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::Name.eq(name))
            .exec(self.db)
            .await?;
        Ok(res.rows_affected)
    }

    pub async fn calculate_actual_storage(&self, tenant_id: Uuid) -> DataResult<i64> {
        use sea_orm::sea_query::Expr;
        let res: Option<i64> = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .filter(files_metadata::Column::IsDirectory.eq(false))
            .select_only()
            .expr(Expr::col(files_metadata::Column::SizeBytes).sum())
            .into_tuple::<Option<i64>>()
            .one(self.db)
            .await?
            .flatten();
        Ok(res.unwrap_or(0))
    }

    pub async fn create(
        &self,
        id: Uuid,
        tenant_id: Uuid,
        department_id: Option<Uuid>,
        name: String,
        storage_path: String,
        size_bytes: i64,
        content_type: Option<String>,
        owner_id: Uuid,
        parent_path: Option<String>,
        visibility: String,
        ulid: String,
        content_hash: Option<String>,
    ) -> DataResult<files_metadata::Model> {
        let now = chrono::Utc::now().into();
        let active = files_metadata::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            department_id: Set(department_id),
            name: Set(name),
            storage_path: Set(storage_path),
            size_bytes: Set(size_bytes),
            content_type: Set(content_type),
            is_directory: Set(false),
            owner_id: Set(Some(owner_id)),
            parent_path: Set(parent_path),
            visibility: Set(visibility),
            ulid: Set(Some(ulid)),
            content_hash: Set(content_hash),
            is_deleted: Set(false),
            is_locked: Set(false),
            approval_status: Set("approved".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn calculate_storage_used(&self, tenant_id: Uuid) -> DataResult<i64> {
        let sql = "SELECT COALESCE(SUM(size_bytes), 0)::bigint AS storage FROM files_metadata WHERE tenant_id = $1 AND is_deleted = false AND is_directory = false";
        let stmt = sea_orm::Statement::from_sql_and_values(
            self.db.get_database_backend(),
            sql,
            vec![tenant_id.into()],
        );
        let row = self
            .db
            .query_one(stmt)
            .await?
            .ok_or(sea_orm::DbErr::RecordNotFound("storage".into()))?;
        Ok(row.try_get("", "storage")?)
    }

    pub async fn lock_file(
        &self,
        file_id: Uuid,
        user_id: Uuid,
        password_hash: Option<String>,
        required_role: Option<String>,
    ) -> DataResult<bool> {
        let Some(model) = files_metadata::Entity::find_by_id(file_id).one(self.db).await? else {
            return Ok(false);
        };
        let mut active: files_metadata::ActiveModel = model.into();
        active.is_locked = Set(true);
        active.locked_by = Set(Some(user_id));
        active.locked_at = Set(Some(chrono::Utc::now().into()));
        active.lock_password_hash = Set(password_hash);
        active.lock_requires_role = Set(required_role);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn unlock_file(&self, file_id: Uuid) -> DataResult<bool> {
        let Some(model) = files_metadata::Entity::find_by_id(file_id).one(self.db).await? else {
            return Ok(false);
        };
        let mut active: files_metadata::ActiveModel = model.into();
        active.is_locked = Set(false);
        active.locked_by = Set(None);
        active.locked_at = Set(None);
        active.lock_password_hash = Set(None);
        active.lock_requires_role = Set(None);
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(true)
    }

    pub async fn toggle_company_folder(&self, tenant_id: Uuid, file_id: Uuid) -> DataResult<Option<bool>> {
        let Some(model) = self.by_tenant_id(tenant_id, file_id).await? else {
            return Ok(None);
        };
        if !model.is_directory {
            return Ok(None);
        }
        let new_value = !model.is_company_folder.unwrap_or(false);
        let mut active: files_metadata::ActiveModel = model.into();
        active.is_company_folder = Set(Some(new_value));
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;
        Ok(Some(new_value))
    }

    pub async fn is_folder_company_folder(
        &self,
        tenant_id: Uuid,
        name: &str,
        parent_path: Option<&str>,
    ) -> DataResult<bool> {
        let mut q = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::Name.eq(name))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .filter(files_metadata::Column::IsDirectory.eq(true));
        match parent_path {
            Some(p) => q = q.filter(files_metadata::Column::ParentPath.eq(p)),
            None => q = q.filter(files_metadata::Column::ParentPath.is_null()),
        }
        let item = q.one(self.db).await?;
        Ok(item.and_then(|f| f.is_company_folder).unwrap_or(false))
    }

    pub async fn list_files_in_folder_recursive(
        &self,
        tenant_id: Uuid,
        folder_path: &str,
    ) -> DataResult<Vec<(String, String, Option<String>, i64)>> {
        let prefix = format!("{}/%", folder_path);
        let items = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .filter(files_metadata::Column::IsDirectory.eq(false))
            .filter(
                sea_orm::Condition::any()
                    .add(files_metadata::Column::ParentPath.eq(folder_path))
                    .add(files_metadata::Column::ParentPath.like(&prefix)),
            )
            .select_only()
            .column(files_metadata::Column::Name)
            .column(files_metadata::Column::StoragePath)
            .column(files_metadata::Column::ParentPath)
            .column(files_metadata::Column::SizeBytes)
            .into_tuple::<(String, String, Option<String>, i64)>()
            .all(self.db)
            .await?;
        Ok(items)
    }

    pub async fn exists_sibling_name(
        &self,
        tenant_id: Uuid,
        name: &str,
        parent_path: Option<&str>,
        exclude_file_id: Option<Uuid>,
    ) -> DataResult<bool> {
        let mut q = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::Name.eq(name))
            .filter(files_metadata::Column::IsDeleted.eq(false));

        if let Some(pp) = parent_path {
            q = q.filter(files_metadata::Column::ParentPath.eq(pp));
        } else {
            q = q.filter(files_metadata::Column::ParentPath.is_null());
        }

        if let Some(exclude_id) = exclude_file_id {
            q = q.filter(files_metadata::Column::Id.ne(exclude_id));
        }

        Ok(q.one(self.db).await?.is_some())
    }

    pub async fn update_children_parent_path_for_rename(
        &self,
        tenant_id: Uuid,
        old_folder_path: &str,
        new_folder_path: &str,
    ) -> DataResult<()> {
        let stmt1 = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            "UPDATE files_metadata SET parent_path = $1 WHERE tenant_id = $2 AND parent_path = $3",
            vec![new_folder_path.into(), tenant_id.into(), old_folder_path.into()],
        );
        self.db.execute(stmt1).await?;

        let old_prefix = format!("{}/", old_folder_path);
        let nested_children = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::ParentPath.like(format!("{}%", old_prefix)))
            .all(self.db)
            .await?;

        for child in nested_children {
            if let Some(ref pp) = child.parent_path {
                let new_child_parent = pp.replacen(old_folder_path, new_folder_path, 1);
                let stmt = Statement::from_sql_and_values(
                    self.db.get_database_backend(),
                    "UPDATE files_metadata SET parent_path = $1 WHERE id = $2 AND tenant_id = $3",
                    vec![new_child_parent.into(), child.id.into(), tenant_id.into()],
                );
                self.db.execute(stmt).await?;
            }
        }
        Ok(())
    }

    pub async fn move_file_and_children(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        new_name: &str,
        new_parent_path: Option<&str>,
        target_dept_id: Option<Uuid>,
        target_visibility: &str,
        actor_user_id: Uuid,
        is_directory: bool,
        old_path: &str,
        new_path: &str,
    ) -> DataResult<()> {
        let owner_id = if target_visibility == "private" {
            Some(actor_user_id)
        } else {
            None
        };

        let mut active = match files_metadata::Entity::find_by_id(file_id)
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .one(self.db)
            .await?
        {
            Some(m) => m.into_active_model(),
            None => return Ok(()),
        };

        active.name = Set(new_name.to_string());
        active.parent_path = Set(new_parent_path.map(str::to_string));
        active.department_id = Set(target_dept_id);
        active.visibility = Set(target_visibility.to_string());
        if target_visibility == "private" {
            active.owner_id = Set(owner_id);
        }
        active.updated_at = Set(chrono::Utc::now().into());
        active.update(self.db).await?;

        if is_directory {
            let stmt1 = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                r#"
                UPDATE files_metadata 
                SET parent_path = $1, 
                    visibility = $2, 
                    owner_id = CASE WHEN $2 = 'private' THEN $5 ELSE owner_id END,
                    updated_at = NOW()
                WHERE tenant_id = $3 AND parent_path = $4 AND is_deleted = false
                "#,
                vec![
                    new_path.into(),
                    target_visibility.into(),
                    tenant_id.into(),
                    old_path.into(),
                    actor_user_id.into(),
                ],
            );
            self.db.execute(stmt1).await?;

            let stmt2 = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                r#"
                UPDATE files_metadata 
                SET parent_path = $1 || SUBSTRING(parent_path FROM LENGTH($2) + 1), 
                    visibility = $3, 
                    owner_id = CASE WHEN $3 = 'private' THEN $6 ELSE owner_id END,
                    updated_at = NOW()
                WHERE tenant_id = $4 AND parent_path LIKE $5 AND is_deleted = false
                "#,
                vec![
                    new_path.into(),
                    old_path.into(),
                    target_visibility.into(),
                    tenant_id.into(),
                    format!("{}/%", old_path).into(),
                    actor_user_id.into(),
                ],
            );
            self.db.execute(stmt2).await?;
        }
        Ok(())
    }

    pub async fn exists_file_with_visibility(
        &self,
        tenant_id: Uuid,
        name: &str,
        parent_path: Option<&str>,
        visibility: &str,
    ) -> DataResult<bool> {
        let mut q = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::Name.eq(name))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .filter(files_metadata::Column::Visibility.eq(visibility));

        if let Some(pp) = parent_path {
            q = q.filter(files_metadata::Column::ParentPath.eq(pp));
        } else {
            q = q.filter(files_metadata::Column::ParentPath.is_null());
        }

        Ok(q.one(self.db).await?.is_some())
    }

    pub async fn copy_file_metadata(
        &self,
        new_file_id: Uuid,
        tenant_id: Uuid,
        target_dept_id: Option<Uuid>,
        copy_name: &str,
        storage_path: &str,
        size_bytes: i64,
        content_type: Option<&str>,
        owner_id: Option<Uuid>,
        target_parent_path: Option<&str>,
        target_visibility: &str,
        new_ulid: &str,
    ) -> DataResult<()> {
        let now = chrono::Utc::now().into();
        let active = files_metadata::ActiveModel {
            id: Set(new_file_id),
            tenant_id: Set(tenant_id),
            department_id: Set(target_dept_id),
            name: Set(copy_name.to_string()),
            storage_path: Set(storage_path.to_string()),
            size_bytes: Set(size_bytes),
            content_type: Set(content_type.map(str::to_string)),
            is_directory: Set(false),
            owner_id: Set(owner_id),
            parent_path: Set(target_parent_path.map(str::to_string)),
            visibility: Set(target_visibility.to_string()),
            ulid: Set(Some(new_ulid.to_string())),
            is_deleted: Set(false),
            is_locked: Set(false),
            is_immutable: Set(Some(false)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        active.insert(self.db).await?;
        Ok(())
    }

    pub async fn copy_summary_if_exists(
        &self,
        new_file_id: Uuid,
        original_file_id: Uuid,
        tenant_id: Uuid,
    ) -> DataResult<()> {
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            INSERT INTO file_summaries (file_id, tenant_id, summary, content_hash)
            SELECT $1, tenant_id, summary, content_hash
            FROM file_summaries
            WHERE file_id = $2 AND tenant_id = $3
            ON CONFLICT (file_id) DO NOTHING
            "#,
            vec![new_file_id.into(), original_file_id.into(), tenant_id.into()],
        );
        self.db.execute(stmt).await?;
        Ok(())
    }

    pub async fn find_active_by_ids(
        &self,
        tenant_id: Uuid,
        file_ids: &[Uuid],
    ) -> DataResult<Vec<files_metadata::Model>> {
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::Id.is_in(file_ids.to_vec()))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .all(self.db)
            .await?)
    }

    pub async fn find_unhashed_files_for_migration(
        &self,
        limit: u64,
    ) -> DataResult<Vec<files_metadata::Model>> {
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::ContentHash.is_null())
            .filter(files_metadata::Column::IsDirectory.eq(false))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .order_by_asc(files_metadata::Column::CreatedAt)
            .limit(limit)
            .all(self.db)
            .await?)
    }

    pub async fn count_unhashed_files(&self) -> DataResult<i64> {
        let count = files_metadata::Entity::find()
            .filter(files_metadata::Column::ContentHash.is_null())
            .filter(files_metadata::Column::IsDirectory.eq(false))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .count(self.db)
            .await?;
        Ok(count as i64)
    }

    pub async fn update_content_hash_and_ulid(
        &self,
        file_id: Uuid,
        content_hash: &str,
        ulid: &str,
    ) -> DataResult<()> {
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            "UPDATE files_metadata SET content_hash = $1, ulid = $2 WHERE id = $3",
            vec![content_hash.into(), ulid.into(), file_id.into()],
        );
        self.db.execute(stmt).await?;
        Ok(())
    }

    pub async fn update_file_content(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        storage_path: &str,
        size_bytes: i64,
        content_hash: &str,
    ) -> DataResult<files_metadata::Model> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let file = files_metadata::Entity::find_by_id(file_id)
            .filter(files_metadata::Column::TenantId.eq(tenant_id))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .one(self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("File {} not found", file_id)))?;

        let mut active = file.into_active_model();
        active.storage_path = Set(storage_path.to_string());
        active.size_bytes = Set(size_bytes);
        active.content_hash = Set(Some(content_hash.to_string()));
        active.updated_at = Set(now);
        Ok(active.update(self.db).await?)
    }


    pub async fn create_folder(&self, p: CreateFolderParams<'_>) -> DataResult<files_metadata::Model> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let active = files_metadata::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(p.tenant_id),
            name: Set(p.name.to_string()),
            storage_path: Set(p.storage_path.to_string()),
            size_bytes: Set(0),
            content_type: Set(Some("directory".to_string())),
            is_directory: Set(true),
            owner_id: Set(Some(p.owner_id)),
            department_id: Set(p.department_id),
            parent_path: Set(p.parent_path.map(str::to_string)),
            visibility: Set(p.visibility.to_string()),
            is_company_folder: Set(Some(p.is_company_folder)),
            is_locked: Set(false),
            is_deleted: Set(false),
            approval_status: Set("approved".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn is_parent_company_folder(
        &self,
        tenant_id: Uuid,
        parent_name: &str,
        parent_parent_path: Option<&str>,
    ) -> DataResult<bool> {
        #[derive(sea_orm::FromQueryResult)]
        struct FolderRow {
            is_company_folder: Option<bool>,
        }
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            SELECT COALESCE(is_company_folder, false) as is_company_folder FROM files_metadata 
            WHERE tenant_id = $1 AND name = $2 AND parent_path IS NOT DISTINCT FROM $3 AND is_deleted = false AND is_directory = true
            LIMIT 1
            "#,
            vec![
                tenant_id.into(),
                parent_name.into(),
                parent_parent_path.map(Into::into).unwrap_or(sea_orm::Value::String(None)),
            ],
        );
        let row: Option<FolderRow> = FolderRow::find_by_statement(stmt).one(self.db).await?;
        Ok(row.and_then(|r| r.is_company_folder).unwrap_or(false))
    }

    pub async fn find_descendant_files(
        &self,
        tenant_id: Uuid,
        folder_path: &str,
    ) -> DataResult<Vec<files_metadata::Model>> {
        let prefix = format!("{}/%", folder_path);
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            SELECT * FROM files_metadata 
            WHERE tenant_id = $1 
            AND is_deleted = false 
            AND is_directory = false
            AND (parent_path = $2 OR parent_path LIKE $3)
            "#,
            vec![tenant_id.into(), folder_path.into(), prefix.into()],
        );
        Ok(files_metadata::Entity::find()
            .from_raw_sql(stmt)
            .all(self.db)
            .await?)
    }

    pub async fn calculate_folder_size(
        &self,
        tenant_id: Uuid,
        folder_path: &str,
        visibility: &str,
    ) -> DataResult<i64> {
        #[derive(sea_orm::FromQueryResult)]
        struct FolderSizeRow {
            total: Option<i64>,
        }
        let prefix = format!("{}/%", folder_path);
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            SELECT COALESCE(SUM(size_bytes), 0)::bigint as total
            FROM files_metadata 
            WHERE tenant_id = $1 
            AND is_deleted = false 
            AND is_directory = false
            AND visibility = $4
            AND (parent_path = $2 OR parent_path LIKE $3)
            "#,
            vec![
                tenant_id.into(),
                folder_path.into(),
                prefix.into(),
                visibility.into(),
            ],
        );
        let row: Option<FolderSizeRow> = FolderSizeRow::find_by_statement(stmt).one(self.db).await?;
        Ok(row.and_then(|r| r.total).unwrap_or(0))
    }

    pub async fn generate_unique_filename(
        &self,
        tenant_id: Uuid,
        original_name: &str,
        parent_path: &str,
        department_id: Option<Uuid>,
        visibility: &str,
    ) -> DataResult<String> {
        let (base_name, extension) = if let Some(dot_pos) = original_name.rfind('.') {
            (&original_name[..dot_pos], &original_name[dot_pos..])
        } else {
            (original_name, "")
        };

        for i in 1..1000 {
            let candidate = format!("{} ({}){}", base_name, i, extension);
            let stmt = Statement::from_sql_and_values(
                self.db.get_database_backend(),
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM files_metadata 
                    WHERE tenant_id = $1 
                    AND name = $2 
                    AND is_deleted = false
                    AND (parent_path = $3 OR (parent_path IS NULL AND $3 = ''))
                    AND (department_id IS NOT DISTINCT FROM $4)
                    AND visibility = $5
                ) as exists
                "#,
                vec![
                    tenant_id.into(),
                    candidate.clone().into(),
                    (if parent_path.is_empty() { "".to_string() } else { parent_path.to_string() }).into(),
                    department_id.map(Into::into).unwrap_or(sea_orm::Value::Uuid(None)),
                    visibility.into(),
                ],
            );

            #[derive(sea_orm::FromQueryResult)]
            struct ExistsRow {
                exists: Option<bool>,
            }
            let row: Option<ExistsRow> = ExistsRow::find_by_statement(stmt).one(self.db).await?;
            if !row.and_then(|r| r.exists).unwrap_or(false) {
                return Ok(candidate);
            }
        }

        Ok(format!("{}_{}", original_name, Uuid::new_v4()))
    }

    pub async fn find_existing_file_for_upload(
        &self,
        tenant_id: Uuid,
        name: &str,
        parent_path: &str,
        department_id: Option<Uuid>,
        visibility: &str,
    ) -> DataResult<Option<(Uuid, Option<i32>)>> {
        #[derive(sea_orm::FromQueryResult)]
        struct ExistingRow {
            id: Uuid,
            version: Option<i32>,
        }
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            SELECT id, version FROM files_metadata 
            WHERE tenant_id = $1 AND name = $2 AND is_deleted = false
            AND (parent_path = $3 OR (parent_path IS NULL AND $3 = ''))
            AND (department_id IS NOT DISTINCT FROM $4)
            AND visibility = $5
            ORDER BY version DESC NULLS LAST
            LIMIT 1
            "#,
            vec![
                tenant_id.into(),
                name.into(),
                (if parent_path.is_empty() { "".to_string() } else { parent_path.to_string() }).into(),
                department_id.map(Into::into).unwrap_or(sea_orm::Value::Uuid(None)),
                visibility.into(),
            ],
        );
        let row: Option<ExistingRow> = ExistingRow::find_by_statement(stmt).one(self.db).await?;
        Ok(row.map(|r| (r.id, r.version)))
    }

    pub async fn find_content_hash_storage_path(
        &self,
        tenant_id: Uuid,
        department_id: Option<Uuid>,
        content_hash: &str,
    ) -> DataResult<Option<String>> {
        #[derive(sea_orm::FromQueryResult)]
        struct HashStorageRow {
            storage_path: String,
        }
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            r#"
            SELECT storage_path FROM files_metadata 
            WHERE tenant_id = $1 
            AND (department_id IS NOT DISTINCT FROM $2)
            AND content_hash = $3
            AND is_deleted = false 
            AND is_directory = false
            LIMIT 1
            "#,
            vec![
                tenant_id.into(),
                department_id.map(Into::into).unwrap_or(sea_orm::Value::Uuid(None)),
                content_hash.into(),
            ],
        );
        let row: Option<HashStorageRow> = HashStorageRow::find_by_statement(stmt).one(self.db).await?;
        Ok(row.map(|r| r.storage_path))
    }

    pub async fn set_immutable(&self, id: Uuid, is_immutable: bool) -> DataResult<bool> {
        let res = files_metadata::Entity::update_many()
            .col_expr(files_metadata::Column::IsImmutable, sea_orm::sea_query::Expr::val(is_immutable).into())
            .filter(files_metadata::Column::Id.eq(id))
            .exec(self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn create_file(&self, p: CreateFileParams<'_>) -> DataResult<files_metadata::Model> {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        let active = files_metadata::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(p.tenant_id),
            name: Set(p.name.to_string()),
            storage_path: Set(p.storage_path.to_string()),
            size_bytes: Set(p.size_bytes),
            content_type: Set(Some(p.content_type.to_string())),
            is_directory: Set(false),
            owner_id: Set(Some(p.owner_id)),
            department_id: Set(p.department_id),
            parent_path: Set(p.parent_path.map(str::to_string)),
            version: Set(Some(p.version)),
            version_parent_id: Set(p.version_parent_id),
            is_immutable: Set(Some(false)),
            visibility: Set(p.visibility.to_string()),
            content_hash: Set(Some(p.content_hash.to_string())),
            ulid: Set(Some(p.ulid.to_string())),
            is_locked: Set(false),
            is_deleted: Set(false),
            approval_status: Set("approved".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        Ok(active.insert(self.db).await?)
    }

    pub async fn list_files(&self, filter: ListFilesFilter) -> DataResult<Vec<files_metadata::Model>> {
        let mut q = files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(filter.tenant_id))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .filter(files_metadata::Column::GroupId.is_null());

        if !filter.is_approver {
            q = q.filter(
                files_metadata::Column::ApprovalStatus.eq("approved")
                    .or(files_metadata::Column::OwnerId.eq(filter.user_id)),
            );
        }

        if filter.view_mode == "private" {
            q = q.filter(files_metadata::Column::Visibility.eq("private"));
            if let Some(owner) = filter.target_owner_id {
                q = q.filter(files_metadata::Column::OwnerId.eq(owner));
            } else {
                q = q.filter(files_metadata::Column::OwnerId.eq(filter.user_id));
            }
        } else {
            q = q.filter(files_metadata::Column::Visibility.eq("department"));
            if filter.is_admin {
                if let Some(dept_id) = filter.selected_department_id {
                    q = q.filter(files_metadata::Column::DepartmentId.eq(dept_id));
                }
            } else {
                let mut cond = sea_orm::Condition::any();
                cond = cond.add(files_metadata::Column::DepartmentId.is_null());
                if let Some(dept_id) = filter.user_department_id {
                    cond = cond.add(files_metadata::Column::DepartmentId.eq(dept_id));
                }
                if let Some(allowed) = filter.user_allowed_department_ids {
                    if !allowed.is_empty() {
                        cond = cond.add(files_metadata::Column::DepartmentId.is_in(allowed));
                    }
                }
                q = q.filter(cond);
            }
        }

        if let Some(ref path) = filter.parent_path {
            let p = path.trim_start_matches('/');
            if !p.is_empty() {
                q = q.filter(files_metadata::Column::ParentPath.eq(p));
            } else {
                q = q.filter(
                    files_metadata::Column::ParentPath.is_null()
                        .or(files_metadata::Column::ParentPath.eq("")),
                );
            }
        } else {
            q = q.filter(
                files_metadata::Column::ParentPath.is_null()
                    .or(files_metadata::Column::ParentPath.eq("")),
            );
        }

        Ok(q.all(self.db).await?)
    }
}

pub struct CreateFolderParams<'a> {
    pub tenant_id: Uuid,
    pub name: &'a str,
    pub storage_path: &'a str,
    pub owner_id: Uuid,
    pub department_id: Option<Uuid>,
    pub parent_path: Option<&'a str>,
    pub visibility: &'a str,
    pub is_company_folder: bool,
}

pub struct CreateFileParams<'a> {
    pub tenant_id: Uuid,
    pub name: &'a str,
    pub storage_path: &'a str,
    pub size_bytes: i64,
    pub content_type: &'a str,
    pub owner_id: Uuid,
    pub department_id: Option<Uuid>,
    pub parent_path: Option<&'a str>,
    pub version: i32,
    pub version_parent_id: Option<Uuid>,
    pub visibility: &'a str,
    pub content_hash: &'a str,
    pub ulid: &'a str,
}

pub struct ListFilesFilter {
    pub tenant_id: Uuid,
    pub is_approver: bool,
    pub user_id: Uuid,
    pub view_mode: String,
    pub target_owner_id: Option<Uuid>,
    pub is_admin: bool,
    pub selected_department_id: Option<Uuid>,
    pub user_department_id: Option<Uuid>,
    pub user_allowed_department_ids: Option<Vec<Uuid>>,
    pub parent_path: Option<String>,
}


