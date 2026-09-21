pub mod config;
pub mod plugins;
pub mod server;
pub mod version;

pub use server::run_server;

#[allow(unused_imports)]
use anyhow::Result;
pub use plugins::*;
use rmqtt::hook::Type;
#[cfg(feature = "message-storage")]
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub trait IType {
    fn to_string(&self) -> String;
}

impl IType for Type {
    fn to_string(&self) -> String {
        let str = match &self {
            Type::BeforeStartup => "before_startup".to_string(),
            Type::SessionCreated => "session_created".to_string(),
            Type::SessionTerminated => "session_terminated".to_string(),
            Type::SessionSubscribed => "session_subscribed".to_string(),
            Type::SessionUnsubscribed => "session_unsubscribed".to_string(),
            Type::ClientAuthenticate => "client_authenticate".to_string(),
            Type::ClientConnect => "client_connect".to_string(),
            Type::ClientConnack => "client_connack".to_string(),
            Type::ClientConnected => "client_connected".to_string(),
            Type::ClientDisconnected => "client_disconnected".to_string(),
            Type::ClientSubscribe => "client_subscribe".to_string(),
            Type::ClientUnsubscribe => "client_unsubscribe".to_string(),
            Type::ClientSubscribeCheckAcl => "client_subscribe_check_acl".to_string(),
            Type::ClientKeepalive => "client_keepalive".to_string(),
            Type::MessagePublishCheckAcl => "message_publish_check_acl".to_string(),
            Type::MessagePublish => "message_publish".to_string(),
            Type::MessageDelivered => "message_delivered".to_string(),
            Type::MessageAcked => "message_acked".to_string(),
            Type::MessageDropped => "message_dropped".to_string(),
            Type::MessageExpiryCheck => "message_expiry_check".to_string(),
            Type::MessageNonsubscribed => "message_nonsubscribed".to_string(),
            Type::OfflineMessage => "offline_message".to_string(),
            Type::OfflineInflightMessages => "offline_inflight_messages".to_string(),
            Type::GrpcMessageReceived => "grpc_message_received".to_string(),
        };
        str
    }
}

// In memory-> sqlite::memory:
//sqlite://path/to/db.sqlite?mode=ro  read only
#[cfg(feature = "message-storage")]
async fn create_db(conn_str: &str) -> Result<DatabaseConnection> {
    use std::time::Duration;
    //let conn_str = "sqlite://./app.db?mode=rwc";
    let mut opt = ConnectOptions::new(conn_str);
    opt.max_connections(5)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(false)
        .sqlx_logging_level(log::LevelFilter::Error);
    let db = Database::connect(opt).await?;
    Ok(db)
}
