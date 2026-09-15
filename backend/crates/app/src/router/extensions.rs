use crate::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use app_extensions::routes::ExtensionState;
use std::sync::Arc;

/// 构建 Clovalink 扩展插件系统路由
///
/// 挂载插件生命周期管理与自动化任务端点，并统一注入 `ExtensionState` 与认证鉴权中间件：
/// - `POST   /api/extensions/register`: 开发者注册新插件
/// - `POST   /api/extensions/install/{extension_id}`: 当前租户安装指定插件
/// - `GET    /api/extensions/list`: 获取市场上所有可用插件列表
/// - `GET    /api/extensions/installed`: 获取当前租户已安装插件列表
/// - `POST   /api/extensions/validate-manifest`: 校验插件清单 (Manifest) 合法性
/// - `GET    /api/extensions/ui`: 获取已加载插件注册的 UI 扩展插槽与组件定义
/// - `POST   /api/extensions/trigger/automation/{job_id}`: 手动触发插件自动化任务
/// - `PUT    /api/extensions/{id}/settings`: 更新指定插件的配置参数
/// - `PUT    /api/extensions/{id}/access`: 配置插件访问授权策略
/// - `DELETE /api/extensions/{id}`: 卸载插件
/// - `GET/POST /api/extensions/{extension_id}/jobs`: 获取或创建插件关联的任务作业
/// - `GET    /api/extensions/{extension_id}/logs`: 获取插件 Webhook 执行历史与日志
pub(super) fn build_extension_routes(
    app_state: &Arc<AppState>,
    extension_state: Arc<ExtensionState>,
) -> Router {
    Router::new()
        .route(
            "/api/extensions/register",
            post(app_extensions::routes::register_extension),
        )
        .route(
            "/api/extensions/install/{extension_id}",
            post(app_extensions::routes::install_extension),
        )
        .route(
            "/api/extensions/list",
            get(app_extensions::routes::list_extensions),
        )
        .route(
            "/api/extensions/installed",
            get(app_extensions::routes::list_installed_extensions),
        )
        .route(
            "/api/extensions/validate-manifest",
            post(app_extensions::routes::validate_manifest),
        )
        .route(
            "/api/extensions/ui",
            get(app_extensions::routes::get_ui_extensions),
        )
        .route(
            "/api/extensions/trigger/automation/{job_id}",
            post(app_extensions::routes::trigger_automation),
        )
        .route(
            "/api/extensions/{id}/settings",
            put(app_extensions::routes::update_extension_settings),
        )
        .route(
            "/api/extensions/{id}/access",
            put(app_extensions::routes::update_extension_access),
        )
        .route(
            "/api/extensions/{id}",
            delete(app_extensions::routes::uninstall_extension),
        )
        .route(
            "/api/extensions/{extension_id}/jobs",
            get(app_extensions::routes::list_jobs)
                .post(app_extensions::routes::create_job),
        )
        .route(
            "/api/extensions/{extension_id}/logs",
            get(app_extensions::routes::get_webhook_logs),
        )
        // 挂载数据库级权限认证中间件，验证用户登录与账号状态
        .layer(axum::middleware::from_fn_with_state(
            crate::auth::middleware::AuthDatabaseState {
                store: app_state.store.clone(),
            },
            crate::auth::middleware::auth_middleware_with_db,
        ))
        .with_state(extension_state)
}
