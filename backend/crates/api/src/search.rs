use crate::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use clovalink_auth::AuthUser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub result_type: String,
    pub link: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub companies: Vec<SearchResult>,
    pub users: Vec<SearchResult>,
    pub files: Vec<SearchResult>,
    pub groups: Vec<SearchResult>,
    pub total: i64,
}

/// Global search across companies, users, files, and groups
/// GET /api/search?q=query
pub async fn global_search(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, StatusCode> {
    let query = params.q.trim();
    if query.is_empty() || query.len() < 2 {
        return Ok(Json(json!(SearchResponse {
            companies: vec![],
            users: vec![],
            files: vec![],
            groups: vec![],
            total: 0,
        })));
    }

    let limit = params.limit.unwrap_or(5).min(20);
    let search_pattern = format!("%{}%", query.to_lowercase());

    let bundle = state
        .store
        .search()
        .global(
            auth.tenant_id,
            auth.user_id,
            &auth.role,
            &search_pattern,
            limit,
        )
        .await
        .map_err(|e| {
            tracing::error!("Search failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let company_results: Vec<SearchResult> = bundle
        .companies
        .into_iter()
        .map(|c| SearchResult {
            id: c.id.to_string(),
            name: c.name.clone(),
            description: Some(c.domain),
            result_type: "company".to_string(),
            link: format!("/companies/{}", urlencoding::encode(&c.name)),
        })
        .collect();

    let user_results: Vec<SearchResult> = bundle
        .users
        .into_iter()
        .map(|u| SearchResult {
            id: u.id.to_string(),
            name: u.name.clone(),
            description: Some(u.email),
            result_type: "user".to_string(),
            link: format!("/users?id={}", u.id),
        })
        .collect();

    let file_results: Vec<SearchResult> = bundle
        .files
        .into_iter()
        .map(|f| {
            let parent = f.parent_path.as_deref().unwrap_or("");
            let link_path = if f.is_directory {
                if parent.is_empty() || parent == "/" {
                    format!("/{}", f.name)
                } else {
                    format!("{}/{}", parent, f.name)
                }
            } else {
                parent.to_string()
            };
            SearchResult {
                id: f.id.to_string(),
                name: f.name.clone(),
                description: f.parent_path.clone(),
                result_type: if f.is_directory { "folder" } else { "file" }.to_string(),
                link: format!("/files?path={}", urlencoding::encode(&link_path)),
            }
        })
        .collect();

    let group_results: Vec<SearchResult> = bundle
        .groups
        .into_iter()
        .map(|g| {
            let parent = g.parent_path.as_deref().unwrap_or("");
            let link = if parent.is_empty() {
                format!("/files?group={}", g.id)
            } else {
                format!("/files?path={}&group={}", urlencoding::encode(parent), g.id)
            };
            SearchResult {
                id: g.id.to_string(),
                name: g.name.clone(),
                description: g.description.or(Some(format!(
                    "in {}",
                    if parent.is_empty() { "Home" } else { parent }
                ))),
                result_type: "group".to_string(),
                link,
            }
        })
        .collect();

    let total = (company_results.len()
        + user_results.len()
        + file_results.len()
        + group_results.len()) as i64;

    Ok(Json(json!(SearchResponse {
        companies: company_results,
        users: user_results,
        files: file_results,
        groups: group_results,
        total,
    })))
}
