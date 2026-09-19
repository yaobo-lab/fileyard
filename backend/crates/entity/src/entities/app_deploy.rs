use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "app_deploy")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub number: String,
    pub app_no: String,
    pub name: String,
    pub branch_name: String,
    pub build_tag: String,
    pub auto_pub: i16,
    pub api_uri: String,
    pub api_key_id: i32,
    pub is_del: i16,
    pub create_time: DateTimeWithTimeZone,
    pub envs: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
