use crate::{api::health, AppState};
use axum::{routing::get, Router};
use std::sync::Arc;

/// 构建系统健康检查与就绪检查路由
///
/// 包含以下端点（无速率限制，供负载均衡器与容器探针实时探测）：
/// - `GET /health`: 存活探针（liveness probe），快速检查服务主进程是否存活
/// - `GET /health/ready`: 就绪探针（readiness probe），检查数据库连接、Redis 缓存等核心组件是否就绪
pub(super) fn build_health_routes(app_state: &Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        .with_state(app_state.clone())
}
