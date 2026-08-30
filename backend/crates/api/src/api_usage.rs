//! API Usage Query Endpoints
//!
//! Provides endpoints for SuperAdmins to query API usage statistics.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use std::sync::Arc;
use uuid::Uuid;

use clovalink_auth::AuthUser;
use crate::AppState;

/// Time range for usage queries
#[derive(Debug, Deserialize)]
pub struct UsageQueryParams {
    /// Start time (ISO 8601 format)
    pub from: Option<DateTime<Utc>>,
    /// End time (ISO 8601 format)
    pub to: Option<DateTime<Utc>>,
    /// Filter by tenant ID
    pub tenant_id: Option<Uuid>,
    /// Time granularity: hour, day, week
    pub granularity: Option<String>,
}

impl UsageQueryParams {
    fn get_time_range(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let to = self.to.unwrap_or_else(Utc::now);
        let from = self.from.unwrap_or_else(|| to - chrono::Duration::days(1));
        (from, to)
    }
}

/// Overall usage summary
#[derive(Debug, Serialize)]
pub struct UsageSummary {
    pub total_requests: i64,
    pub total_errors: i64,
    pub error_rate: f64,
    pub avg_response_time_ms: f64,
    pub total_request_bytes: i64,
    pub total_response_bytes: i64,
    pub unique_users: i64,
    pub unique_tenants: i64,
    pub requests_per_minute: f64,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// GET /api/admin/usage/summary
/// Overall API usage statistics
pub async fn get_usage_summary(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<UsageSummary>, StatusCode> {
    // Only SuperAdmin can access
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let (from, to) = params.get_time_range();
    let duration_minutes = (to - from).num_minutes().max(1) as f64;

    #[derive(FromRow)]
    struct Stats {
        total_requests: Option<i64>,
        total_errors: Option<i64>,
        avg_response_time: Option<f64>,
        total_request_bytes: Option<i64>,
        total_response_bytes: Option<i64>,
        unique_users: Option<i64>,
        unique_tenants: Option<i64>,
    }

    let stats = if params.tenant_id.is_some() {
        sqlx::query_as::<_, Stats>(r#"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(*) FILTER (WHERE status_code >= 400) as total_errors,
                AVG(response_time_ms)::FLOAT8 as avg_response_time,
                COALESCE(SUM(request_size_bytes), 0)::BIGINT as total_request_bytes,
                COALESCE(SUM(response_size_bytes), 0)::BIGINT as total_response_bytes,
                COUNT(DISTINCT user_id) as unique_users,
                COUNT(DISTINCT tenant_id) as unique_tenants
            FROM api_usage
            WHERE created_at >= $1 AND created_at <= $2 AND tenant_id = $3
        "#)
        .bind(from)
        .bind(to)
        .bind(params.tenant_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get usage summary: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        sqlx::query_as::<_, Stats>(r#"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(*) FILTER (WHERE status_code >= 400) as total_errors,
                AVG(response_time_ms)::FLOAT8 as avg_response_time,
                COALESCE(SUM(request_size_bytes), 0)::BIGINT as total_request_bytes,
                COALESCE(SUM(response_size_bytes), 0)::BIGINT as total_response_bytes,
                COUNT(DISTINCT user_id) as unique_users,
                COUNT(DISTINCT tenant_id) as unique_tenants
            FROM api_usage
            WHERE created_at >= $1 AND created_at <= $2
        "#)
        .bind(from)
        .bind(to)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get usage summary: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    let total_requests = stats.total_requests.unwrap_or(0);
    let total_errors = stats.total_errors.unwrap_or(0);

    Ok(Json(UsageSummary {
        total_requests,
        total_errors,
        error_rate: if total_requests > 0 {
            (total_errors as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        },
        avg_response_time_ms: stats.avg_response_time.unwrap_or(0.0),
        total_request_bytes: stats.total_request_bytes.unwrap_or(0),
        total_response_bytes: stats.total_response_bytes.unwrap_or(0),
        unique_users: stats.unique_users.unwrap_or(0),
        unique_tenants: stats.unique_tenants.unwrap_or(0),
        requests_per_minute: total_requests as f64 / duration_minutes,
        from,
        to,
    }))
}

/// Usage by tenant
#[derive(Debug, Serialize, FromRow)]
pub struct TenantUsage {
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub category: String,  // "tenant", "unauthenticated", or "unknown"
    pub request_count: i64,
    pub error_count: i64,
    pub avg_response_time_ms: f64,
    pub total_bytes: i64,
}

/// GET /api/admin/usage/by-tenant
/// Usage breakdown by tenant
/// Categorizes null tenant_ids as either "unauthenticated" (for public endpoints)
/// or "unknown" (for requests that should have had a tenant)
pub async fn get_usage_by_tenant(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<Vec<TenantUsage>>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let (from, to) = params.get_time_range();

    // Query with categorization for null tenant_ids
    // Public endpoints: /api/auth/login, /api/auth/register, /api/auth/forgot-password, 
    // /api/auth/reset-password, /health, /health/ready, /api/public/*
    let tenants = sqlx::query_as::<_, TenantUsage>(r#"
        WITH categorized AS (
            SELECT 
                u.tenant_id,
                t.name as tenant_name,
                CASE 
                    WHEN u.tenant_id IS NOT NULL THEN 'tenant'
                    WHEN u.endpoint LIKE '/api/auth/login%' 
                      OR u.endpoint LIKE '/api/auth/register%'
                      OR u.endpoint LIKE '/api/auth/forgot-password%'
                      OR u.endpoint LIKE '/api/auth/reset-password%'
                      OR u.endpoint LIKE '/health%'
                      OR u.endpoint LIKE '/api/public%'
                      OR u.endpoint = '/'
                    THEN 'unauthenticated'
                    ELSE 'unknown'
                END as category,
                u.status_code,
                u.response_time_ms,
                u.request_size_bytes,
                u.response_size_bytes
            FROM api_usage u
            LEFT JOIN tenants t ON t.id = u.tenant_id
            WHERE u.created_at >= $1 AND u.created_at <= $2
        )
        SELECT 
            tenant_id,
            CASE 
                WHEN category = 'tenant' THEN tenant_name
                WHEN category = 'unauthenticated' THEN 'Unauthenticated'
                ELSE 'Unknown'
            END as tenant_name,
            category,
            COUNT(*) as request_count,
            COUNT(*) FILTER (WHERE status_code >= 400) as error_count,
            AVG(response_time_ms)::FLOAT8 as avg_response_time_ms,
            COALESCE(SUM(request_size_bytes + response_size_bytes), 0)::BIGINT as total_bytes
        FROM categorized
        GROUP BY tenant_id, tenant_name, category
        ORDER BY request_count DESC
        LIMIT 50
    "#)
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get usage by tenant: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(tenants))
}

/// Usage by user within a tenant
#[derive(Debug, Serialize, FromRow)]
pub struct UserUsage {
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_response_time_ms: f64,
}

/// GET /api/admin/usage/by-user
/// Usage breakdown by user (optionally filtered by tenant)
pub async fn get_usage_by_user(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<Vec<UserUsage>>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let (from, to) = params.get_time_range();

    let users = if let Some(tenant_id) = params.tenant_id {
        sqlx::query_as::<_, UserUsage>(r#"
            SELECT 
                u.user_id,
                usr.name as user_name,
                usr.email as user_email,
                COUNT(*) as request_count,
                COUNT(*) FILTER (WHERE u.status_code >= 400) as error_count,
                AVG(u.response_time_ms)::FLOAT8 as avg_response_time_ms
            FROM api_usage u
            LEFT JOIN users usr ON usr.id = u.user_id
            WHERE u.created_at >= $1 AND u.created_at <= $2 AND u.tenant_id = $3
            GROUP BY u.user_id, usr.name, usr.email
            ORDER BY request_count DESC
            LIMIT 50
        "#)
        .bind(from)
        .bind(to)
        .bind(tenant_id)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, UserUsage>(r#"
            SELECT 
                u.user_id,
                usr.name as user_name,
                usr.email as user_email,
                COUNT(*) as request_count,
                COUNT(*) FILTER (WHERE u.status_code >= 400) as error_count,
                AVG(u.response_time_ms)::FLOAT8 as avg_response_time_ms
            FROM api_usage u
            LEFT JOIN users usr ON usr.id = u.user_id
            WHERE u.created_at >= $1 AND u.created_at <= $2
            GROUP BY u.user_id, usr.name, usr.email
            ORDER BY request_count DESC
            LIMIT 50
        "#)
        .bind(from)
        .bind(to)
        .fetch_all(&state.pool)
        .await
    }.map_err(|e| {
        tracing::error!("Failed to get usage by user: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(users))
}

/// Usage by endpoint
#[derive(Debug, Serialize, FromRow)]
pub struct EndpointUsage {
    pub endpoint: String,
    pub method: String,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_response_time_ms: f64,
    pub p95_response_time_ms: Option<f64>,
}

/// GET /api/admin/usage/by-endpoint
/// Usage breakdown by endpoint
pub async fn get_usage_by_endpoint(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<Vec<EndpointUsage>>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let (from, to) = params.get_time_range();

    let endpoints = if let Some(tenant_id) = params.tenant_id {
        sqlx::query_as::<_, EndpointUsage>(r#"
            SELECT 
                endpoint,
                method,
                COUNT(*) as request_count,
                COUNT(*) FILTER (WHERE status_code >= 400) as error_count,
                AVG(response_time_ms)::FLOAT8 as avg_response_time_ms,
                PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY response_time_ms)::FLOAT8 as p95_response_time_ms
            FROM api_usage
            WHERE created_at >= $1 AND created_at <= $2 AND tenant_id = $3
            GROUP BY endpoint, method
            ORDER BY request_count DESC
            LIMIT 50
        "#)
        .bind(from)
        .bind(to)
        .bind(tenant_id)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, EndpointUsage>(r#"
            SELECT 
                endpoint,
                method,
                COUNT(*) as request_count,
                COUNT(*) FILTER (WHERE status_code >= 400) as error_count,
                AVG(response_time_ms)::FLOAT8 as avg_response_time_ms,
                PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY response_time_ms)::FLOAT8 as p95_response_time_ms
            FROM api_usage
            WHERE created_at >= $1 AND created_at <= $2
            GROUP BY endpoint, method
            ORDER BY request_count DESC
            LIMIT 50
        "#)
        .bind(from)
        .bind(to)
        .fetch_all(&state.pool)
        .await
    }.map_err(|e| {
        tracing::error!("Failed to get usage by endpoint: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(endpoints))
}

/// Slow requests
#[derive(Debug, Serialize, FromRow)]
pub struct SlowRequest {
    pub endpoint: String,
    pub method: String,
    pub avg_response_time_ms: f64,
    pub max_response_time_ms: i32,
    pub request_count: i64,
    pub error_rate: f64,
}

/// GET /api/admin/usage/slow-requests
/// Endpoints with slowest average response times
pub async fn get_slow_requests(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<Vec<SlowRequest>>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let (from, to) = params.get_time_range();

    let slow = sqlx::query_as::<_, SlowRequest>(r#"
        SELECT 
            endpoint,
            method,
            AVG(response_time_ms)::FLOAT8 as avg_response_time_ms,
            MAX(response_time_ms) as max_response_time_ms,
            COUNT(*) as request_count,
            (COUNT(*) FILTER (WHERE status_code >= 400)::FLOAT8 / COUNT(*)::FLOAT8 * 100) as error_rate
        FROM api_usage
        WHERE created_at >= $1 AND created_at <= $2
        GROUP BY endpoint, method
        HAVING COUNT(*) >= 10  -- Only include endpoints with enough samples
        ORDER BY avg_response_time_ms DESC
        LIMIT 20
    "#)
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get slow requests: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(slow))
}

/// Time series data point
#[derive(Debug, Serialize, FromRow)]
pub struct TimeSeriesPoint {
    pub time_bucket: DateTime<Utc>,
    pub request_count: i64,
    pub error_count: i64,
    pub avg_response_time_ms: f64,
}

/// GET /api/admin/usage/timeseries
/// Time series of requests over time
pub async fn get_usage_timeseries(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<Vec<TimeSeriesPoint>>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let (from, to) = params.get_time_range();
    let granularity = params.granularity.as_deref().unwrap_or("hour");
    
    let _interval = match granularity {
        "minute" => "1 minute",
        "hour" => "1 hour",
        "day" => "1 day",
        _ => "1 hour",
    };

    let query = format!(r#"
        SELECT 
            date_trunc('{}', created_at) as time_bucket,
            COUNT(*) as request_count,
            COUNT(*) FILTER (WHERE status_code >= 400) as error_count,
            AVG(response_time_ms)::FLOAT8 as avg_response_time_ms
        FROM api_usage
        WHERE created_at >= $1 AND created_at <= $2
        {}
        GROUP BY time_bucket
        ORDER BY time_bucket ASC
    "#, 
        granularity,
        if params.tenant_id.is_some() { "AND tenant_id = $3" } else { "" }
    );

    let series = if let Some(tenant_id) = params.tenant_id {
        sqlx::query_as::<_, TimeSeriesPoint>(&query)
            .bind(from)
            .bind(to)
            .bind(tenant_id)
            .fetch_all(&state.pool)
            .await
    } else {
        sqlx::query_as::<_, TimeSeriesPoint>(&query)
            .bind(from)
            .bind(to)
            .fetch_all(&state.pool)
            .await
    }.map_err(|e| {
        tracing::error!("Failed to get usage timeseries: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(series))
}

/// Aggregate hourly stats (called by cron)
/// POST /api/admin/usage/aggregate
pub async fn aggregate_hourly_stats(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    match sqlx::query("SELECT aggregate_api_usage_hourly()")
        .execute(&state.pool)
        .await
    {
        Ok(_) => {
            tracing::info!("API usage hourly aggregation completed");
            Ok(Json(json!({ "success": true, "message": "Hourly aggregation completed" })))
        }
        Err(e) => {
            tracing::error!("Failed to aggregate hourly stats: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Cleanup old raw data (called by cron)
/// POST /api/admin/usage/cleanup
pub async fn cleanup_old_usage(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    match sqlx::query("SELECT cleanup_old_api_usage()")
        .execute(&state.pool)
        .await
    {
        Ok(_) => {
            tracing::info!("API usage cleanup completed");
            Ok(Json(json!({ "success": true, "message": "Cleanup completed" })))
        }
        Err(e) => {
            tracing::error!("Failed to cleanup old usage: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Query params for errors endpoint with pagination
#[derive(Debug, Deserialize)]
pub struct ErrorQueryParams {
    /// Start time (ISO 8601 format)
    pub from: Option<DateTime<Utc>>,
    /// End time (ISO 8601 format)
    pub to: Option<DateTime<Utc>>,
    /// Filter by status code (e.g., 401, 500)
    pub status_code: Option<i32>,
    /// Page number (1-indexed)
    pub page: Option<i64>,
    /// Items per page (default 20, max 100)
    pub per_page: Option<i64>,
}

impl ErrorQueryParams {
    fn get_time_range(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let to = self.to.unwrap_or_else(Utc::now);
        let from = self.from.unwrap_or_else(|| to - chrono::Duration::days(1));
        (from, to)
    }
    
    fn get_pagination(&self) -> (i64, i64) {
        let per_page = self.per_page.unwrap_or(20).min(100).max(1);
        let page = self.page.unwrap_or(1).max(1);
        let offset = (page - 1) * per_page;
        (per_page, offset)
    }
}

/// Recent error details
#[derive(Debug, Serialize, FromRow)]
pub struct ErrorDetail {
    pub id: Uuid,
    pub endpoint: String,
    pub method: String,
    pub status_code: i32,
    pub error_message: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub tenant_name: Option<String>,
    pub user_id: Option<Uuid>,
    pub user_email: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub response_time_ms: i32,
}

/// Paginated error response
#[derive(Debug, Serialize)]
pub struct PaginatedErrors {
    pub errors: Vec<ErrorDetail>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// GET /api/admin/usage/errors
/// Recent error requests with details and pagination
pub async fn get_recent_errors(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<ErrorQueryParams>,
) -> Result<Json<PaginatedErrors>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let (from, to) = params.get_time_range();
    let (per_page, offset) = params.get_pagination();
    let page = params.page.unwrap_or(1).max(1);

    // Get total count
    let total: i64 = if let Some(status_code) = params.status_code {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM api_usage WHERE created_at >= $1 AND created_at <= $2 AND status_code = $3"
        )
        .bind(from)
        .bind(to)
        .bind(status_code)
        .fetch_one(&state.pool)
        .await
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM api_usage WHERE created_at >= $1 AND created_at <= $2 AND status_code >= 400"
        )
        .bind(from)
        .bind(to)
        .fetch_one(&state.pool)
        .await
    }.map_err(|e| {
        tracing::error!("Failed to get error count: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get paginated errors
    let errors = if let Some(status_code) = params.status_code {
        sqlx::query_as::<_, ErrorDetail>(r#"
            SELECT 
                u.id,
                u.endpoint,
                u.method,
                u.status_code,
                u.error_message,
                u.tenant_id,
                t.name as tenant_name,
                u.user_id,
                usr.email as user_email,
                u.ip_address,
                u.created_at,
                u.response_time_ms
            FROM api_usage u
            LEFT JOIN tenants t ON t.id = u.tenant_id
            LEFT JOIN users usr ON usr.id = u.user_id
            WHERE u.created_at >= $1 AND u.created_at <= $2
              AND u.status_code = $3
            ORDER BY u.created_at DESC
            LIMIT $4 OFFSET $5
        "#)
        .bind(from)
        .bind(to)
        .bind(status_code)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, ErrorDetail>(r#"
            SELECT 
                u.id,
                u.endpoint,
                u.method,
                u.status_code,
                u.error_message,
                u.tenant_id,
                t.name as tenant_name,
                u.user_id,
                usr.email as user_email,
                u.ip_address,
                u.created_at,
                u.response_time_ms
            FROM api_usage u
            LEFT JOIN tenants t ON t.id = u.tenant_id
            LEFT JOIN users usr ON usr.id = u.user_id
            WHERE u.created_at >= $1 AND u.created_at <= $2
              AND u.status_code >= 400
            ORDER BY u.created_at DESC
            LIMIT $3 OFFSET $4
        "#)
        .bind(from)
        .bind(to)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
    }.map_err(|e| {
        tracing::error!("Failed to get recent errors: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(PaginatedErrors {
        errors,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// Error summary by status code
#[derive(Debug, Serialize, FromRow)]
pub struct ErrorSummary {
    pub status_code: i32,
    pub error_count: i64,
    pub last_occurrence: DateTime<Utc>,
    pub most_common_endpoint: Option<String>,
}

/// GET /api/admin/usage/error-summary
/// Summary of errors grouped by status code
pub async fn get_error_summary(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(params): Query<UsageQueryParams>,
) -> Result<Json<Vec<ErrorSummary>>, StatusCode> {
    if auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let (from, to) = params.get_time_range();

    let summary = sqlx::query_as::<_, ErrorSummary>(r#"
        WITH error_counts AS (
            SELECT 
                status_code,
                COUNT(*) as error_count,
                MAX(created_at) as last_occurrence,
                endpoint,
                ROW_NUMBER() OVER (PARTITION BY status_code ORDER BY COUNT(*) DESC) as rn
            FROM api_usage
            WHERE created_at >= $1 AND created_at <= $2
              AND status_code >= 400
            GROUP BY status_code, endpoint
        )
        SELECT 
            status_code,
            SUM(error_count)::BIGINT as error_count,
            MAX(last_occurrence) as last_occurrence,
            MAX(CASE WHEN rn = 1 THEN endpoint END) as most_common_endpoint
        FROM error_counts
        GROUP BY status_code
        ORDER BY error_count DESC
    "#)
    .bind(from)
    .bind(to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get error summary: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(summary))
}

