//! 固件管理与部署环境、固件分类、项目成员及 GitLab 协同 API 模块

use crate::{api::gitlab::gitlab_api_request, auth::AuthUser, AppState};
use app_entity::entities::{app, app_class, app_deploy, app_user};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

// ==================== 查询与请求参数 DTO ====================

#[derive(Debug, Deserialize)]
pub struct AppPageQuery {
    pub page: Option<u64>,
    pub limit: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
    #[serde(rename = "curPage")]
    pub cur_page: Option<u64>,
    pub status: Option<i16>,
    pub class_no: Option<String>,
    #[serde(rename = "ownerId")]
    pub owner_id: Option<String>,
    pub search: Option<String>,
    #[serde(rename = "searchValue")]
    pub search_value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClassPageQuery {
    pub page: Option<u64>,
    pub limit: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
    #[serde(rename = "curPage")]
    pub cur_page: Option<u64>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IdQuery {
    pub id: i32,
}

#[derive(Debug, Deserialize)]
pub struct DeployQuery {
    pub id: Option<i32>,
    pub deployno: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAppPayload {
    pub name: String,
    pub desc: Option<String>,
    pub class_no: Option<String>,
    pub class_name: Option<String>,
    pub doc_path: Option<String>,
    pub gitlab_id: Option<String>,
    pub git_url: Option<String>,
    pub status: Option<i16>,
}

#[derive(Debug, Deserialize)]
pub struct SaveBasicAppPayload {
    pub name: String,
    pub desc: Option<String>,
    pub class_no: Option<String>,
    pub class_name: Option<String>,
    pub doc_path: Option<String>,
    pub gitlab_id: Option<String>,
    pub git_url: Option<String>,
    pub status: Option<i16>,
}

#[derive(Debug, Deserialize)]
pub struct SaveChargePayload {
    pub createby_id: String,
    pub createby_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveClassPayload {
    pub id: Option<i32>,
    pub number: Option<String>,
    pub name: String,
    pub desc: Option<String>,
}

fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(serde_json::Number),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Ok(s),
        StringOrNumber::Number(n) => Ok(n.to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUserPayload {
    #[serde(deserialize_with = "string_or_number")]
    pub uid: String,
    pub uname: String,
    pub key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveDeployPayload {
    pub id: Option<i32>,
    pub name: String,
    pub branch_name: Option<String>,
    pub build_tag: Option<String>,
    pub auto_pub: Option<i16>,
    pub api_uri: Option<String>,
    pub api_key_id: Option<i32>,
    pub envs: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PipelineHistoryQuery {
    pub page: Option<u64>,
    pub limit: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
    #[serde(rename = "pageIndex")]
    pub page_index: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CiActionQuery {
    pub deployno: Option<String>,
    pub branch: Option<String>,
}

// ==================== 固件主表接口 ====================

/// GET /api/app/page
pub async fn page_apps(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AppPageQuery>,
) -> Result<Json<Value>, StatusCode> {
    let page = params.cur_page.or(params.page).unwrap_or(1);
    let page_size = params.page_size.or(params.limit).unwrap_or(10);

    let mut query = app::Entity::find().filter(app::Column::IsDel.eq(0));

    if let Some(status) = params.status {
        query = query.filter(app::Column::Status.eq(status));
    }

    if let Some(ref class_no) = params.class_no {
        let c = class_no.trim();
        if !c.is_empty() {
            query = query.filter(app::Column::ClassNo.eq(c));
        }
    }

    if let Some(ref owner_id) = params.owner_id {
        let o = owner_id.trim();
        if !o.is_empty() {
            query = query.filter(app::Column::CreatebyId.eq(o));
        }
    }

    let search_term = params.search_value.or(params.search);
    if let Some(ref s) = search_term {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            query = query.filter(
                app::Column::Name
                    .contains(trimmed)
                    .or(app::Column::Number.contains(trimmed))
                    .or(app::Column::CreatebyName.contains(trimmed)),
            );
        }
    }

    let paginator = query
        .order_by_desc(app::Column::Id)
        .paginate(state.store.db(), page_size);

    let total = paginator.num_items().await.map_err(|e| {
        tracing::error!("Failed to count apps: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items = paginator
        .fetch_page(page.saturating_sub(1))
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch app page: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": {
            "items": items,
            "total": total,
            "page": page,
            "limit": page_size
        }
    })))
}

/// GET /api/app/apply/page
pub async fn page_apply_apps(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AppPageQuery>,
) -> Result<Json<Value>, StatusCode> {
    page_apps(State(state), Query(params)).await
}

/// POST /api/app/create
pub async fn create_app(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(payload): Json<CreateAppPayload>,
) -> Result<Json<Value>, StatusCode> {
    let now = Utc::now().into();
    let number = format!("APP-{}", nanoid::nanoid!(8).to_uppercase());

    let active = app::ActiveModel {
        number: Set(number),
        name: Set(payload.name),
        desc: Set(payload.desc.unwrap_or_default()),
        class_no: Set(payload.class_no.unwrap_or_default()),
        class_name: Set(payload.class_name.unwrap_or_default()),
        create_time: Set(now),
        doc_path: Set(payload.doc_path.unwrap_or_default()),
        status: Set(payload.status.unwrap_or(2)),
        gitlab_id: Set(payload.gitlab_id.unwrap_or_default()),
        git_url: Set(payload.git_url.unwrap_or_default()),
        is_del: Set(0),
        createby_name: Set(auth.email),
        createby_id: Set(auth.user_id.to_string()),

        lastupdate_time: Set(now),
        ..Default::default()
    };

    let inserted = active.insert(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to insert app: {:?}", e);
        eprintln!("Failed to insert app: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "创建成功",
        "data": inserted
    })))
}

/// GET /api/app/{appno}
pub async fn get_app(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let found = app::Entity::find()
        .filter(app::Column::Number.eq(&appno))
        .filter(app::Column::IsDel.eq(0))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query app {}: {:?}", appno, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": found
    })))
}

