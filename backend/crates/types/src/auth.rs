use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 认证用户上下文数据结构，挂载于 HTTP 请求的 Extensions 中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role: String,
    pub email: String,
    pub ip_address: Option<String>,
}
