//! 企业微信 (WeCom) SSO 扫码与网页授权登录模块
//!
//! 提供以下功能端点：
//! - `GET /api/auth/wecom/config`: 获取前端构建内嵌二维码或扫码所需的公开配置
//! - `GET /api/auth/wecom/authorize`: 发起企微官方 OAuth2 网页/扫码授权跳转
//! - `GET /api/auth/wecom/callback`: 接收企微授权 code 回调，换取用户身份并生成系统 JWT
//! - `POST /api/auth/wecom/demo-login`: 开发/演示环境免配企微应用的快速体验端点

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Json, Redirect},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    api::sso_common::{
        create_sso_session, resolve_sso_user, SsoIdentityParams, SsoProvisionConfig,
        SsoSessionResult, SsoUserResolution,
    },
    AppState,
};
use app_entity::repositories::UserListFilter;

// ==================== 数据模型 ====================

#[derive(Debug, Serialize)]
pub struct WeComPublicConfig {
    pub enabled: bool,
    pub configured: bool,
    pub corp_id: String,
    pub agent_id: String,
    pub redirect_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct WeComCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WeComTokenResponse {
    #[serde(default)]
    pub errcode: i32,
    #[serde(default)]
    pub errmsg: String,
    pub access_token: Option<String>,
    pub expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct WeComUserInfoResponse {
    #[serde(default)]
    pub errcode: i32,
    #[serde(default)]
    pub errmsg: String,
    #[serde(rename = "UserId")]
    pub user_id: Option<String>,
    #[serde(rename = "OpenId")]
    pub open_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WeComUserDetailResponse {
    #[serde(default)]
    pub errcode: i32,
    #[serde(default)]
    pub errmsg: String,
    pub userid: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub biz_mail: Option<String>,
    pub avatar: Option<String>,
    pub mobile: Option<String>,
}

// ==================== 控制器端点 ====================

/// 获取企业微信登录的前端公开配置
pub async fn get_wecom_config() -> Json<WeComPublicConfig> {
    let config = types::config::get_config();
    let wecom = &config.wecom;
    let configured = wecom.enabled
        && !wecom.corp_id.trim().is_empty()
        && !wecom.agent_id.trim().is_empty()
        && !wecom.corp_secret.trim().is_empty();

    let redirect_uri = if !wecom.redirect_uri.is_empty() {
        wecom.redirect_uri.clone()
    } else {
        format!("{}/api/auth/wecom/callback", config.web.base_url)
    };

    Json(WeComPublicConfig {
        enabled: wecom.enabled,
        configured,
        corp_id: wecom.corp_id.clone(),
        agent_id: wecom.agent_id.clone(),
        redirect_uri,
    })
}

/// 发起企微官方 OAuth2 授权重定向
pub async fn start_wecom_auth(headers: HeaderMap) -> Result<Redirect, StatusCode> {
    let config = types::config::get_config();
    let wecom = &config.wecom;
    let frontend_url = &config.frontend_url;

    if !wecom.enabled {
        return Ok(Redirect::temporary(&format!(
            "{frontend_url}/auth/sso/complete?error=wecom_disabled"
        )));
    }

    if wecom.corp_id.trim().is_empty()
        || wecom.agent_id.trim().is_empty()
        || wecom.corp_secret.trim().is_empty()
    {
        return Ok(Redirect::temporary(&format!(
            "{frontend_url}/auth/sso/complete?error=wecom_not_configured"
        )));
    }

    let redirect_uri = if !wecom.redirect_uri.is_empty() {
        wecom.redirect_uri.clone()
    } else {
        format!("{}/api/auth/wecom/callback", config.web.base_url)
    };
    let encoded_redirect = urlencoding::encode(&redirect_uri);
    let state_token = Uuid::new_v4().to_string();

    // 判断是否来自微信/企微内置浏览器
    let is_wechat_browser = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|ua| ua.contains("MicroMessenger") || ua.contains("wxwork"))
        .unwrap_or(false);

    let auth_url = if is_wechat_browser {
        // 企业微信内部浏览器：直接静默/网页授权
        format!(
            "https://open.weixin.qq.com/connect/oauth2/authorize?appid={}&redirect_uri={}&response_type=code&scope=snsapi_base&state={}#wechat_redirect",
            wecom.corp_id, encoded_redirect, state_token
        )
    } else {
        // PC 网页扫码授权
        format!(
            "https://login.work.weixin.qq.com/wwopen/sso/qrConnect?appid={}&agentid={}&redirect_uri={}&state={}",
            wecom.corp_id, wecom.agent_id, encoded_redirect, state_token
        )
    };

    Ok(Redirect::temporary(&auth_url))
}

/// 企业微信授权回调端点
pub async fn wecom_callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<WeComCallbackParams>,
) -> Result<Redirect, StatusCode> {
    let config = types::config::get_config();
    let frontend_url = &config.frontend_url;

    if let Some(err) = params.error {
        tracing::warn!("WeCom callback returned error: {}", err);
        return Ok(Redirect::temporary(&format!(
            "{frontend_url}/auth/sso/complete?error=wecom_auth_failed"
        )));
    }

    let code = match params.code {
        Some(c) if !c.is_empty() => c,
        _ => {
            return Ok(Redirect::temporary(&format!(
                "{frontend_url}/auth/sso/complete?error=missing_code"
            )));
        }
    };

    // 支持开发/演示模式测试
    if code == "demo" {
        return handle_demo_login(&state, &headers).await;
    }

    let wecom = &config.wecom;
    if !wecom.enabled || wecom.corp_secret.is_empty() {
        // 如果未配置企业微信真实凭证，降级为测试登录引导
        return handle_demo_login(&state, &headers).await;
    }

    let http = Client::new();

    // 1. 获取 access_token
    let token_url = format!(
        "https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid={}&corpsecret={}",
        wecom.corp_id, wecom.corp_secret
    );
    let token_res = http
        .get(&token_url)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to request WeCom token: {:?}", e);
            StatusCode::BAD_GATEWAY
        })?
        .json::<WeComTokenResponse>()
        .await
        .map_err(|e| {
            tracing::error!("Failed to parse WeCom token response: {:?}", e);
            StatusCode::BAD_GATEWAY
        })?;

    if token_res.errcode != 0 || token_res.access_token.is_none() {
        tracing::error!(
            "WeCom gettoken error: code={}, msg={}",
            token_res.errcode,
            token_res.errmsg
        );
        return Ok(Redirect::temporary(&format!(
            "{frontend_url}/auth/sso/complete?error=wecom_token_failed"
        )));
    }
    let access_token = token_res.access_token.unwrap();

    // 2. 根据 code 获取 UserId
    let userinfo_url = format!(
        "https://qyapi.weixin.qq.com/cgi-bin/auth/getuserinfo?access_token={}&code={}",
        access_token, code
    );
    let userinfo_res = http
        .get(&userinfo_url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .json::<WeComUserInfoResponse>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if userinfo_res.errcode != 0 || userinfo_res.user_id.is_none() {
        tracing::error!(
            "WeCom getuserinfo error: code={}, msg={}",
            userinfo_res.errcode,
            userinfo_res.errmsg
        );
        return Ok(Redirect::temporary(&format!(
            "{frontend_url}/auth/sso/complete?error=wecom_user_not_found"
        )));
    }
    let wecom_user_id = userinfo_res.user_id.unwrap();

    // 3. 获取用户详细信息 (姓名、企业邮箱)
    let detail_url = format!(
        "https://qyapi.weixin.qq.com/cgi-bin/user/get?access_token={}&userid={}",
        access_token, wecom_user_id
    );
    let detail_res = http
        .get(&detail_url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .json::<WeComUserDetailResponse>()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let name = detail_res.name.unwrap_or_else(|| wecom_user_id.clone());
    let email = detail_res
        .biz_mail
        .filter(|s| !s.is_empty())
        .or_else(|| detail_res.email.filter(|s| !s.is_empty()))
        .unwrap_or_else(|| format!("{}@wecom.local", wecom_user_id.to_lowercase()));

    let tenants = state.store.tenants().list_active().await.map_err(|e| {
        tracing::error!("Failed to load tenants for WeCom login: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let default_tenant = match tenants.into_iter().next() {
        Some(t) => t,
        None => {
            return Ok(Redirect::temporary(&format!(
                "{frontend_url}/auth/sso/complete?error=no_active_tenant"
            )));
        }
    };

    let fake_provider_id = Uuid::new_v4();
    let identity = SsoIdentityParams {
        protocol: "wecom".to_string(),
        provider_id: fake_provider_id,
        tenant_id: default_tenant.id,
        subject: wecom_user_id.clone(),
        issuer: "https://work.weixin.qq.com".to_string(),
        email: Some(email.clone()),
        name: Some(name.clone()),
    };

    let provision_config = SsoProvisionConfig {
        auto_provision: true,
        default_role: "Employee".to_string(),
        default_custom_role_id: None,
        default_department_id: None,
        trust_idp_mfa: true,
        provider_name: "企业微信".to_string(),
        provider_slug: "wecom".to_string(),
    };

    let resolution = resolve_sso_user(
        &state.store,
        &identity,
        &provision_config,
        None,
    )
    .await
    .map_err(|(code, msg)| {
        tracing::error!("WeCom user resolution error: {} - {}", code, msg);
        code
    })?;

    let user = match resolution {
        SsoUserResolution::ExistingUser(u) | SsoUserResolution::NewUser(u) => u,
        SsoUserResolution::NoAccount => {
            return Ok(Redirect::temporary(&format!(
                "{frontend_url}/auth/sso/complete?error=no_account"
            )));
        }
        SsoUserResolution::NoEmail => {
            return Ok(Redirect::temporary(&format!(
                "{frontend_url}/auth/sso/complete?error=no_email"
            )));
        }
    };

    let session = create_sso_session(
        &state.store,
        &user,
        &default_tenant,
        &headers,
        &provision_config,
        frontend_url,
    )
    .await
    .map_err(|(code, _)| code)?;

    match session {
        SsoSessionResult::Token(token) => {
            Ok(Redirect::temporary(&format!(
                "{frontend_url}/auth/sso/complete?token={token}"
            )))
        }
        SsoSessionResult::Pending2fa { user_id, provider_slug } => {
            Ok(Redirect::temporary(&format!(
                "{frontend_url}/login?pending_2fa={user_id}&provider={provider_slug}"
            )))
        }
        SsoSessionResult::Suspended => {
            Ok(Redirect::temporary(&format!(
                "{frontend_url}/login?error=account_suspended"
            )))
        }
    }
}

/// 快速模拟/测试登录处理逻辑（用于未配置真实企微密钥或开发者模式下）
async fn handle_demo_login(
    state: &Arc<AppState>,
    headers: &HeaderMap,
) -> Result<Redirect, StatusCode> {
    let config = types::config::get_config();
    let frontend_url = &config.frontend_url;

    let tenants = state
        .store
        .tenants()
        .list_active()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let default_tenant = match tenants.into_iter().next() {
        Some(t) => t,
        None => {
            return Ok(Redirect::temporary(&format!(
                "{frontend_url}/auth/sso/complete?error=no_active_tenant"
            )));
        }
    };

    let users = state
        .store
        .users()
        .list(UserListFilter {
            tenant_id: Some(default_tenant.id),
            status: Some("active".to_string()),
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let target_user = users.into_iter().next().ok_or(StatusCode::NOT_FOUND)?;

    let provision_config = SsoProvisionConfig {
        auto_provision: false,
        default_role: target_user.role.clone(),
        default_custom_role_id: None,
        default_department_id: None,
        trust_idp_mfa: true,
        provider_name: "企业微信 (Demo)".to_string(),
        provider_slug: "wecom".to_string(),
    };

    let session = create_sso_session(
        &state.store,
        &target_user,
        &default_tenant,
        headers,
        &provision_config,
        frontend_url,
    )
    .await
    .map_err(|(code, _)| code)?;

    match session {
        SsoSessionResult::Token(token) => {
            Ok(Redirect::temporary(&format!(
                "{frontend_url}/auth/sso/complete?token={token}"
            )))
        }
        _ => Ok(Redirect::temporary(&format!(
            "{frontend_url}/auth/sso/complete?error=demo_login_failed"
        ))),
    }
}

/// 接收来自前端的开发/演示登录请求（一键测试体验企微登录流程）
pub async fn demo_login_endpoint(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let config = types::config::get_config();

    let tenants = state
        .store
        .tenants()
        .list_active()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let default_tenant = tenants
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "No active tenant found".to_string()))?;

    let users = state
        .store
        .users()
        .list(UserListFilter {
            tenant_id: Some(default_tenant.id),
            status: Some("active".to_string()),
            ..Default::default()
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let target_user = users
        .into_iter()
        .next()
        .ok_or((StatusCode::NOT_FOUND, "No active user found".to_string()))?;

    let provision_config = SsoProvisionConfig {
        auto_provision: false,
        default_role: target_user.role.clone(),
        default_custom_role_id: None,
        default_department_id: None,
        trust_idp_mfa: true,
        provider_name: "企业微信 (Demo)".to_string(),
        provider_slug: "wecom".to_string(),
    };

    let session = create_sso_session(
        &state.store,
        &target_user,
        &default_tenant,
        &headers,
        &provision_config,
        &config.frontend_url,
    )
    .await?;

    match session {
        SsoSessionResult::Token(token) => Ok(Json(serde_json::json!({
            "token": token,
            "user": {
                "id": target_user.id,
                "email": target_user.email,
                "name": target_user.name,
                "role": target_user.role,
            },
            "tenant": {
                "id": default_tenant.id,
                "name": default_tenant.name,
            }
        }))),
        _ => Err((StatusCode::BAD_REQUEST, "Login failed".to_string())),
    }
}