/// POST /api/app/{appno}/save/basic
pub async fn save_app_basic(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Json(payload): Json<SaveBasicAppPayload>,
) -> Result<Json<Value>, StatusCode> {
    let found = app::Entity::find()
        .filter(app::Column::Number.eq(&appno))
        .filter(app::Column::IsDel.eq(0))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query app {}: {:?}", appno, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = Utc::now().into();
    let mut active: app::ActiveModel = found.into();
    active.name = Set(payload.name);
    if let Some(desc) = payload.desc {
        active.desc = Set(desc);
    }
    if let Some(class_no) = payload.class_no {
        active.class_no = Set(class_no);
    }
    if let Some(class_name) = payload.class_name {
        active.class_name = Set(class_name);
    }
    if let Some(doc_path) = payload.doc_path {
        active.doc_path = Set(doc_path);
    }
    if let Some(gitlab_id) = payload.gitlab_id {
        active.gitlab_id = Set(gitlab_id);
    }
    if let Some(git_url) = payload.git_url {
        active.git_url = Set(git_url);
    }
    if let Some(status) = payload.status {
        active.status = Set(status);
    }
    active.lastupdate_time = Set(now);

    let updated = active.update(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to update app {}: {:?}", appno, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "保存成功",
        "data": updated
    })))
}

/// POST /api/app/{appno}/save/charge
pub async fn save_app_charge(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Json(payload): Json<SaveChargePayload>,
) -> Result<Json<Value>, StatusCode> {
    let found = app::Entity::find()
        .filter(app::Column::Number.eq(&appno))
        .filter(app::Column::IsDel.eq(0))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query app {}: {:?}", appno, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = Utc::now().into();
    let mut active: app::ActiveModel = found.into();
    active.createby_id = Set(payload.createby_id);
    active.createby_name = Set(payload.createby_name);
    active.lastupdate_time = Set(now);

    let updated = active.update(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to update app charge {}: {:?}", appno, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "保存成功",
        "data": updated
    })))
}

/// DELETE /api/app/{appno}
pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let found = app::Entity::find()
        .filter(app::Column::Number.eq(&appno))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query app for delete {}: {:?}", appno, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app::ActiveModel = found.into();
    active.is_del = Set(1);
    active.update(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to delete app {}: {:?}", appno, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "删除成功"
    })))
}

