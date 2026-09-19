//! 通用配置管理 API 模块 (app_config)

use crate::AppState;
use app_entity::entities::app_config;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;

use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ConfigPageQuery {
    pub page: Option<u64>,
    pub limit: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
    #[serde(rename = "curPage")]
    pub cur_page: Option<u64>,
    pub key: Option<String>,
    pub name: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigIdQuery {
    pub id: i32,
}

#[derive(Debug, Deserialize)]
pub struct SaveConfigPayload {
    pub id: Option<i32>,
    pub number: Option<String>,
    pub name: String,
    pub key: String,
    pub value: Option<String>,
    pub remark: Option<String>,
}

/// GET /api/config/page
pub async fn page_configs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ConfigPageQuery>,
) -> Result<Json<Value>, StatusCode> {
    let page = params.cur_page.or(params.page).unwrap_or(1);
    let page_size = params.page_size.or(params.limit).unwrap_or(10);

    let mut query = app_config::Entity::find().filter(app_config::Column::IsDel.eq(0));

    if let Some(ref search) = params.search {
        let s = search.trim();
        if !s.is_empty() {
            query = query.filter(
                app_config::Column::Name
                    .contains(s)
                    .or(app_config::Column::Key.contains(s))
                    .or(app_config::Column::Number.contains(s)),
            );
        }
    }

    if let Some(ref name) = params.name {
        let n = name.trim();
        if !n.is_empty() {
            query = query.filter(app_config::Column::Name.contains(n));
        }
    }

    if let Some(ref key) = params.key {
        let k = key.trim();
        if !k.is_empty() {
            query = query.filter(app_config::Column::Key.contains(k));
        }
    }

    let paginator = query
        .order_by_desc(app_config::Column::Id)
        .paginate(state.store.db(), page_size);

    let total = paginator.num_items().await.map_err(|e| {
        tracing::error!("Failed to count configs: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items = paginator.fetch_page(page.saturating_sub(1)).await.map_err(|e| {
        tracing::error!("Failed to fetch config page: {:?}", e);
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

/// GET /api/config?id=xx
pub async fn get_config(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ConfigIdQuery>,
) -> Result<Json<Value>, StatusCode> {
    let config = app_config::Entity::find_by_id(params.id)
        .filter(app_config::Column::IsDel.eq(0))
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to get config: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(json!({
        "code": 200,
        "message": "获取成功",
        "data": config
    })))
}

/// POST /api/config
pub async fn save_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SaveConfigPayload>,
) -> Result<Json<Value>, StatusCode> {
    let now = Utc::now().into();

    if let Some(id) = payload.id {
        if id > 0 {
            // Update existing
            let existing = app_config::Entity::find_by_id(id)
                .one(state.store.db())
                .await
                .map_err(|e| {
                    tracing::error!("Failed to find config {}: {:?}", id, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .ok_or(StatusCode::NOT_FOUND)?;

            let mut active: app_config::ActiveModel = existing.into();
            active.name = Set(payload.name);
            active.key = Set(payload.key);
            if let Some(val) = payload.value {
                active.value = Set(val);
            }
            if let Some(remark) = payload.remark {
                active.remark = Set(remark);
            }

            let saved = active.update(state.store.db()).await.map_err(|e| {
                tracing::error!("Failed to update config {}: {:?}", id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            return Ok(Json(json!({
                "code": 200,
                "message": "保存成功",
                "data": saved
            })));
        }
    }

    // Insert new config
    let number = payload.number.unwrap_or_else(|| {
        format!("CFG-{}", nanoid::nanoid!(8).to_uppercase())
    });

    let active = app_config::ActiveModel {
        number: Set(number),
        name: Set(payload.name),
        key: Set(payload.key),
        value: Set(payload.value.unwrap_or_default()),
        create_time: Set(now),
        remark: Set(payload.remark.unwrap_or_default()),
        is_del: Set(0),
        ..Default::default()
    };

    let saved = active.insert(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to insert config: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "创建成功",
        "data": saved
    })))
}

/// DELETE /api/config?id=xx
pub async fn delete_config(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ConfigIdQuery>,
) -> Result<Json<Value>, StatusCode> {
    let existing = app_config::Entity::find_by_id(params.id)
        .one(state.store.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to find config for delete {}: {:?}", params.id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut active: app_config::ActiveModel = existing.into();
    active.is_del = Set(1);
    active.update(state.store.db()).await.map_err(|e| {
        tracing::error!("Failed to soft delete config: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "code": 200,
        "message": "删除成功"
    })))
}

/// GET /api/config/{id}
pub async fn get_config_by_path(
    state: State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<Value>, StatusCode> {
    get_config(state, Query(ConfigIdQuery { id })).await
}

/// DELETE /api/config/{id}
pub async fn delete_config_by_path(
    state: State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<Value>, StatusCode> {
    delete_config(state, Query(ConfigIdQuery { id })).await
}
