//! GitLab API 客户端与 CI/CD 模块
//!
//! 提供基于官方 `gitlab` crate 的异步集成：
//! - 连接健康检查与 Token 认证状态检测
//! - 项目信息（项目元数据、分支列表、标签列表）
//! - CI/CD 流水线管理（触发流水线、状态监控、取消、重试）
//! - 构建 Job 任务管理与实时/历史控制台日志拉取 (Trace Logs)

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use gitlab::AsyncGitlab;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use base64::Engine;

// ==================== 数据传输对象 (DTOs) ====================

#[derive(Debug, Serialize)]
pub struct GitlabStatusResponse {
    pub enabled: bool,
    pub configured: bool,
    pub url: String,
    pub default_project_id: Option<String>,
    pub current_user: Option<GitlabUserInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitlabUserInfo {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub state: String,
    pub avatar_url: Option<String>,
    pub web_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ListPipelinesQuery {
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePipelinePayload {
    #[serde(rename = "ref")]
    pub ref_name: String,
    #[serde(default)]
    pub variables: Option<Vec<PipelineVariable>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineVariable {
    pub key: String,
    pub value: String,
    #[serde(rename = "variable_type")]
    pub variable_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetCiFileQuery {
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SyncCiFilePayload {
    #[serde(default)]
    pub branch: Option<String>,
    pub content: String,
    #[serde(default)]
    pub commit_message: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LintCiFilePayload {
    pub content: String,
    #[serde(default)]
    pub dry_run: Option<bool>,
}

// ==================== 客户端构造辅助函数 ====================

/// 规范化 project_id（支持数值 ID 或 URL 编码的命名空间路径，如 "group/project" -> "group%2Fproject"）
fn normalize_project_id(id: &str) -> String {
    let trimmed = id.trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        trimmed.to_string()
    } else {
        urlencoding::encode(trimmed).into_owned()
    }
}

/// 解析并构造异步 GitLab 客户端
pub async fn get_gitlab_client() -> Result<(AsyncGitlab, String, String), (StatusCode, Json<Value>)>
{
    let config = types::config::get_config();
    let gitlab_conf = &config.gitlab;

    if !gitlab_conf.enabled {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "gitlab_disabled",
                "message": "GitLab integration is disabled in configuration"
            })),
        ));
    }

    if gitlab_conf.token.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "gitlab_not_configured",
                "message": "GitLab access token is not configured in backend/etc/config.toml"
            })),
        ));
    }

    let url_str = gitlab_conf.url.trim_end_matches('/');
    let client = gitlab::GitlabBuilder::new(url_str, gitlab_conf.token.trim())
        .build_async()
        .await
        .map_err(|e| {
            tracing::error!("Failed to create GitLab async client: {:?}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": "gitlab_client_error",
                    "message": format!("Failed to connect to GitLab: {}", e)
                })),
            )
        })?;

    Ok((
        client,
        url_str.to_string(),
        gitlab_conf.token.trim().to_string(),
    ))
}

/// 底层使用 reqwest 携带认证 Token 请求 GitLab REST API（通用高效网关）
pub(crate) async fn gitlab_api_request(
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<reqwest::Response, (StatusCode, Json<Value>)> {
    let (_, base_url, token) = get_gitlab_client().await?;
    let config = types::config::get_config();
    let timeout = Duration::from_secs(config.gitlab.timeout_secs.max(5));

    let full_url = format!("{}/api/v4{}", base_url, path);
    let http = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "http_client_init_failed", "message": e.to_string() })),
            )
        })?;

    let mut req = http
        .request(method, &full_url)
        .header("PRIVATE-TOKEN", &token)
        .header("Accept", "application/json");

    if let Some(b) = body {
        req = req.json(&b);
    }

    req.send().await.map_err(|e| {
        tracing::error!("GitLab API HTTP request failed: {:?}", e);
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": "gitlab_upstream_unreachable",
                "message": format!("Failed to reach GitLab instance: {}", e)
            })),
        )
    })
}