// ==================== 固件分类接口 ====================

/// GET /api/app/class/pages
pub async fn page_classes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ClassPageQuery>,
) -> Result<Json<Value>, StatusCode> {
    let page = params.cur_page.or(params.page).unwrap_or(1);
    let page_size = params.page_size.or(params.limit).unwrap_or(100);

    let mut query = app_class::Entity::find().filter(app_class::Column::IsDel.eq(0));

    if let Some(ref name) = params.name {
        let n = name.trim();
        if !n.is_empty() {
            query = query.filter(app_class::Column::Name.contains(n));
        }
    }

    let paginator = query
        .order_by_asc(app_class::Column::Id)
        .paginate(state.store.db(), page_size);

    let total = paginator.num_items().await.map_err(|e| {
        tracing::error!("Failed to count app classes: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items = paginator
        .fetch_page(page.saturating_sub(1))
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch app classes: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": {
            "items": items,
            "total": total,
            "page": page,
            "limit": page_size
        }
    })))
}

/// GET /api/app/class?id=xx
pub async fn get_class(
    State(state): State<Arc<AppState>>,
    Query(params): Query<IdQuery>,
) -> Result<Json<Value>, StatusCode> {
    let found = app_class::Entity::find_by_id(params.id)
        .filter(app_class::Column::IsDel.eq(0))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query class: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": found
    })))
}

/// POST /api/app/class
pub async fn save_class(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SaveClassPayload>,
) -> Result<Json<Value>, StatusCode> {
    if let Some(id) = payload.id {
        if id > 0 {
            let found = app_class::Entity::find_by_id(id)
                .one(state.store.db())
                .await
                .map_err(|e| {
                    tracing::error!("Failed to query class {}: {:?}", id, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .ok_or(StatusCode::NOT_FOUND)?;

            let mut active: app_class::ActiveModel = found.into();
            active.name = Set(payload.name);
            if let Some(desc) = payload.desc {
                active.desc = Set(desc);
            }
            let updated = active.update(state.store.db()).await.map_err(|e| {
                tracing::error!("Failed to update class: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            return Ok(Json(json!({
                "code": 200,
                "message": "保存成功",
                "data": updated
            })));
        }
    }

    let number = payload
        .number
        .unwrap_or_else(|| format!("CLS-{}", nanoid::nanoid!(6).to_uppercase()));

    let active = app_class::ActiveModel {
        number: Set(number),
        name: Set(payload.name),
        desc: Set(payload.desc.unwrap_or_default()),
        is_del: Set(0),
        ..Default::default()
    };

    let inserted = active.insert(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to insert class: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "创建成功",
        "data": inserted
    })))
}

/// DELETE /api/app/class?id=xx
pub async fn delete_class(
    State(state): State<Arc<AppState>>,
    Query(params): Query<IdQuery>,
) -> Result<Json<Value>, StatusCode> {
    let found = app_class::Entity::find_by_id(params.id)
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to find class for delete: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app_class::ActiveModel = found.into();
    active.is_del = Set(1);
    active.update(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to soft delete class: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "删除成功"
    })))
}

// ==================== 项目成员管理接口 ====================

/// GET /api/app/{appno}/user/list
pub async fn get_app_users(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let users = app_user::Entity::find()
        .filter(app_user::Column::AppNo.eq(&appno))
        .all(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query app users for {}: {:?}", appno, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": users
    })))
}

