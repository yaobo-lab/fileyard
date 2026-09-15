use crate::{
    api::{auth, oidc, saml, wecom},
    middleware, AppState,
};
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// 构建用户认证与单点登录（SSO）相关路由
///
/// 本模块将公开身份验证端点按安全级别区分为两组并分别挂载不同的限流中间件：
///
/// 1. **核心登录与注册路由**（挂载严格限流 `rate_limit_login`，默认每个 IP 每分钟 5 次）：
///    - `POST /api/auth/login`: 用户登录，生成认证凭证
///    - `POST /api/auth/register`: 新用户注册（若开放注册）
///
/// 2. **密码管理与 SSO 公开回调路由**（挂载普通限流 `rate_limit_public`，默认每个 IP 每分钟 60 次）：
///    - `POST /api/auth/forgot-password`: 申请找回密码，发送重置邮件
///    - `POST /api/auth/reset-password`: 使用令牌重置密码
///    - `GET  /api/auth/password-policy`: 查询当前租户的密码复杂度策略
///    - `GET  /api/auth/oidc/providers`: 发现公开可用的 OIDC 身份提供商列表
///    - `GET  /api/auth/oidc/authorize/{provider_id}`: 发起 OIDC 授权重定向
///    - `GET  /api/auth/oidc/callback`: OIDC 认证回调处理
///    - `GET  /api/auth/saml/metadata/{provider_id}`: 获取本系统作为服务提供商（SP）的 SAML 元数据 XML
///    - `GET  /api/auth/saml/authorize/{provider_id}`: 发起 SAML SSO 请求
///    - `POST /api/auth/saml/acs`: SAML 断言消费服务（ACS）端点，接收 IdP 的 SAML 响应
pub(super) fn build_auth_routes(app_state: &Arc<AppState>) -> Router {
    // 登录与注册路由：严格防爆破限流 (5次/分钟/IP)
    let login_routes = Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::rate_limit::rate_limit_login,
        ))
        .with_state(app_state.clone());

    // 密码策略及公开 SSO 授权/回调路由：常规防滥用限流 (60次/分钟/IP)
    let auth_public_routes = Router::new()
        .route("/api/auth/forgot-password", post(auth::forgot_password))
        .route("/api/auth/reset-password", post(auth::reset_password))
        .route("/api/auth/password-policy", get(auth::get_password_policy))
        // OIDC SSO 公开授权与回调端点
        .route("/api/auth/oidc/providers", get(oidc::discover_providers))
        .route(
            "/api/auth/oidc/authorize/{provider_id}",
            get(oidc::start_oidc_auth),
        )
        .route("/api/auth/oidc/callback", get(oidc::oidc_callback))
        // SAML SSO 公开元数据与回调端点
        .route(
            "/api/auth/saml/metadata/{provider_id}",
            get(saml::sp_metadata),
        )
        .route(
            "/api/auth/saml/authorize/{provider_id}",
            get(saml::start_saml_auth),
        )
        .route("/api/auth/saml/acs", post(saml::saml_acs))
        // 企业微信 SSO 公开端点
        .route("/api/auth/wecom/config", get(wecom::get_wecom_config))
        .route("/api/auth/wecom/authorize", get(wecom::start_wecom_auth))
        .route("/api/auth/wecom/callback", get(wecom::wecom_callback))
        .route("/api/auth/wecom/demo-login", post(wecom::demo_login_endpoint))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            middleware::rate_limit::rate_limit_public,
        ))
        .with_state(app_state.clone());

    login_routes.merge(auth_public_routes)
}
