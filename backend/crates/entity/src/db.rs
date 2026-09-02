use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

#[derive(Debug, Clone)]
pub struct DatabaseConfig<'a> {
    pub url: &'a str,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub sqlx_logging: bool,
}

pub async fn connect(config: DatabaseConfig<'_>) -> Result<DatabaseConnection, DbErr> {
    let mut options = ConnectOptions::new(config.url);
    options
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(config.acquire_timeout)
        .acquire_timeout(config.acquire_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime)
        .sqlx_logging(config.sqlx_logging);

    Database::connect(options).await
}