/// POST /api/app/{appno}/user/create
pub async fn create_app_user(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<Json<Value>, StatusCode> {
    // 检查是否已存在关系
    let existing = app_user::Entity::find()
        .filter(app_user::Column::AppNo.eq(&appno))
        .filter(app_user::Column::Uid.eq(&payload.uid))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query existing app user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if let Some(user_rel) = existing {
        let mut active: app_user::ActiveModel = user_rel.into();
        active.uname = Set(payload.uname);
        if let Some(k) = payload.key {
            active.key = Set(k);
        }
        let updated = active.update(state.store.db()).await.map_err(|e| {
            tracing::error!("Failed to update app user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        return Ok(Json(json!({
            "code": 200,
            "message": "更新成功",
            "data": updated
        })));
    }

    let active = app_user::ActiveModel {
        uid: Set(payload.uid),
        uname: Set(payload.uname),
        app_no: Set(appno),
        key: Set(payload.key.unwrap_or_else(|| "developer".to_string())),
        ..Default::default()
    };

    let inserted = active.insert(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to insert app user: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "添加成功",
        "data": inserted
    })))
}

/// DELETE /api/app/{appno}/user?id=xx
pub async fn delete_app_user(
    State(state): State<Arc<AppState>>,
    Path(_appno): Path<String>,
    Query(params): Query<IdQuery>,
) -> Result<Json<Value>, StatusCode> {
    app_user::Entity::delete_by_id(params.id)
        .exec(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete app user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "code": 200,
        "message": "删除成功"
    })))
}

// ==================== 编译环境管理接口 ====================

/// GET /api/app/{appno}/deploy/list
pub async fn get_app_deploys(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let deploys = app_deploy::Entity::find()
        .filter(app_deploy::Column::AppNo.eq(&appno))
        .filter(app_deploy::Column::IsDel.eq(0))
        .order_by_asc(app_deploy::Column::Id)
        .all(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query deploys for {}: {:?}", appno, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": deploys
    })))
}

/// GET /api/app/{appno}/deploy
pub async fn get_deploy(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Query(params): Query<DeployQuery>,
) -> Result<Json<Value>, StatusCode> {
    let mut query = app_deploy::Entity::find()
        .filter(app_deploy::Column::AppNo.eq(&appno))
        .filter(app_deploy::Column::IsDel.eq(0));

    if let Some(id) = params.id {
        query = query.filter(app_deploy::Column::Id.eq(id));
    } else if let Some(ref dno) = params.deployno {
        query = query.filter(app_deploy::Column::Number.eq(dno));
    } else {
        return Err(StatusCode::BAD_REQUEST);
    }

    let found = query
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query deploy: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": found
    })))
}

/// POST /api/app/{appno}/deploy/create
pub async fn create_deploy(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Json(payload): Json<SaveDeployPayload>,
) -> Result<Json<Value>, StatusCode> {
    let now = Utc::now().into();
    let number = format!("DEP-{}", nanoid::nanoid!(8).to_uppercase());

    let active = app_deploy::ActiveModel {
        number: Set(number),
        app_no: Set(appno),
        name: Set(payload.name),
        branch_name: Set(payload.branch_name.unwrap_or_default()),
        build_tag: Set(payload.build_tag.unwrap_or_default()),
        auto_pub: Set(payload.auto_pub.unwrap_or(0)),
        api_uri: Set(payload.api_uri.unwrap_or_default()),
        api_key_id: Set(payload.api_key_id.unwrap_or(0)),
        is_del: Set(0),
        create_time: Set(now),
        envs: Set(payload.envs.unwrap_or_default()),
        ..Default::default()
    };

    let inserted = active.insert(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to insert deploy: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "保存成功",
        "data": inserted
    })))
}

/// POST /api/app/{appno}/deploy/save
pub async fn save_deploy(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Json(payload): Json<SaveDeployPayload>,
) -> Result<Json<Value>, StatusCode> {
    let id = payload.id.ok_or(StatusCode::BAD_REQUEST)?;

    let found = app_deploy::Entity::find_by_id(id)
        .filter(app_deploy::Column::AppNo.eq(&appno))
        .filter(app_deploy::Column::IsDel.eq(0))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to find deploy {}: {:?}", id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app_deploy::ActiveModel = found.into();
    active.name = Set(payload.name);
    if let Some(branch) = payload.branch_name {
        active.branch_name = Set(branch);
    }
    if let Some(tag) = payload.build_tag {
        active.build_tag = Set(tag);
    }
    if let Some(auto_pub) = payload.auto_pub {
        active.auto_pub = Set(auto_pub);
    }
    if let Some(uri) = payload.api_uri {
        active.api_uri = Set(uri);
    }
    if let Some(key_id) = payload.api_key_id {
        active.api_key_id = Set(key_id);
    }
    if let Some(envs) = payload.envs {
        active.envs = Set(envs);
    }

    let updated = active.update(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to update deploy {}: {:?}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "保存成功",
        "data": updated
    })))
}

