pub mod ai;
pub mod api_usage;
pub mod app_manage;
pub mod approvals;
pub mod audit;
pub mod auth;
pub mod comments;
pub mod compliance;
pub mod config_manage;

pub mod cron;
pub mod dashboard;
pub mod departments;
pub mod discord;
pub mod email_templates;
pub mod extensions;
pub mod file_requests;
pub mod gitlab;
pub mod global_settings;
pub mod groups;
pub mod handlers;
pub mod health;
pub mod mqtt;
pub mod notifications;
pub mod oidc;
pub mod password;
pub mod roles;
pub mod saml;
pub mod saml_crypto;
pub mod saml_xml;
pub mod search;
pub mod security;
pub mod settings;
pub mod settings_backup;
pub mod sharing;
pub mod sso_common;
pub mod sso_mappings;
pub mod tenants;
pub mod text_extract;
pub mod users;
pub mod virus_scan;
pub mod wecom;

// 别名兼容原 auth_handlers 模块命名
pub use auth as auth_handlers;

/// 根路径响应
pub async fn root() -> &'static str {
    "ClovaLink Backend API v2.0 - Multi-Tenant Edition with Extensions"
}
