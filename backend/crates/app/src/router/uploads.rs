use crate::{api::handlers, AppState};
use axum::{routing::get, Router};
use std::sync::Arc;

/// 构建公开静态上传资源服务路由
///
/// 适用于用户头像、公开附件、Logo、Favicon 等静态资源的托管访问：
/// - `GET /uploads/{*path}`: 从底层存储（本地存储或 S3 对象存储）安全读取并回传静态资源文件流
pub(super) fn build_uploads_routes(app_state: &Arc<AppState>) -> Router {
    Router::new()
        .route("/uploads/{*path}", get(handlers::serve_upload))
        .with_state(app_state.clone())
}