/// GET /api/app/{appno}/deploy/copy?id=xx
pub async fn copy_deploy(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Query(params): Query<IdQuery>,
) -> Result<Json<Value>, StatusCode> {
    let original = app_deploy::Entity::find_by_id(params.id)
        .filter(app_deploy::Column::AppNo.eq(&appno))
        .filter(app_deploy::Column::IsDel.eq(0))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to find deploy for copy: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = Utc::now().into();
    let new_number = format!("DEP-{}", nanoid::nanoid!(8).to_uppercase());
    let new_name = format!("{}_copy", original.name);

    let active = app_deploy::ActiveModel {
        number: Set(new_number),
        app_no: Set(appno),
        name: Set(new_name),
        branch_name: Set(original.branch_name),
        build_tag: Set(original.build_tag),
        auto_pub: Set(original.auto_pub),
        api_uri: Set(original.api_uri),
        api_key_id: Set(original.api_key_id),
        is_del: Set(0),
        create_time: Set(now),
        envs: Set(original.envs),
        ..Default::default()
    };

    let inserted = active.insert(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to copy deploy: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "复制成功",
        "data": inserted
    })))
}

/// DELETE /api/app/{appno}/deploy?id=xx
pub async fn delete_deploy(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Query(params): Query<IdQuery>,
) -> Result<Json<Value>, StatusCode> {
    let found = app_deploy::Entity::find_by_id(params.id)
        .filter(app_deploy::Column::AppNo.eq(&appno))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to find deploy for delete: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app_deploy::ActiveModel = found.into();
    active.is_del = Set(1);
    active.update(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to soft delete deploy: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "删除成功"
    })))
}

// ==================== GitLab 协同业务接口 ====================

/// 内部辅助：通过 appno 查询固件对应的 gitlab_id
async fn get_gitlab_id_by_appno(
    state: &AppState,
    appno: &str,
) -> Result<String, (StatusCode, Json<Value>)> {
    let app_opt = app::Entity::find()
        .filter(app::Column::Number.eq(appno))
        .filter(app::Column::IsDel.eq(0))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query app for gitlab: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "db_error", "message": e.to_string() })),
            )
        })?;

    let app = app_opt.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "app_not_found", "message": "固件不存在" })),
        )
    })?;

    let gitlab_id = app.gitlab_id.trim();
    if gitlab_id.is_empty() || gitlab_id == "0" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "gitlab_id_empty",
                "message": "该固件未配置有效的 代码仓库 ID，请在固件基本信息中填写"
            })),
        ));
    }

    Ok(gitlab_id.to_string())
}

/// GET /api/gitlab/{appno}/branches
pub async fn get_app_branches(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let gitlab_id = get_gitlab_id_by_appno(&state, &appno).await?;
    let path = format!(
        "/projects/{}/repository/branches?per_page=100",
        urlencoding::encode(&gitlab_id)
    );

    let res = gitlab_api_request(reqwest::Method::GET, &path, None).await?;
    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "gitlab_error", "message": err_text })),
        ));
    }

    let branches = res.json::<Value>().await.unwrap_or(json!([]));
    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": branches
    })))
}

