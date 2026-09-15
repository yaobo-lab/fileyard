mod auth;
mod extensions;
mod health;
mod protect;
mod public;
mod uploads;

use auth::build_auth_routes;
use extensions::build_extension_routes;
use health::build_health_routes;
use protect::build_protect_routes;
use public::build_public_routes;
use uploads::build_uploads_routes;

use crate::{
    middleware::{self, ApiUsageState, ApiUsageWriter},
    AppState,
};
use axum::Router;
use app_extensions::routes::ExtensionState;
use std::sync::Arc;
use std::time::Duration;

/// 构建并组装完整的 Axum 应用路由器，同时挂载全局中间件
///
/// # 参数说明
/// - `app_state`: 核心应用共享状态句柄（包含 DB 连接池、Storage 驱动、Redis 缓存等）
/// - `config`: 全局配置对象
/// - `api_usage_writer`: API 调用统计与指标写入器（如果配置开启）
/// - `extension_state`: Clovalink 扩展插件系统上下文状态
///
/// # 路由装配与中间件分层顺序
/// 1. 优先聚合无限制健康检查路由 (`health_routes`)
/// 2. 依次合并登录认证 (`auth_routes`)、公开免登录 (`public_routes`)、受保护业务 (`protect_routes`)、扩展插件 (`extension_routes`) 及静态资源 (`uploads_routes`)
/// 3. 条件挂载 API 请求度量中间件 (`api_usage_middleware`)
/// 4. 挂载全局请求体上限层 (`RequestBodyLimitLayer` 500MB，支持大文件上传)
/// 5. 挂载全局请求超时控制层 (`TimeoutLayer`)
/// 6. 挂载并发请求限制保护层 (`ConcurrencyLimitLayer`)
/// 7. 挂载严格跨域安全策略层 (`CorsLayer`)
pub fn routers(
    app_state: Arc<AppState>,
    config: &types::config::Conf,
    api_usage_writer: Option<Arc<ApiUsageWriter>>,
    extension_state: Arc<ExtensionState>,
) -> Router {
    let health_routes = build_health_routes(&app_state);
    let auth_routes = build_auth_routes(&app_state);
    let public_routes = build_public_routes(&app_state);
    let protect_routes = build_protect_routes(&app_state);
    let extension_routes = build_extension_routes(&app_state, extension_state);
    let uploads_routes = build_uploads_routes(&app_state);

    let mut app = Router::new()
        .merge(health_routes) // 健康检查优先，不受限流影响
        .merge(auth_routes)
        .merge(public_routes)
        .merge(protect_routes)
        .merge(extension_routes)
        .merge(uploads_routes);

    // 如果开启了 API 使用情况追踪，挂载中间件
    if let Some(writer) = api_usage_writer {
        let usage_state = ApiUsageState { writer };
        app = app.layer(axum::middleware::from_fn_with_state(
            usage_state,
            middleware::api_usage::api_usage_middleware,
        ));
    }

    let max_concurrent_requests = config.web.max_concurrent_requests;
    let request_timeout_secs = config.web.request_timeout_secs;
    let cors = configure_cors(&config.cors);

    app
        // 请求体大小限制 (500MB，与 nginx 保持一致)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            500 * 1024 * 1024,
        ))
        // 请求超时
        .layer(tower_http::timeout::TimeoutLayer::new(Duration::from_secs(
            request_timeout_secs,
        )))
        // 并发上限
        .layer(tower::limit::ConcurrencyLimitLayer::new(
            max_concurrent_requests,
        ))
        // CORS 配置
        .layer(cors)
}

/// 配置生产安全级别的跨域资源共享（CORS）中间件
///
/// # 安全机制与行为规则
/// - 严格限制允许的 HTTP 方法（GET, POST, PUT, DELETE, OPTIONS, PATCH）
/// - 严格限制允许的请求头（Authorization, Content-Type, Accept, Origin, x-requested-with, x-tenant-id）
/// - 允许携带凭证凭据（allow_credentials: true）
/// - 针对开发环境（dev_mode = true 或 environment = "development"）：放行 localhost 与 127.0.0.1 常见前端端口以及自定义配置域名
/// - 针对生产环境：必须配置明确的 `allowed_origins` 白名单；若白名单为空则采取 Fail-safe 原则默认拒绝所有跨域请求，防范 CSRF 与越权攻击
/// - 预检请求（Preflight）缓存时长设为 1 小时 (3600s)
fn configure_cors(config: &types::config::CorsConf) -> tower_http::cors::CorsLayer {
    use axum::http::{header, HeaderName, Method};
    use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

    let environment = &config.environment;
    let dev_mode = config.dev_mode;

    // Allowed methods - restrict to actual API methods
    let allowed_methods = AllowMethods::list([
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::PATCH,
    ]);

    // Allowed headers - restrict to necessary ones
    let allowed_headers = AllowHeaders::list([
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::ORIGIN,
        HeaderName::from_static("x-requested-with"),
        HeaderName::from_static("x-tenant-id"),
    ]);

    // Build origin policy
    let allow_origin = if dev_mode || environment == "development" {
        // Development mode: allow localhost origins + any configured origins
        log::warn!("CORS: Development mode enabled - allowing localhost origins");

        let mut origins: Vec<String> = vec![
            "http://localhost:3000".to_string(),
            "http://localhost:5173".to_string(),
            "http://localhost:8080".to_string(),
            "http://127.0.0.1:3000".to_string(),
            "http://127.0.0.1:5173".to_string(),
            "http://127.0.0.1:8080".to_string(),
        ];

        // Add any explicitly configured origins
        for origin in &config.allowed_origins {
            let trimmed = origin.trim().to_string();
            if !trimmed.is_empty() && !origins.contains(&trimmed) {
                origins.push(trimmed);
            }
        }

        log::info!("CORS: Allowed origins: {:?}", origins);

        AllowOrigin::predicate(move |origin, _| {
            if let Ok(origin_str) = origin.to_str() {
                origins.iter().any(|allowed| allowed == origin_str)
            } else {
                false
            }
        })
    } else if !config.allowed_origins.is_empty() {
        // Production mode with explicit allowlist
        let origins: Vec<String> = config
            .allowed_origins
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if origins.is_empty() {
            log::error!("CORS: CORS_ALLOWED_ORIGINS is empty in production mode!");
            // Fail safe - block all cross-origin requests
            AllowOrigin::predicate(|_, _| false)
        } else {
            log::info!(
                "CORS: Production mode with {} allowed origins",
                origins.len()
            );
            AllowOrigin::predicate(move |origin, _| {
                if let Ok(origin_str) = origin.to_str() {
                    origins.iter().any(|allowed| allowed == origin_str)
                } else {
                    false
                }
            })
        }
    } else {
        // Production mode without allowlist - fail safe
        log::error!(
            "CORS: No CORS_ALLOWED_ORIGINS configured in production mode! \
            Set CORS_ALLOWED_ORIGINS or enable CORS_DEV_MODE=true for development."
        );
        // Block all cross-origin requests
        AllowOrigin::predicate(|_, _| false)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods(allowed_methods)
        .allow_headers(allowed_headers)
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600)) // Cache preflight for 1 hour
}
