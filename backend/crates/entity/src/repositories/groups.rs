use crate::{
    entities::{file_groups, files_metadata},
    DataResult,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder, Statement,
};
use uuid::Uuid;
#[derive(Debug, FromQueryResult)]
pub struct GroupListRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub department_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub parent_path: Option<String>,
    pub visibility: String,
    pub owner_id: Option<Uuid>,
    pub is_locked: Option<bool>,
    pub locked_by: Option<Uuid>,
    pub locked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub lock_requires_role: Option<String>,
    pub file_count: i64,
    pub total_size: i64,
    pub owner_name: Option<String>,
}
pub struct NewGroup {
    pub tenant_id: Uuid,
    pub department_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: String,
    pub user_id: Uuid,
    pub visibility: String,
}
pub struct GroupPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}
pub struct GroupRepository<'a> {
    db: &'a DatabaseConnection,
}
impl<'a> GroupRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }
    fn stmt(&self, s: &str, v: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), s, v)
    }
    pub async fn in_company_folder(&self, t: Uuid, g: Uuid) -> DataResult<bool> {
        let r=self.db.query_one(self.stmt("SELECT EXISTS(SELECT 1 FROM file_groups g JOIN files_metadata fm ON fm.tenant_id=g.tenant_id AND fm.is_directory=true AND fm.is_deleted=false AND COALESCE(fm.is_company_folder,false)=true AND (g.parent_path=fm.name OR g.parent_path LIKE fm.name||'/%' OR (fm.parent_path IS NOT NULL AND g.parent_path LIKE fm.parent_path||'/'||fm.name||'%')) WHERE g.id=$1 AND g.tenant_id=$2) AS found",vec![g.into(),t.into()])).await?;
        Ok(r.map(|x| x.try_get("", "found"))
            .transpose()?
            .unwrap_or(false))
    }
    pub async fn list(
        &self,
        t: Uuid,
        dept: Option<Uuid>,
        path: Option<&str>,
        vis: Option<&str>,
        user: Uuid,
        role: &str,
        user_dept: Option<Uuid>,
        allowed: &[Uuid],
    ) -> DataResult<Vec<GroupListRow>> {
        Ok(GroupListRow::find_by_statement(self.stmt("SELECT g.id,g.tenant_id,g.department_id,g.name,g.description,g.color,g.icon,g.created_by,g.created_at,g.updated_at,g.parent_path,g.visibility,g.owner_id,g.is_locked,g.locked_by,g.locked_at,g.lock_requires_role,COALESCE(COUNT(f.id),0)::bigint file_count,COALESCE(SUM(f.size_bytes),0)::bigint total_size,u.name owner_name FROM file_groups g LEFT JOIN files_metadata f ON f.group_id=g.id AND f.is_deleted=false LEFT JOIN users u ON u.id=g.created_by WHERE g.tenant_id=$1 AND ($3::text IS NULL OR ($3='' AND (g.parent_path IS NULL OR g.parent_path='')) OR g.parent_path=$3) AND ($4::text IS NULL OR g.visibility=$4) AND (CASE WHEN $6 IN ('SuperAdmin','Admin') THEN ($2::uuid IS NULL OR g.department_id IS NULL OR g.department_id=$2) ELSE ((g.department_id IS NOT NULL AND (g.department_id=$7 OR g.department_id=ANY($8))) OR EXISTS(SELECT 1 FROM files_metadata fm WHERE fm.tenant_id=g.tenant_id AND fm.is_directory=true AND fm.is_deleted=false AND COALESCE(fm.is_company_folder,false)=true AND (g.parent_path=fm.name OR g.parent_path LIKE fm.name||'/%' OR (fm.parent_path IS NOT NULL AND g.parent_path LIKE fm.parent_path||'/'||fm.name||'%')))) END) AND (g.visibility='department' OR (g.visibility='private' AND g.owner_id=$5) OR $6 IN ('SuperAdmin','Admin')) GROUP BY g.id,u.name ORDER BY g.name",vec![t.into(),dept.into(),path.into(),vis.into(),user.into(),role.into(),user_dept.into(),allowed.to_vec().into()])).all(self.db).await?)
    }
    pub async fn get(&self, t: Uuid, id: Uuid) -> DataResult<Option<file_groups::Model>> {
        Ok(file_groups::Entity::find_by_id(id)
            .filter(file_groups::Column::TenantId.eq(t))
            .one(self.db)
            .await?)
    }
    pub async fn create(&self, n: NewGroup) -> DataResult<file_groups::Model> {
        let now = chrono::Utc::now().into();
        Ok(file_groups::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(n.tenant_id),
            department_id: Set(n.department_id),
            name: Set(n.name),
            description: Set(n.description),
            color: Set(n.color),
            icon: Set(Some(n.icon)),
            created_by: Set(n.user_id),
            parent_path: Set(None),
            visibility: Set(n.visibility),
            owner_id: Set(Some(n.user_id)),
            is_locked: Set(Some(false)),
            locked_by: Set(None),
            locked_at: Set(None),
            lock_password_hash: Set(None),
            lock_requires_role: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(self.db)
        .await?)
    }
    pub async fn update(
        &self,
        t: Uuid,
        id: Uuid,
        p: GroupPatch,
    ) -> DataResult<Option<file_groups::Model>> {
        let Some(r) = self.get(t, id).await? else {
            return Ok(None);
        };
        let mut a: file_groups::ActiveModel = r.into();
        if let Some(v) = p.name {
            a.name = Set(v)
        }
        if let Some(v) = p.description {
            a.description = Set(Some(v))
        }
        if let Some(v) = p.color {
            a.color = Set(Some(v))
        }
        if let Some(v) = p.icon {
            a.icon = Set(Some(v))
        }
        a.updated_at = Set(chrono::Utc::now().into());
        Ok(Some(a.update(self.db).await?))
    }
    pub async fn delete(&self, t: Uuid, id: Uuid) -> DataResult<bool> {
        Ok(file_groups::Entity::delete_many()
            .filter(file_groups::Column::Id.eq(id))
            .filter(file_groups::Column::TenantId.eq(t))
            .exec(self.db)
            .await?
            .rows_affected
            > 0)
    }
    pub async fn active_file(
        &self,
        t: Uuid,
        id: Uuid,
    ) -> DataResult<Option<files_metadata::Model>> {
        Ok(files_metadata::Entity::find_by_id(id)
            .filter(files_metadata::Column::TenantId.eq(t))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .one(self.db)
            .await?)
    }
    pub async fn file_count(&self, t: Uuid, g: Uuid) -> DataResult<u64> {
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(t))
            .filter(files_metadata::Column::GroupId.eq(g))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .count(self.db)
            .await?)
    }
    pub async fn set_file_group(&self, t: Uuid, id: Uuid, g: Option<Uuid>) -> DataResult<bool> {
        let Some(r) = self.active_file(t, id).await? else {
            return Ok(false);
        };
        let mut a: files_metadata::ActiveModel = r.into();
        a.group_id = Set(g);
        a.update(self.db).await?;
        Ok(true)
    }
    pub async fn duplicate_ungrouped(
        &self,
        t: Uuid,
        id: Uuid,
        name: &str,
        path: Option<&str>,
        vis: &str,
    ) -> DataResult<bool> {
        let r=self.db.query_one(self.stmt("SELECT id FROM files_metadata WHERE tenant_id=$1 AND name=$2 AND parent_path IS NOT DISTINCT FROM $3 AND visibility=$4 AND is_deleted=false AND group_id IS NULL AND id!=$5",vec![t.into(),name.into(),path.into(),vis.into(),id.into()])).await?;
        Ok(r.is_some())
    }
    pub async fn files(&self, t: Uuid, g: Uuid) -> DataResult<Vec<files_metadata::Model>> {
        Ok(files_metadata::Entity::find()
            .filter(files_metadata::Column::TenantId.eq(t))
            .filter(files_metadata::Column::GroupId.eq(g))
            .filter(files_metadata::Column::IsDeleted.eq(false))
            .filter(files_metadata::Column::IsDirectory.eq(false))
            .order_by_asc(files_metadata::Column::Name)
            .all(self.db)
            .await?)
    }
    pub async fn folder(&self, t: Uuid, id: Uuid) -> DataResult<Option<(String, Option<String>)>> {
        Ok(files_metadata::Entity::find_by_id(id)
            .filter(files_metadata::Column::TenantId.eq(t))
            .filter(files_metadata::Column::IsDirectory.eq(true))
            .one(self.db)
            .await?
            .map(|f| (f.name, f.parent_path)))
    }
    pub async fn move_group(&self, t: Uuid, id: Uuid, path: Option<String>) -> DataResult<()> {
        let Some(r) = self.get(t, id).await? else {
            return Ok(());
        };
        let mut a: file_groups::ActiveModel = r.into();
        a.parent_path = Set(path);
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn custom_permission(
        &self,
        t: Uuid,
        role: &str,
        permission: &str,
    ) -> DataResult<bool> {
        let Some(r) = crate::entities::roles::Entity::find()
            .filter(crate::entities::roles::Column::TenantId.eq(t))
            .filter(crate::entities::roles::Column::Name.eq(role))
            .one(self.db)
            .await?
        else {
            return Ok(false);
        };
        Ok(crate::entities::role_permissions::Entity::find()
            .filter(crate::entities::role_permissions::Column::RoleId.eq(r.id))
            .filter(crate::entities::role_permissions::Column::Permission.eq(permission))
            .filter(crate::entities::role_permissions::Column::Granted.eq(true))
            .one(self.db)
            .await?
            .is_some())
    }
    pub async fn lock(
        &self,
        t: Uuid,
        id: Uuid,
        user: Uuid,
        hash: Option<String>,
        role: Option<String>,
    ) -> DataResult<()> {
        let Some(r) = self.get(t, id).await? else {
            return Ok(());
        };
        let mut a: file_groups::ActiveModel = r.into();
        a.is_locked = Set(Some(true));
        a.locked_by = Set(Some(user));
        a.locked_at = Set(Some(chrono::Utc::now().into()));
        a.lock_password_hash = Set(hash);
        a.lock_requires_role = Set(role);
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn unlock(&self, t: Uuid, id: Uuid) -> DataResult<()> {
        let Some(r) = self.get(t, id).await? else {
            return Ok(());
        };
        let mut a: file_groups::ActiveModel = r.into();
        a.is_locked = Set(Some(false));
        a.locked_by = Set(None);
        a.locked_at = Set(None);
        a.lock_password_hash = Set(None);
        a.lock_requires_role = Set(None);
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
}
