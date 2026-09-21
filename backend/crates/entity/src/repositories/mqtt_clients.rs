use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, Statement,
};
use uuid::Uuid;

use crate::{
    entities::mqtt_clients::{self, Entity as MqttClientsEntity},
    DataResult,
};

pub struct MqttClientRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> MqttClientRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// 手动创建或添加编译机记录（以 username 为唯一键，只存一条记录）
    pub async fn create_client(
        &self,
        username: &str,
        client_id: &str,
        device_name: Option<&str>,
        ip_address: Option<&str>,
        port: Option<i32>,
        proto_ver: Option<i16>,
        keepalive: Option<i32>,
        online: bool,
    ) -> DataResult<mqtt_clients::Model> {
        let existing = MqttClientsEntity::find()
            .filter(mqtt_clients::Column::Username.eq(username))
            .one(self.db)
            .await?;
        if existing.is_some() {
            return Err(crate::DataError::Conflict);
        }

        let now = chrono::Utc::now().into();
        let model = mqtt_clients::ActiveModel {
            id: Set(Uuid::new_v4()),
            username: Set(username.to_string()),
            client_id: Set(client_id.to_string()),
            device_name: Set(device_name.map(|s| s.to_string()).or_else(|| Some(client_id.to_string()))),
            ip_address: Set(ip_address.map(|s| s.to_string())),
            port: Set(port),
            proto_ver: Set(proto_ver.or(Some(5))),
            keepalive: Set(keepalive.or(Some(60))),
            clean_start: Set(Some(true)),
            online: Set(online),
            connected_at: Set(now),
            disconnected_at: Set(None),
            disconnected_reason: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let inserted = model.insert(self.db).await?;
        Ok(inserted)
    }

    /// 登记或更新 MQTT 客户端连接信息（以 username 为唯一键，只存一条记录）
    pub async fn upsert_connected(
        &self,
        username: &str,
        client_id: &str,
        ip_address: Option<&str>,
        port: Option<i32>,
        proto_ver: Option<i16>,
        keepalive: Option<i32>,
        clean_start: Option<bool>,
    ) -> DataResult<()> {
        let id = Uuid::new_v4();
        let sql = r#"
            INSERT INTO mqtt_clients (
                id, username, client_id, device_name, ip_address, port, proto_ver, keepalive, clean_start,
                online, connected_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
            )
            ON CONFLICT (username) DO UPDATE SET
                client_id = EXCLUDED.client_id,
                ip_address = EXCLUDED.ip_address,
                port = EXCLUDED.port,
                proto_ver = EXCLUDED.proto_ver,
                keepalive = EXCLUDED.keepalive,
                clean_start = EXCLUDED.clean_start,
                online = true,
                connected_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP;
        "#;

        let values = vec![
            id.into(),
            username.to_string().into(),
            client_id.to_string().into(),
            client_id.to_string().into(), // 默认 device_name 为 client_id
            ip_address.map(|s| s.to_string()).into(),
            port.into(),
            proto_ver.into(),
            keepalive.into(),
            clean_start.into(),
        ];

        self.db
            .execute(Statement::from_sql_and_values(
                self.db.get_database_backend(),
                sql,
                values,
            ))
            .await?;

        Ok(())
    }

    /// 标记客户端断开连接
    pub async fn mark_disconnected(
        &self,
        username: Option<&str>,
        client_id: Option<&str>,
        reason: Option<&str>,
    ) -> DataResult<()> {
        if username.is_none() && client_id.is_none() {
            return Ok(());
        }

        let sql = r#"
            UPDATE mqtt_clients SET
                online = false,
                disconnected_at = CURRENT_TIMESTAMP,
                disconnected_reason = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE ($2::TEXT IS NOT NULL AND username = $2)
               OR ($3::TEXT IS NOT NULL AND client_id = $3);
        "#;

        let values = vec![
            reason.map(|r| r.to_string()).into(),
            username.map(|u| u.to_string()).into(),
            client_id.map(|c| c.to_string()).into(),
        ];

        self.db
            .execute(Statement::from_sql_and_values(
                self.db.get_database_backend(),
                sql,
                values,
            ))
            .await?;

        Ok(())
    }

    /// 查询所有 MQTT 客户端记录（按最近连接时间倒序）
    pub async fn list_all(&self) -> DataResult<Vec<mqtt_clients::Model>> {
        let clients = MqttClientsEntity::find()
            .order_by_desc(mqtt_clients::Column::ConnectedAt)
            .all(self.db)
            .await?;

        Ok(clients)
    }

    /// 按状态与搜索关键词查询
    pub async fn search_clients(
        &self,
        search: Option<&str>,
        online_filter: Option<bool>,
    ) -> DataResult<Vec<mqtt_clients::Model>> {
        let mut query = MqttClientsEntity::find();

        if let Some(online) = online_filter {
            query = query.filter(mqtt_clients::Column::Online.eq(online));
        }

        if let Some(s) = search {
            if !s.trim().is_empty() {
                let pattern = format!("%{}%", s.trim());
                query = query.filter(
                    sea_orm::Condition::any()
                        .add(mqtt_clients::Column::Username.like(&pattern))
                        .add(mqtt_clients::Column::ClientId.like(&pattern))
                        .add(mqtt_clients::Column::IpAddress.like(&pattern)),
                );
            }
        }

        let clients = query
            .order_by_desc(mqtt_clients::Column::ConnectedAt)
            .all(self.db)
            .await?;

        Ok(clients)
    }
}