// ==================== 接口端点实现 ====================

/// `GET /api/gitlab/status`: 检查 GitLab 配置与连通性
pub async fn get_status() -> Result<Json<GitlabStatusResponse>, (StatusCode, Json<Value>)> {
    let config = types::config::get_config();
    let gitlab_conf = &config.gitlab;

    if !gitlab_conf.enabled || gitlab_conf.token.trim().is_empty() {
        return Ok(Json(GitlabStatusResponse {
            enabled: gitlab_conf.enabled,
            configured: false,
            url: gitlab_conf.url.clone(),
            default_project_id: gitlab_conf.default_project_id.clone(),
            current_user: None,
            error: Some("GitLab token is not configured".to_string()),
        }));
    }

    match gitlab_api_request(reqwest::Method::GET, "/user", None).await {
        Ok(res) => {
            if res.status().is_success() {
                let user_info = res.json::<GitlabUserInfo>().await.ok();
                Ok(Json(GitlabStatusResponse {
                    enabled: true,
                    configured: true,
                    url: gitlab_conf.url.clone(),
                    default_project_id: gitlab_conf.default_project_id.clone(),
                    current_user: user_info,
                    error: None,
                }))
            } else {
                let err_text = res.text().await.unwrap_or_default();
                Ok(Json(GitlabStatusResponse {
                    enabled: true,
                    configured: false,
                    url: gitlab_conf.url.clone(),
                    default_project_id: gitlab_conf.default_project_id.clone(),
                    current_user: None,
                    error: Some(format!("GitLab authentication failed: {}", err_text)),
                }))
            }
        }
        Err((_, err_json)) => Ok(Json(GitlabStatusResponse {
            enabled: true,
            configured: false,
            url: gitlab_conf.url.clone(),
            default_project_id: gitlab_conf.default_project_id.clone(),
            current_user: None,
            error: Some(
                err_json
                    .0
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Network error")
                    .to_string(),
            ),
        })),
    }
}

