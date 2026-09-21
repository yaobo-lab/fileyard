use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "mqtt_clients")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub username: String,
    pub client_id: String,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
    pub proto_ver: Option<i16>,
    pub keepalive: Option<i32>,
    pub clean_start: Option<bool>,
    pub online: bool,
    pub connected_at: DateTimeWithTimeZone,
    pub disconnected_at: Option<DateTimeWithTimeZone>,
    pub disconnected_reason: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
