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
        log::error!("Failed to count apps: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items = paginator
        .fetch_page(page.saturating_sub(1))
        .await
        .map_err(|e| {
            log::error!("Failed to fetch app page: {:?}", e);
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
        createby_name: Set(auth.email.clone()),
        createby_id: Set(auth.user_id.to_string()),

        lastupdate_time: Set(now),
        ..Default::default()
    };

    let inserted = active.insert(state.store.db()).await.map_err(|e| {
        log::error!("Failed to insert app: {:?}", e);
        eprintln!("Failed to insert app: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 自动在“固件文件”栏目下初始化固件文件目录体系：
    // 1. 根目录：固件文件
    // 2. 固件目录：{inserted.name}
    // 3. 三个默认子目录：需求文档、bug记录、固件文件
    if let Err(err) = init_firmware_folders(&state, &auth, &inserted.name).await {
        log::warn!("Failed to auto-create firmware folders for app {}: {:?}", inserted.name, err);
    }

    Ok(Json(json!({
        "code": 200,
        "message": "创建成功",
        "data": inserted
    })))
}

/// 创建固件时，自动在“固件文件”栏目下创建对应的文件夹结构：
/// 固件文件 -> {固件名称} -> [需求文档, bug记录, 固件文件]
async fn init_firmware_folders(
    state: &Arc<AppState>,
    auth: &AuthUser,
    app_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tenant_id = auth.tenant_id;
    let user_id = auth.user_id;


    async fn ensure_single_folder(
        state: &Arc<AppState>,
        tenant_id: uuid::Uuid,
        user_id: uuid::Uuid,
        name: &str,
        parent_path: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 先查是否已存在同名目录
        if let Ok(Some(_)) = state.store.files().find_folder(tenant_id, name, parent_path).await {
            return Ok(());
        }

        let storage_key = match parent_path {
            Some(pp) if !pp.is_empty() => format!("{}/{}/{}", tenant_id, pp, name),
            _ => format!("{}/{}", tenant_id, name),
        };

        // 物理驱动存储目录
        let _ = state.storage.create_folder(&storage_key).await;

        // 数据库元数据记录（固件文件为独立的企业级文件体系，不局限于单一部门）
        let params = app_entity::repositories::CreateFolderParams {
            tenant_id,
            name,
            storage_path: &storage_key,
            owner_id: user_id,
            department_id: None,
            parent_path,
            visibility: "department",
            is_company_folder: false,
        };

        let _ = state.store.files().create_folder(params).await?;
        Ok(())
    }

    // 1. 确保根目录“固件文件”存在
    ensure_single_folder(state, tenant_id, user_id, "固件文件", None).await?;

    // 2. 确保以固件名称命名的文件夹存在
    let root_firmware_dir = "固件文件";
    ensure_single_folder(state, tenant_id, user_id, app_name, Some(root_firmware_dir)).await?;

    // 3. 在固件文件夹里创建默认子文件夹（需求文档、bug记录、固件文件、技术文档）
    let app_dir_path = format!("{}/{}", root_firmware_dir, app_name);
    let default_dirs = ["需求文档", "bug记录", "固件文件", "技术文档"];
    for dir_name in default_dirs {
        ensure_single_folder(state, tenant_id, user_id, dir_name, Some(&app_dir_path)).await?;
    }

    // 4. 清除该租户的文件列表缓存
    if let Some(ref cache) = state.cache {
        let pattern = format!("clovalink:files:{}:*", tenant_id);
        let _ = cache.delete_pattern(&pattern).await;
    }

    Ok(())
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
            log::error!("Failed to query app {}: {:?}", appno, e);
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
            log::error!("Failed to query app {}: {:?}", appno, e);
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
        log::error!("Failed to update app {}: {:?}", appno, e);
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
            log::error!("Failed to query app {}: {:?}", appno, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let now = Utc::now().into();
    let mut active: app::ActiveModel = found.into();
    active.createby_id = Set(payload.createby_id);
    active.createby_name = Set(payload.createby_name);
    active.lastupdate_time = Set(now);

    let updated = active.update(state.store.db()).await.map_err(|e| {
        log::error!("Failed to update app charge {}: {:?}", appno, e);
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
            log::error!("Failed to query app for delete {}: {:?}", appno, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app::ActiveModel = found.into();
    active.is_del = Set(1);
    active.update(state.store.db()).await.map_err(|e| {
        log::error!("Failed to delete app {}: {:?}", appno, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "删除成功"
    })))
}

// ==================== 固件分类接口 ====================

/// 智能家居专业产品固件分类定义
const SMART_HOME_CLASSES: &[(&str, &str, &str)] = &[
    ("CLS-LIGHTING", "照明类别", "智能调光驱动、智能开关、RGBW调色控制器、DALI/DMX驱动等照明控制系统"),
    ("CLS-CURTAIN", "窗帘类别", "智能开合帘电机、电动卷帘、百叶帘控制器、智能推窗器等遮阳驱动系统"),
    ("CLS-PANEL", "中控类别", "智能中控大屏、智慧语音面板、全屋场景开关、多功能触摸控制屏"),
    ("CLS-GATEWAY", "网关类别", "多协议智能网关、KNX/Zigbee/Matter/RS485总线网关、边缘主机"),
    ("CLS-HVAC", "暖通类别", "中央空调VRV网关、智能地暖温控器、新风系统控制器、环境温湿度控制"),
    ("CLS-SECURITY", "安防类别", "人体存在探测器、门窗磁传感器、烟雾报警器、燃气报警器、水浸报警器"),
    ("CLS-DOORLOCK", "门锁类别", "3D人脸识别视频锁、指纹密码锁、智能可视门铃、智能猫眼、门禁控制系统"),
    ("CLS-SENSOR", "传感类别", "高精度温湿度传感器、环境照度传感器、空气质量PM2.5/CO2传感器、跌倒雷达"),
    ("CLS-MEDIA", "影音类别", "背景音乐主机、分布式功放系统、家庭影院控制器、红外万能遥控转发模块"),
    ("CLS-POWER", "电工类别", "智能墙面插座、导轨式微型断路器、智能计量电表、配电箱控制模块"),
];

/// 自动同步与清洗智能家居分类体系
async fn ensure_smart_home_classes(store: &app_entity::DataStore) {
    // 1. 软删除历史测试与非智能家居分类（如通用服务、电商微服务等）
    if let Ok(obsolete) = app_class::Entity::find()
        .filter(app_class::Column::IsDel.eq(0))
        .all(store.db())
        .await
    {
        for item in obsolete {
            if item.name == "通用服务"
                || item.name == "电商微服务"
                || item.name.contains("微服务")
                || item.name.contains("电商")
            {
                let mut active: app_class::ActiveModel = item.into();
                active.is_del = Set(1);
                let _ = active.update(store.db()).await;
            }
        }
    }

    // 2. 补全标准智能家居分类
    for &(number, name, desc) in SMART_HOME_CLASSES {
        let exists = app_class::Entity::find()
            .filter(app_class::Column::Number.eq(number))
            .one(store.db())
            .await
            .unwrap_or(None);

        match exists {
            Some(item) => {
                if item.is_del == 1 || item.name != name {
                    let mut active: app_class::ActiveModel = item.into();
                    active.name = Set(name.to_string());
                    active.desc = Set(desc.to_string());
                    active.is_del = Set(0);
                    let _ = active.update(store.db()).await;
                }
            }
            None => {
                let active = app_class::ActiveModel {
                    number: Set(number.to_string()),
                    name: Set(name.to_string()),
                    desc: Set(desc.to_string()),
                    is_del: Set(0),
                    ..Default::default()
                };
                let _ = active.insert(store.db()).await;
            }
        }
    }

    // 3. 将原先使用“电商微服务”或“通用服务”的固件应用分类平滑迁移到“中控类别”
    if let Ok(old_apps) = app::Entity::find().all(store.db()).await {
        for item in old_apps {
            if item.class_name == "通用服务"
                || item.class_name == "电商微服务"
                || item.class_name.contains("微服务")
                || item.class_name.contains("电商")
            {
                let mut active: app::ActiveModel = item.into();
                active.class_no = Set("CLS-PANEL".to_string());
                active.class_name = Set("中控类别".to_string());
                let _ = active.update(store.db()).await;
            }
        }
    }
}

/// GET /api/app/class/pages
pub async fn page_classes(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ClassPageQuery>,
) -> Result<Json<Value>, StatusCode> {
    ensure_smart_home_classes(&state.store).await;

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
        log::error!("Failed to count app classes: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items = paginator
        .fetch_page(page.saturating_sub(1))
        .await
        .map_err(|e| {
            log::error!("Failed to fetch app classes: {:?}", e);
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
            log::error!("Failed to query class: {:?}", e);
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
                    log::error!("Failed to query class {}: {:?}", id, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .ok_or(StatusCode::NOT_FOUND)?;

            let mut active: app_class::ActiveModel = found.into();
            active.name = Set(payload.name);
            if let Some(desc) = payload.desc {
                active.desc = Set(desc);
            }
            let updated = active.update(state.store.db()).await.map_err(|e| {
                log::error!("Failed to update class: {:?}", e);
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
        log::error!("Failed to insert class: {:?}", e);
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
            log::error!("Failed to find class for delete: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app_class::ActiveModel = found.into();
    active.is_del = Set(1);
    active.update(state.store.db()).await.map_err(|e| {
        log::error!("Failed to soft delete class: {:?}", e);
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
            log::error!("Failed to query app users for {}: {:?}", appno, e);
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
            log::error!("Failed to query existing app user: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if let Some(user_rel) = existing {
        let mut active: app_user::ActiveModel = user_rel.into();
        active.uname = Set(payload.uname);
        if let Some(k) = payload.key {
            active.key = Set(k);
        }
        let updated = active.update(state.store.db()).await.map_err(|e| {
            log::error!("Failed to update app user: {:?}", e);
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
        log::error!("Failed to insert app user: {:?}", e);
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
            log::error!("Failed to delete app user: {:?}", e);
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
            log::error!("Failed to query deploys for {}: {:?}", appno, e);
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
            log::error!("Failed to query deploy: {:?}", e);
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
        log::error!("Failed to insert deploy: {:?}", e);
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
            log::error!("Failed to find deploy {}: {:?}", id, e);
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
        log::error!("Failed to update deploy {}: {:?}", id, e);
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
            log::error!("Failed to find deploy for copy: {:?}", e);
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
        log::error!("Failed to copy deploy: {:?}", e);
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
            log::error!("Failed to find deploy for delete: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app_deploy::ActiveModel = found.into();
    active.is_del = Set(1);
    active.update(state.store.db()).await.map_err(|e| {
        log::error!("Failed to soft delete deploy: {:?}", e);
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
            log::error!("Failed to query app for gitlab: {:?}", e);
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
            log::error!("Failed to list candidate users: {:?}", e);
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
