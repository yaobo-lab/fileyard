//! API Usage Tracking Middleware
//!
//! Records request metrics for performance monitoring and analysis.
//! Uses sampling for high-traffic endpoints to reduce database load.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Metrics for a single API request
#[derive(Debug, Clone)]
pub struct ApiMetric {
    pub tenant_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub response_time_ms: u32,
    pub request_size_bytes: i64,
    pub response_size_bytes: i64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub error_message: Option<String>,
}

/// Batch writer for API metrics - writes metrics in batches to reduce DB load
pub struct ApiUsageWriter {
    sender: mpsc::Sender<ApiMetric>,
}

impl ApiUsageWriter {
    /// Create a new API usage writer with a background batch processor
    pub fn new(pool: PgPool) -> Self {
        let (sender, receiver) = mpsc::channel::<ApiMetric>(10000);
        
        // Spawn background task to batch write metrics
        tokio::spawn(batch_writer(pool, receiver));
        
        Self { sender }
    }
    
    /// Record a metric (non-blocking)
    pub fn record(&self, metric: ApiMetric) {
        // Use try_send to avoid blocking - if channel is full, drop the metric
        if let Err(e) = self.sender.try_send(metric) {
            tracing::warn!("API usage channel full, dropping metric: {:?}", e);
        }
    }
}

/// Background task that batches metrics and writes to database
async fn batch_writer(pool: PgPool, mut receiver: mpsc::Receiver<ApiMetric>) {
    let mut buffer = Vec::with_capacity(100);
    let mut last_flush = Instant::now();
    let flush_interval = std::time::Duration::from_secs(5);
    let batch_size = 100;
    
    loop {
        // Try to receive with timeout
        match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            receiver.recv()
        ).await {
            Ok(Some(metric)) => {
                buffer.push(metric);
                
                // Flush if buffer is full
                if buffer.len() >= batch_size {
                    flush_metrics(&pool, &mut buffer).await;
                    last_flush = Instant::now();
                }
            }
            Ok(None) => {
                // Channel closed, flush remaining and exit
                if !buffer.is_empty() {
                    flush_metrics(&pool, &mut buffer).await;
                }
                break;
            }
            Err(_) => {
                // Timeout - check if we should flush based on time
                if last_flush.elapsed() >= flush_interval && !buffer.is_empty() {
                    flush_metrics(&pool, &mut buffer).await;
                    last_flush = Instant::now();
                }
            }
        }
    }
}

/// Flush metrics to database
async fn flush_metrics(pool: &PgPool, buffer: &mut Vec<ApiMetric>) {
    if buffer.is_empty() {
        return;
    }
    
    // Build batch insert query
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO api_usage (tenant_id, user_id, endpoint, method, status_code, response_time_ms, request_size_bytes, response_size_bytes, ip_address, user_agent, error_message) "
    );
    
    query_builder.push_values(buffer.iter(), |mut b, metric| {
        b.push_bind(metric.tenant_id)
            .push_bind(metric.user_id)
            .push_bind(&metric.endpoint)
            .push_bind(&metric.method)
            .push_bind(metric.status_code as i32)
            .push_bind(metric.response_time_ms as i32)
            .push_bind(metric.request_size_bytes)
            .push_bind(metric.response_size_bytes)
            .push_bind(&metric.ip_address)
            .push_bind(&metric.user_agent)
            .push_bind(&metric.error_message);
    });
    
    match query_builder.build().execute(pool).await {
        Ok(_) => {
            tracing::debug!("Flushed {} API usage metrics to database", buffer.len());
        }
        Err(e) => {
            tracing::error!("Failed to write API usage metrics: {:?}", e);
        }
    }
    
    buffer.clear();
}

/// Extract user info from response extensions (after auth middleware has run)
fn extract_user_info_from_response(response: &Response) -> (Option<Uuid>, Option<Uuid>) {
    // Try to get AuthUser from response extensions (added by auth middleware after running)
    if let Some(auth) = response.extensions().get::<clovalink_auth::AuthUser>() {
        (Some(auth.tenant_id), Some(auth.user_id))
    } else {
        (None, None)
    }
}

