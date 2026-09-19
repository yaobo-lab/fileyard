use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "app")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub number: String,
    pub name: String,
    #[sea_orm(column_name = "desc")]
    pub desc: String,
    pub class_no: String,
    pub class_name: String,
    pub create_time: DateTimeWithTimeZone,
    pub doc_path: String,
    pub status: i16,
    pub gitlab_id: String,
    pub git_url: String,
    pub is_del: i16,
    pub createby_name: String,
    pub createby_id: String,
    pub lastupdate_time: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
