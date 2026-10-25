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
use app_extensions::routes::ExtensionState;
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Router,
};
use std::sync::Arc;
use std::time::Duration;

/// 构建并组装完整的 Axum 固件路由器，同时挂载全局中间件
///
/// # 参数说明
/// - `app_state`: 核心固件共享状态句柄（包含 DB 连接池、Storage 驱动、Redis 缓存等）
/// - `config`: 全局配置对象
/// - `api_usage_writer`: API 调用统计与指标写入器（如果配置开启）
/// - `extension_state`: Clovalink 扩展插件系统上下文状态
///
/// # 路由装配与中间件分层顺序
/// 1. 优先聚合无限制健康检查路由 (`health_routes`)
/// 2. 依次合并登录认证 (`auth_routes`)、公开免登录 (`public_routes`)、受保护业务 (`protect_routes`)、扩展插件 (`extension_routes`) 及静态资源 (`uploads_routes`)
/// 3. 条件挂载 API 请求度量中间件 (`api_usage_middleware`)
/// 4. 挂载全局请求体上限层 (`RequestBodyLimitLayer` 500MB，支持大文件上传)
/// 5. 挂载按请求类型区分的超时中间件（文件上传使用独立长超时）
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
    let request_timeouts = RequestTimeouts {
        default: Duration::from_secs(config.web.request_timeout_secs),
        upload: Duration::from_secs(config.web.upload_timeout_secs),
    };
    log::info!(
        "Request timeouts configured: default={}s, multipart_upload={}s",
        request_timeouts.default.as_secs(),
        request_timeouts.upload.as_secs()
    );
    let cors = configure_cors(&config.web.allowed_origins);

    app
        // 请求体大小限制 (500MB，与 nginx 保持一致)
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            500 * 1024 * 1024,
        ))
        // multipart 上传使用更长超时，普通 API 保持较短的全局超时
        .layer(axum::middleware::from_fn_with_state(
            request_timeouts,
            request_timeout_middleware,
        ))
        // 并发上限
        .layer(tower::limit::ConcurrencyLimitLayer::new(
            max_concurrent_requests,
        ))
        // CORS 配置
        .layer(cors)
}

#[derive(Clone, Copy)]
struct RequestTimeouts {
    default: Duration,
    upload: Duration,
}

async fn request_timeout_middleware(
    State(timeouts): State<RequestTimeouts>,
    request: Request,
    next: Next,
) -> Response {
    let is_multipart_upload = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .to_ascii_lowercase()
                .starts_with("multipart/form-data")
        });
    let timeout = if is_multipart_upload {
        timeouts.upload
    } else {
        timeouts.default
    };
    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    match tokio::time::timeout(timeout, next.run(request)).await {
        Ok(response) => response,
        Err(_) => {
            log::warn!(
                "Request timed out: method={}, path={}, timeout={}s, multipart={}",
                method,
                path,
                timeout.as_secs(),
                is_multipart_upload
            );
            StatusCode::REQUEST_TIMEOUT.into_response()
        }
    }
}

/// 配置跨域资源共享（CORS）中间件
///
/// # 安全机制与行为规则
/// - 严格限制允许的 HTTP 方法（GET, POST, PUT, DELETE, OPTIONS, PATCH）
/// - 严格限制允许的请求头（Authorization, Content-Type, Accept, Origin, x-requested-with, x-tenant-id）
/// - 允许携带凭证凭据（allow_credentials: true）
/// - 根据 `[web].allowed_origins` 白名单进行来源匹配
/// - 预检请求（Preflight）缓存时长设为 1 小时 (3600s)
fn configure_cors(allowed_origins: &[String]) -> tower_http::cors::CorsLayer {
    use axum::http::{header, HeaderName, Method};
    use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

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

    let origins: Vec<String> = allowed_origins
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let allow_all = origins.iter().any(|allowed| allowed == "*");

    let allow_origin = if allow_all {
        log::info!("CORS: Allowed origins: wildcard [*] enabled (allowing all origins)");
        AllowOrigin::mirror_request()
    } else if !origins.is_empty() {
        log::info!("CORS: Allowed origins: {:?}", origins);
        AllowOrigin::predicate(move |origin, _| {
            if let Ok(origin_str) = origin.to_str() {
                origins.iter().any(|allowed| allowed == origin_str)
            } else {
                false
            }
        })
    } else {
        log::warn!("CORS: No allowed_origins configured in [web]! Blocking cross-origin requests.");
        AllowOrigin::predicate(|_, _| false)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods(allowed_methods)
        .allow_headers(allowed_headers)
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600)) // Cache preflight for 1 hour
}