/// Extract client IP from request
fn extract_ip(req: &Request) -> Option<String> {
    // Check common proxy headers
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            return Some(s.split(',').next().unwrap_or(s).trim().to_string());
        }
    }
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            return Some(s.to_string());
        }
    }
    None
}

/// Extract user agent from request
fn extract_user_agent(req: &Request) -> Option<String> {
    req.headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.chars().take(512).collect()) // Limit length
}

/// Normalize endpoint path - remove UUIDs and IDs for aggregation
fn normalize_endpoint(path: &str) -> String {
    // Replace UUIDs with placeholder
    let uuid_regex = regex::Regex::new(
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"
    ).unwrap();
    
    let normalized = uuid_regex.replace_all(path, "{id}");
    
    // Replace numeric IDs
    let id_regex = regex::Regex::new(r"/\d+(?:/|$)").unwrap();
    id_regex.replace_all(&normalized, "/{id}/").to_string()
}

/// Sampling configuration
struct SamplingConfig {
    /// Default sample rate (1.0 = 100%, 0.1 = 10%)
    default_rate: f64,
    /// Endpoints that should always be sampled (e.g., uploads)
    high_priority_endpoints: Vec<&'static str>,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            // Sample 10% by default (can be increased in low-traffic environments)
            default_rate: std::env::var("API_USAGE_SAMPLE_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.1),
            // Always track these important endpoints
            high_priority_endpoints: vec![
                "/api/upload",
                "/api/download",
                "/api/auth/login",
                "/api/auth/register",
                "/api/users",
                "/api/tenants",
                "/api/files",
            ],
        }
    }
}

/// Decide whether to sample this request
fn should_sample(endpoint: &str) -> bool {
    let config = SamplingConfig::default();
    
    // Always sample high-priority endpoints
    for prefix in &config.high_priority_endpoints {
        if endpoint.starts_with(prefix) {
            return true;
        }
    }
    
    // Sample based on rate
    if config.default_rate >= 1.0 {
        true
    } else {
        rand::random::<f64>() < config.default_rate
    }
}

/// State for the API usage middleware
#[derive(Clone)]
pub struct ApiUsageState {
    pub writer: Arc<ApiUsageWriter>,
}

/// Middleware that tracks API usage metrics
/// 
/// Note: This middleware runs BEFORE auth middleware in the tower stack,
/// but we extract auth info from RESPONSE extensions after the inner handlers complete.
pub async fn api_usage_middleware(
    State(state): State<ApiUsageState>,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let normalized_endpoint = normalize_endpoint(&path);
    
    // Check if we should sample this request
    if !should_sample(&normalized_endpoint) {
        return next.run(req).await;
    }
    
    let ip_address = extract_ip(&req);
    let user_agent = extract_user_agent(&req);
    
    // Get request size from content-length header
    let request_size = req
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    
    // Run the actual request (auth middleware will add AuthUser to response extensions)
    let response = next.run(req).await;
    
    // Extract user info from response extensions (added by auth middleware)
    let (tenant_id, user_id) = extract_user_info_from_response(&response);
    
    let elapsed = start.elapsed();
    let status_code = response.status().as_u16();
    
    // Get response size from content-length header
    let response_size = response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    
    // Generate error message for error responses
    let error_message = if status_code >= 400 {
        // Map common status codes to human-readable messages
        Some(match status_code {
            400 => "Bad Request",
            401 => "Unauthorized - Invalid or missing authentication",
            403 => "Forbidden - Access denied",
            404 => "Not Found",
            405 => "Method Not Allowed",
            408 => "Request Timeout",
            409 => "Conflict",
            413 => "Payload Too Large",
            415 => "Unsupported Media Type",
            422 => "Unprocessable Entity - Validation failed",
            429 => "Too Many Requests - Rate limited",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            _ => "Unknown Error",
        }.to_string())
    } else {
        None
    };
    
    // Record the metric
    state.writer.record(ApiMetric {
        tenant_id,
        user_id,
        endpoint: normalized_endpoint,
        method,
        status_code,
        response_time_ms: elapsed.as_millis() as u32,
        request_size_bytes: request_size,
        response_size_bytes: response_size,
        ip_address,
        user_agent,
        error_message,
    });
    
    response
}