/// `GET /api/gitlab/projects/{project_id}`: 获取项目元数据与详细信息
pub async fn get_project(
    Path(project_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(reqwest::Method::GET, &format!("/projects/{}", pid), None).await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `GET /api/gitlab/projects/{project_id}/branches`: 获取项目分支列表
pub async fn list_branches(
    Path(project_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(
        reqwest::Method::GET,
        &format!("/projects/{}/repository/branches?per_page=100", pid),
        None,
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `GET /api/gitlab/projects/{project_id}/tags`: 获取项目标签列表
pub async fn list_tags(
    Path(project_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(
        reqwest::Method::GET,
        &format!("/projects/{}/repository/tags?per_page=100", pid),
        None,
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `GET /api/gitlab/projects/{project_id}/pipelines`: 获取项目流水线列表
pub async fn list_pipelines(
    Path(project_id): Path<String>,
    Query(query): Query<ListPipelinesQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let mut params = vec![];

    if let Some(r) = query.ref_name {
        params.push(format!("ref={}", urlencoding::encode(&r)));
    }
    if let Some(s) = query.status {
        params.push(format!("status={}", urlencoding::encode(&s)));
    }
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    params.push(format!("page={}", page));
    params.push(format!("per_page={}", per_page));

    let qs = params.join("&");
    let res = gitlab_api_request(
        reqwest::Method::GET,
        &format!("/projects/{}/pipelines?{}", pid, qs),
        None,
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `POST /api/gitlab/projects/{project_id}/pipelines`: 触发创建并执行新流水线
pub async fn create_pipeline(
    Path(project_id): Path<String>,
    Json(payload): Json<CreatePipelinePayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let body = json!({
        "ref": payload.ref_name,
        "variables": payload.variables.unwrap_or_default()
    });

    let res = gitlab_api_request(
        reqwest::Method::POST,
        &format!("/projects/{}/pipeline", pid),
        Some(body),
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `GET /api/gitlab/projects/{project_id}/pipelines/{pipeline_id}`: 获取流水线当前状态与详情
pub async fn get_pipeline(
    Path((project_id, pipeline_id)): Path<(String, u64)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(
        reqwest::Method::GET,
        &format!("/projects/{}/pipelines/{}", pid, pipeline_id),
        None,
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `POST /api/gitlab/projects/{project_id}/pipelines/{pipeline_id}/cancel`: 取消正在执行的流水线
pub async fn cancel_pipeline(
    Path((project_id, pipeline_id)): Path<(String, u64)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(
        reqwest::Method::POST,
        &format!("/projects/{}/pipelines/{}/cancel", pid, pipeline_id),
        None,
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `POST /api/gitlab/projects/{project_id}/pipelines/{pipeline_id}/retry`: 重试失败的流水线
pub async fn retry_pipeline(
    Path((project_id, pipeline_id)): Path<(String, u64)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(
        reqwest::Method::POST,
        &format!("/projects/{}/pipelines/{}/retry", pid, pipeline_id),
        None,
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `GET /api/gitlab/projects/{project_id}/pipelines/{pipeline_id}/jobs`: 获取流水线下所有构建 Job 任务
pub async fn list_pipeline_jobs(
    Path((project_id, pipeline_id)): Path<(String, u64)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(
        reqwest::Method::GET,
        &format!(
            "/projects/{}/pipelines/{}/jobs?per_page=100",
            pid, pipeline_id
        ),
        None,
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `GET /api/gitlab/projects/{project_id}/jobs/{job_id}`: 获取单个 Job 详情及执行状态
pub async fn get_job(
    Path((project_id, job_id)): Path<(String, u64)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(
        reqwest::Method::GET,
        &format!("/projects/{}/jobs/{}", pid, job_id),
        None,
    )
    .await?;
    let status = res.status();
    let json_val = res.json::<Value>().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json_val))
    } else {
        Err((status, Json(json_val)))
    }
}

/// `GET /api/gitlab/projects/{project_id}/jobs/{job_id}/log`: 获取单个 Job 的实时构建控制台日志 (Trace)
pub async fn get_job_log(
    Path((project_id, job_id)): Path<(String, u64)>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let res = gitlab_api_request(
        reqwest::Method::GET,
        &format!("/projects/{}/jobs/{}/trace", pid, job_id),
        None,
    )
    .await?;
    let status = res.status();

    if status.is_success() {
        let log_text = res.text().await.unwrap_or_default();
        Ok((
            StatusCode::OK,
            [("Content-Type", "text/plain; charset=utf-8")],
            log_text,
        )
            .into_response())
    } else {
        let err_json = res.json::<Value>().await.unwrap_or_default();
        Err((status, Json(err_json)))
    }
}

// ==================== CI 配置文件同步与校验 (CI File & Lint) ====================

/// `GET /api/gitlab/projects/{project_id}/ci-file`: 获取项目 CI 配置文件内容（默认为 .gitlab-ci.yml）
pub async fn get_ci_file(
    Path(project_id): Path<String>,
    Query(query): Query<GetCiFileQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let file_path = query.file_path.as_deref().unwrap_or(".gitlab-ci.yml");
    let encoded_path = urlencoding::encode(file_path);

    // 若未指定分支，查询项目的 default_branch
    let branch = match query.ref_name {
        Some(r) if !r.trim().is_empty() => r,
        _ => {
            let proj_res =
                gitlab_api_request(reqwest::Method::GET, &format!("/projects/{}", pid), None)
                    .await?;
            if proj_res.status().is_success() {
                let pjson: Value = proj_res.json().await.unwrap_or_default();
                pjson
                    .get("default_branch")
                    .and_then(|b| b.as_str())
                    .unwrap_or("main")
                    .to_string()
            } else {
                "main".to_string()
            }
        }
    };

    let url_endpoint = format!(
        "/projects/{}/repository/files/{}?ref={}",
        pid,
        encoded_path,
        urlencoding::encode(&branch)
    );

    let res = gitlab_api_request(reqwest::Method::GET, &url_endpoint, None).await?;
    let status = res.status();
    let mut val: Value = res.json().await.unwrap_or_default();

    if status.is_success() {
        // 如果包含 base64 编码的 content，自动解码成 utf-8 明文存入 raw_content
        if let Some(content_str) = val.get("content").and_then(|c| c.as_str()) {
            let clean_b64: String = content_str.chars().filter(|c| !c.is_whitespace()).collect();
            if let Ok(decoded_bytes) = base64::engine::general_purpose::STANDARD.decode(clean_b64) {
                if let Ok(raw_text) = String::from_utf8(decoded_bytes) {
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert("raw_content".to_string(), Value::String(raw_text));
                    }
                }
            }
        }
        Ok(Json(val))
    } else {
        Err((status, Json(val)))
    }
}

/// `POST /api/gitlab/projects/{project_id}/ci-file`: 同步（创建或更新）CI 配置文件至远端 代码仓库
pub async fn sync_ci_file(
    Path(project_id): Path<String>,
    Json(payload): Json<SyncCiFilePayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let file_path = payload.file_path.as_deref().unwrap_or(".gitlab-ci.yml");
    let encoded_path = urlencoding::encode(file_path);

    // 获取目标分支，未指定则使用项目默认分支
    let branch = match payload.branch {
        Some(b) if !b.trim().is_empty() => b,
        _ => {
            let proj_res =
                gitlab_api_request(reqwest::Method::GET, &format!("/projects/{}", pid), None)
                    .await?;
            if proj_res.status().is_success() {
                let pjson: Value = proj_res.json().await.unwrap_or_default();
                pjson
                    .get("default_branch")
                    .and_then(|b| b.as_str())
                    .unwrap_or("main")
                    .to_string()
            } else {
                "main".to_string()
            }
        }
    };

    // 检查文件在远端分支是否已存在
    let check_endpoint = format!(
        "/projects/{}/repository/files/{}?ref={}",
        pid,
        encoded_path,
        urlencoding::encode(&branch)
    );
    let check_res = gitlab_api_request(reqwest::Method::GET, &check_endpoint, None).await?;
    let file_exists = check_res.status().is_success();

    let commit_msg = payload.commit_message.unwrap_or_else(|| {
        if file_exists {
            format!("ci: update {} via Fileyard", file_path)
        } else {
            format!("ci: create {} via Fileyard", file_path)
        }
    });

    let sync_body = json!({
        "branch": branch,
        "content": payload.content,
        "commit_message": commit_msg
    });

    let write_endpoint = format!("/projects/{}/repository/files/{}", pid, encoded_path);
    let write_method = if file_exists {
        reqwest::Method::PUT
    } else {
        reqwest::Method::POST
    };

    let res = gitlab_api_request(write_method, &write_endpoint, Some(sync_body)).await?;
    let status = res.status();
    let val: Value = res.json().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(json!({
            "success": true,
            "action": if file_exists { "updated" } else { "created" },
            "file_path": file_path,
            "branch": branch,
            "result": val
        })))
    } else {
        Err((status, Json(val)))
    }
}

/// `POST /api/gitlab/projects/{project_id}/ci-lint`: 校验 CI 配置文件语法有效性
pub async fn lint_ci_file(
    Path(project_id): Path<String>,
    Json(payload): Json<LintCiFilePayload>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let pid = normalize_project_id(&project_id);
    let lint_body = json!({
        "content": payload.content,
        "dry_run": payload.dry_run.unwrap_or(true)
    });

    let res = gitlab_api_request(
        reqwest::Method::POST,
        &format!("/projects/{}/ci/lint", pid),
        Some(lint_body),
    )
    .await?;
    let status = res.status();
    let val: Value = res.json().await.unwrap_or_default();

    if status.is_success() {
        Ok(Json(val))
    } else {
        Err((status, Json(val)))
    }
}
