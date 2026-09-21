use std::sync::Arc;
use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    response::Response,
    Extension, Json,
};
use crate::app::AppState;
use crate::auth::{require_admin, AuthUser};
use mqttd::plugins::restapi::EmbeddedApi;

#[derive(serde::Deserialize, Default)]
pub struct ListClientsQuery {
    pub search: Option<String>,
    pub online: Option<bool>,
}

/// 查询在数据库中登记的 MQTT 客户端记录（每个 MQTT 账号仅一条记录）
pub async fn list_database_clients(
    State(app_state): State<Arc<AppState>>,
    Extension(_auth): Extension<AuthUser>,
    Query(query): Query<ListClientsQuery>,
) -> Result<Json<Vec<app_entity::mqtt_clients::Model>>, StatusCode> {
    let clients = app_state
        .store
        .mqtt_clients()
        .search_clients(query.search.as_deref(), query.online)
        .await
        .map_err(|e| {
            log::error!("查询 MQTT 客户端登记表失败: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(clients))
}

#[derive(serde::Deserialize)]
pub struct CreateClientRequest {
    pub username: String,
    pub client_id: String,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
    pub proto_ver: Option<i16>,
    pub keepalive: Option<i32>,
    pub online: Option<bool>,
}

/// 手动创建或登记编译机记录（每个 MQTT 账号仅一条记录）
pub async fn create_database_client(
    State(app_state): State<Arc<AppState>>,
    Extension(_auth): Extension<AuthUser>,
    Json(payload): Json<CreateClientRequest>,
) -> Result<(StatusCode, Json<app_entity::mqtt_clients::Model>), (StatusCode, String)> {
    let username = payload.username.trim();
    let client_id = payload.client_id.trim();

    if username.is_empty() || client_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "用户名 (username) 和 设备 Client ID 不能为空".to_string(),
        ));
    }

    match app_state
        .store
        .mqtt_clients()
        .create_client(
            username,
            client_id,
            payload.device_name.as_deref().filter(|s| !s.trim().is_empty()),
            payload.ip_address.as_deref().filter(|s| !s.trim().is_empty()),
            payload.port,
            payload.proto_ver,
            payload.keepalive,
            payload.online.unwrap_or(false),
        )
        .await
    {
        Ok(client) => Ok((StatusCode::CREATED, Json(client))),
        Err(app_entity::DataError::Conflict) => Err((
            StatusCode::CONFLICT,
            format!("MQTT 账号 '{username}' 已存在，请勿重复添加"),
        )),
        Err(e) => {
            log::error!("创建编译机记录失败: {e}");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "创建编译机记录失败".to_string(),
            ))
        }
    }
}

pub async fn dispatch(
    Extension(auth): Extension<AuthUser>,
    Extension(api): Extension<EmbeddedApi>,
    request: Request,
) -> Result<Response, StatusCode> {
    // Read operations (GET) are accessible to authenticated users
    // Mutating operations (POST, PUT, DELETE) require Admin or SuperAdmin
    if request.method() != axum::http::Method::GET {
        require_admin(&auth)?;
    }
    Ok(api.handle(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[tokio::test]
    async fn authenticated_users_can_read_broker_api() {
        for role in ["User", "Employee", "Manager", "Admin", "SuperAdmin"] {
            let auth = AuthUser {
                user_id: uuid::Uuid::nil(),
                tenant_id: uuid::Uuid::nil(),
                role: role.into(),
                email: String::new(),
                ip_address: None,
            };
            let mut req = Request::new(Body::empty());
            *req.method_mut() = axum::http::Method::GET;
            let response = dispatch(
                Extension(auth),
                Extension(EmbeddedApi::default()),
                req,
            )
            .await;
            let status = match response {
                Ok(response) => response.status(),
                Err(status) => status,
            };
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    #[tokio::test]
    async fn mutating_broker_api_requires_admin() {
        for (role, expected) in [
            ("User", StatusCode::FORBIDDEN),
            ("Employee", StatusCode::FORBIDDEN),
            ("Admin", StatusCode::SERVICE_UNAVAILABLE),
            ("SuperAdmin", StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let auth = AuthUser {
                user_id: uuid::Uuid::nil(),
                tenant_id: uuid::Uuid::nil(),
                role: role.into(),
                email: String::new(),
                ip_address: None,
            };
            let mut req = Request::new(Body::empty());
            *req.method_mut() = axum::http::Method::DELETE;
            let response = dispatch(
                Extension(auth),
                Extension(EmbeddedApi::default()),
                req,
            )
            .await;
            let status = match response {
                Ok(response) => response.status(),
                Err(status) => status,
            };
            assert_eq!(status, expected);
        }
    }
}