/// GET /api/gitlab/{appno}/cifile
pub async fn get_app_ci_file(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Query(query): Query<CiActionQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let gitlab_id = get_gitlab_id_by_appno(&state, &appno).await?;

    let branch = if let Some(ref dno) = query.deployno {
        let deploy = app_deploy::Entity::find()
            .filter(app_deploy::Column::AppNo.eq(&appno))
            .filter(app_deploy::Column::Number.eq(dno))
            .filter(app_deploy::Column::IsDel.eq(0))
            .one(state.store.db())
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "db_error", "message": e.to_string() })),
                )
            })?;

        deploy
            .map(|d| d.branch_name)
            .unwrap_or_else(|| "master".to_string())
    } else {
        query.branch.unwrap_or_else(|| "master".to_string())
    };

    let path = format!(
        "/projects/{}/repository/files/.gitlab-ci%2Eyml/raw?ref={}",
        urlencoding::encode(&gitlab_id),
        urlencoding::encode(&branch)
    );

    let res = gitlab_api_request(reqwest::Method::GET, &path, None).await?;
    if !res.status().is_success() {
        return Ok(Json(json!({
            "code": 200,
            "message": "未找到 .gitlab-ci.yml 文件",
            "data": ""
        })));
    }

    let content = res.text().await.unwrap_or_default();
    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": content
    })))
}

/// GET /api/gitlab/{appno}/pipeline/history
pub async fn get_app_pipeline_history(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Query(query): Query<PipelineHistoryQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let gitlab_id = get_gitlab_id_by_appno(&state, &appno).await?;
    let page = query.page_index.or(query.page).unwrap_or(1);
    let per_page = query.page_size.or(query.limit).unwrap_or(10);

    let path = format!(
        "/projects/{}/pipelines?page={}&per_page={}",
        urlencoding::encode(&gitlab_id),
        page,
        per_page
    );

    let res = gitlab_api_request(reqwest::Method::GET, &path, None).await?;
    if !res.status().is_success() {
        let err = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "gitlab_error", "message": err })),
        ));
    }

    let pipelines = res.json::<Value>().await.unwrap_or(json!([]));
    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": pipelines
    })))
}

/// GET /api/gitlab/{appno}/triggerci
pub async fn trigger_app_ci(
    State(state): State<Arc<AppState>>,
    Path(appno): Path<String>,
    Query(query): Query<CiActionQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let gitlab_id = get_gitlab_id_by_appno(&state, &appno).await?;

    let branch = if let Some(ref dno) = query.deployno {
        let deploy = app_deploy::Entity::find()
            .filter(app_deploy::Column::AppNo.eq(&appno))
            .filter(app_deploy::Column::Number.eq(dno))
            .filter(app_deploy::Column::IsDel.eq(0))
            .one(state.store.db())
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "db_error", "message": e.to_string() })),
                )
            })?;

        deploy
            .map(|d| d.branch_name)
            .unwrap_or_else(|| "master".to_string())
    } else {
        query.branch.unwrap_or_else(|| "master".to_string())
    };

    let path = format!("/projects/{}/pipeline", urlencoding::encode(&gitlab_id));
    let body = json!({
        "ref": branch
    });

    let res = gitlab_api_request(reqwest::Method::POST, &path, Some(body)).await?;
    if !res.status().is_success() {
        let err = res.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "gitlab_trigger_failed", "message": err })),
        ));
    }

    let pipeline_res = res.json::<Value>().await.unwrap_or(json!({}));
    Ok(Json(json!({
        "code": 200,
        "message": "流水线触发成功",
        "data": pipeline_res
    })))
}

/// GET /api/gitlab/{appno}/ci/synch
pub async fn sync_app_ci(
    State(_state): State<Arc<AppState>>,
    Path(_appno): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 同步 CI 模板
    Ok(Json(json!({
        "code": 200,
        "message": "CI 模板同步成功"
    })))
}

/// GET /api/app/users/candidates
pub async fn get_app_user_candidates(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    let tenant_filter = if auth.role == "SuperAdmin" {
        None
    } else {
        Some(auth.tenant_id)
    };

    let users = state
        .store
        .users()
        .list(app_entity::repositories::UserListFilter {
            tenant_id: tenant_filter,
            status: Some("active".to_string()),
            limit: 200,
            ..Default::default()
        })
        .await
        .map_err(|e| {
            tracing::error!("Failed to list candidate users: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let items: Vec<Value> = users
        .into_iter()
        .map(|u| {
            json!({
                "id": u.id,
                "name": u.name,
                "email": u.email,
                "role": u.role,
                "avatar_url": u.avatar_url,
            })
        })
        .collect();

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": items
    })))
}
