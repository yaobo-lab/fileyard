use crate::{
    api::{file_requests, handlers, health, root},
    middleware, AppState,
};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// 构建公共免认证路由
///
/// 包含外部访客或公开场景下无需登录即可访问的端点，全部挂载公网限流中间件 `rate_limit_public`：
/// - `GET  /`: 服务根路径欢迎与版本信息
/// - `GET  /api/version`: 获取当前服务运行版本与构建信息
/// - `POST /api/public-upload/{token}`: 外链收集文件上传（由外部用户向特定文件收集请求上传文件）
/// - `GET  /api/share/{token}`: 通过外链分享令牌直接下载共享文件
/// - `GET  /api/share/{token}/info`: 获取外链共享文件的元数据信息（文件名、大小、是否需要提取码等）
pub(super) fn build_public_routes(app_state: &Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/api/version", get(health::get_current_version))
        .route(
            "/api/public-upload/{token}",
            post(file_requests::public_upload),
        )
        // 外链共享公开访问与下载端点
        .route("/api/share/{token}", get(handlers::download_shared_file))
        .route("/api/share/{token}/info", get(handlers::get_share_info))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::rate_limit::rate_limit_public,
        ))
        .with_state(app_state.clone())
}
