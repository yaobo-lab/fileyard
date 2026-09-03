use crate::DataResult;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use uuid::Uuid;

pub struct CompanySearchRow {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
}
pub struct UserSearchRow {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}
pub struct FileSearchRow {
    pub id: Uuid,
    pub name: String,
    pub parent_path: Option<String>,
    pub tenant_id: Uuid,
    pub is_directory: bool,
}
pub struct GroupSearchRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub parent_path: Option<String>,
    pub tenant_id: Uuid,
    pub department_id: Option<Uuid>,
    pub visibility: String,
    pub owner_id: Option<Uuid>,
    pub is_locked: Option<bool>,
}
pub struct SearchBundle {
    pub companies: Vec<CompanySearchRow>,
    pub users: Vec<UserSearchRow>,
    pub files: Vec<FileSearchRow>,
    pub groups: Vec<GroupSearchRow>,
}
pub struct SearchRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> SearchRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }
    fn stmt(&self, s: &str, v: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), s, v)
    }
    async fn role(&self, t: Uuid, role: &str) -> String {
        if ["SuperAdmin", "Admin", "Manager", "Employee"].contains(&role) {
            return role.into();
        }
        self.db
            .query_one(self.stmt(
                "SELECT base_role FROM roles WHERE tenant_id=$1 AND name=$2",
                vec![t.into(), role.into()],
            ))
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get("", "base_role").ok())
            .unwrap_or_else(|| "Employee".into())
    }

    pub async fn global(
        &self,
        t: Uuid,
        user: Uuid,
        role: &str,
        pattern: &str,
        limit: i64,
    ) -> DataResult<SearchBundle> {
        let role = self.role(t, role).await;
        let super_admin = role == "SuperAdmin";
        let admin = super_admin || role == "Admin";
        let manager = admin || role == "Manager";
        let companies = if super_admin {
            self.db.query_all(self.stmt("SELECT id,name,domain FROM tenants WHERE LOWER(name) LIKE $1 OR LOWER(domain) LIKE $1 ORDER BY name LIMIT $2",vec![pattern.into(),limit.into()])).await?.into_iter().map(|r|Ok(CompanySearchRow{id:r.try_get("","id")?,name:r.try_get("","name")?,domain:r.try_get("","domain")?})).collect::<DataResult<_>>()?
        } else {
            vec![]
        };
        let users = if super_admin {
            self.user_rows("SELECT id,name,email FROM users WHERE LOWER(name) LIKE $1 OR LOWER(email) LIKE $1 ORDER BY name LIMIT $2",vec![pattern.into(),limit.into()]).await?
        } else if admin {
            self.user_rows("SELECT id,name,email FROM users WHERE tenant_id=$1 AND (LOWER(name) LIKE $2 OR LOWER(email) LIKE $2) ORDER BY name LIMIT $3",vec![t.into(),pattern.into(),limit.into()]).await?
        } else {
            vec![]
        };
        let info = self
            .db
            .query_one(self.stmt(
                "SELECT department_id,allowed_department_ids FROM users WHERE id=$1",
                vec![user.into()],
            ))
            .await?;
        let (dept, allowed): (Option<Uuid>, Vec<Uuid>) = info
            .map(|r| {
                Ok((
                    r.try_get("", "department_id")?,
                    r.try_get::<Option<Vec<Uuid>>>("", "allowed_department_ids")?
                        .unwrap_or_default(),
                ))
            })
            .transpose()?
            .unwrap_or((None, vec![]));
        let files = if super_admin {
            self.file_rows("SELECT id,name,parent_path,tenant_id,is_directory FROM files_metadata WHERE LOWER(name) LIKE $1 AND is_deleted=false ORDER BY name LIMIT $2",vec![pattern.into(),limit.into()]).await?
        } else if admin {
            self.file_rows("SELECT id,name,parent_path,tenant_id,is_directory FROM files_metadata WHERE tenant_id=$1 AND LOWER(name) LIKE $2 AND is_deleted=false ORDER BY name LIMIT $3",vec![t.into(),pattern.into(),limit.into()]).await?
        } else {
            self.file_rows("SELECT f.id,f.name,f.parent_path,f.tenant_id,f.is_directory FROM files_metadata f WHERE f.tenant_id=$1 AND LOWER(f.name) LIKE $2 AND f.is_deleted=false AND (f.department_id IS NULL OR f.department_id=$3 OR f.department_id=ANY($4) OR f.owner_id=$5) AND (f.visibility!='private' OR f.owner_id=$5) AND (f.is_locked=false OR f.locked_by=$5 OR f.owner_id=$5 OR $6=true) ORDER BY f.name LIMIT $7",vec![t.into(),pattern.into(),dept.into(),allowed.into(),user.into(),manager.into(),limit.into()]).await?
        };
        let groups = if super_admin {
            self.group_rows("SELECT id,name,description,parent_path,tenant_id,department_id,visibility,owner_id,is_locked FROM file_groups WHERE LOWER(name) LIKE $1 ORDER BY name LIMIT $2",vec![pattern.into(),limit.into()]).await?
        } else if admin {
            self.group_rows("SELECT id,name,description,parent_path,tenant_id,department_id,visibility,owner_id,is_locked FROM file_groups WHERE tenant_id=$1 AND LOWER(name) LIKE $2 ORDER BY name LIMIT $3",vec![t.into(),pattern.into(),limit.into()]).await?
        } else {
            self.group_rows("SELECT g.id,g.name,g.description,g.parent_path,g.tenant_id,g.department_id,g.visibility,g.owner_id,g.is_locked FROM file_groups g WHERE g.tenant_id=$1 AND LOWER(g.name) LIKE $2 AND (g.department_id=$3 OR g.department_id=ANY($4) OR g.owner_id=$5 OR EXISTS(SELECT 1 FROM files_metadata fm WHERE fm.tenant_id=g.tenant_id AND fm.is_directory=true AND fm.is_deleted=false AND COALESCE(fm.is_company_folder,false)=true AND (g.parent_path=fm.name OR g.parent_path LIKE fm.name||'/%' OR (fm.parent_path IS NOT NULL AND g.parent_path LIKE fm.parent_path||'/'||fm.name||'%')))) AND (g.visibility!='private' OR g.owner_id=$5) AND (COALESCE(g.is_locked,false)=false OR g.locked_by=$5 OR g.owner_id=$5 OR $6=true) ORDER BY g.name LIMIT $7",vec![t.into(),pattern.into(),dept.into(),allowed.into(),user.into(),manager.into(),limit.into()]).await?
        };
        Ok(SearchBundle {
            companies,
            users,
            files,
            groups,
        })
    }
    async fn user_rows(&self, s: &str, v: Vec<sea_orm::Value>) -> DataResult<Vec<UserSearchRow>> {
        self.db
            .query_all(self.stmt(s, v))
            .await?
            .into_iter()
            .map(|r| {
                Ok(UserSearchRow {
                    id: r.try_get("", "id")?,
                    name: r.try_get("", "name")?,
                    email: r.try_get("", "email")?,
                })
            })
            .collect()
    }
    async fn file_rows(&self, s: &str, v: Vec<sea_orm::Value>) -> DataResult<Vec<FileSearchRow>> {
        self.db
            .query_all(self.stmt(s, v))
            .await?
            .into_iter()
            .map(|r| {
                Ok(FileSearchRow {
                    id: r.try_get("", "id")?,
                    name: r.try_get("", "name")?,
                    parent_path: r.try_get("", "parent_path")?,
                    tenant_id: r.try_get("", "tenant_id")?,
                    is_directory: r.try_get("", "is_directory")?,
                })
            })
            .collect()
    }
    async fn group_rows(&self, s: &str, v: Vec<sea_orm::Value>) -> DataResult<Vec<GroupSearchRow>> {
        self.db
            .query_all(self.stmt(s, v))
            .await?
            .into_iter()
            .map(|r| {
                Ok(GroupSearchRow {
                    id: r.try_get("", "id")?,
                    name: r.try_get("", "name")?,
                    description: r.try_get("", "description")?,
                    parent_path: r.try_get("", "parent_path")?,
                    tenant_id: r.try_get("", "tenant_id")?,
                    department_id: r.try_get("", "department_id")?,
                    visibility: r.try_get("", "visibility")?,
                    owner_id: r.try_get("", "owner_id")?,
                    is_locked: r.try_get("", "is_locked")?,
                })
            })
            .collect()
    }
}
