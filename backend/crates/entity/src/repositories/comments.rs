use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use uuid::Uuid;
use crate::{entities::{file_comments, files_metadata, users}, DataResult};

#[derive(Debug, Clone)] pub struct CommentRow { pub comment:file_comments::Model,pub user_name:String,pub user_avatar:Option<String> }
pub struct CommentRepository<'a>{db:&'a DatabaseConnection}
impl<'a> CommentRepository<'a>{
 pub(crate) fn new(db:&'a DatabaseConnection)->Self{Self{db}}
 pub async fn list(&self,tenant:Uuid,file:Uuid)->DataResult<Vec<CommentRow>>{let rows=file_comments::Entity::find().filter(file_comments::Column::TenantId.eq(tenant)).filter(file_comments::Column::FileId.eq(file)).order_by_asc(file_comments::Column::CreatedAt).all(self.db).await?;let mut out=Vec::with_capacity(rows.len());for comment in rows{let user=users::Entity::find_by_id(comment.user_id).one(self.db).await?;out.push(CommentRow{comment,user_name:user.as_ref().map(|u|u.name.clone()).unwrap_or_else(||"Unknown".into()),user_avatar:user.and_then(|u|u.avatar_url)});}Ok(out)}
 pub async fn exists(&self,id:Uuid,file:Uuid)->DataResult<bool>{Ok(file_comments::Entity::find_by_id(id).filter(file_comments::Column::FileId.eq(file)).count(self.db).await?>0)}
 pub async fn file_info(&self,id:Uuid)->DataResult<Option<(String,Option<Uuid>)>>{Ok(files_metadata::Entity::find_by_id(id).one(self.db).await?.map(|f|(f.name,f.owner_id)))}
 pub async fn create(&self,tenant:Uuid,file:Uuid,user:Uuid,content:&str,parent:Option<Uuid>)->DataResult<Uuid>{let now=chrono::Utc::now().into();let id=Uuid::new_v4();file_comments::ActiveModel{id:Set(id),file_id:Set(file),tenant_id:Set(tenant),user_id:Set(user),content:Set(content.into()),parent_id:Set(parent),is_edited:Set(false),created_at:Set(now),updated_at:Set(now)}.insert(self.db).await?;Ok(id)}
 pub async fn owner(&self,tenant:Uuid,file:Uuid,id:Uuid)->DataResult<Option<Uuid>>{Ok(file_comments::Entity::find_by_id(id).filter(file_comments::Column::TenantId.eq(tenant)).filter(file_comments::Column::FileId.eq(file)).one(self.db).await?.map(|c|c.user_id))}
 pub async fn update(&self,id:Uuid,content:&str)->DataResult<bool>{let Some(row)=file_comments::Entity::find_by_id(id).one(self.db).await? else{return Ok(false)};let mut a:file_comments::ActiveModel=row.into();a.content=Set(content.into());a.is_edited=Set(true);a.updated_at=Set(chrono::Utc::now().into());a.update(self.db).await?;Ok(true)}
 pub async fn delete(&self,id:Uuid)->DataResult<bool>{Ok(file_comments::Entity::delete_by_id(id).exec(self.db).await?.rows_affected>0)}
 pub async fn count(&self,tenant:Uuid,file:Uuid)->DataResult<u64>{Ok(file_comments::Entity::find().filter(file_comments::Column::TenantId.eq(tenant)).filter(file_comments::Column::FileId.eq(file)).count(self.db).await?)}
 pub async fn user_name(&self,id:Uuid)->DataResult<Option<String>>{Ok(users::Entity::find_by_id(id).one(self.db).await?.map(|u|u.name))}
}
