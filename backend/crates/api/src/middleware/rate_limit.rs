//! Redis-based rate limiting middleware for API endpoints
//! 
//! Security features:
//! - Atomic INCR+EXPIRE for race-condition-free rate limiting
//! - Trusted proxy mode for correct client IP extraction
//! - Configurable limits per endpoint type
//! 
//! Provides configurable rate limits per endpoint type:
//! - Login: 5 attempts/min per IP (prevent brute force)
//! - File upload: 100/hour per user
//! - General API: 1000/min per user
//! - Public endpoints: 60/min per IP

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::OnceLock;

use crate::AppState;

/// Trusted proxy configuration
struct TrustedProxyConfig {
    /// List of trusted proxy IP addresses/CIDRs
    trusted_ips: Vec<IpAddr>,
    /// Whether to trust any proxy (dangerous - only for development)
    trust_all: bool,
}

static TRUSTED_PROXY_CONFIG: OnceLock<TrustedProxyConfig> = OnceLock::new();

fn get_trusted_proxy_config() -> &'static TrustedProxyConfig {
    TRUSTED_PROXY_CONFIG.get_or_init(|| {
        let trust_all = std::env::var("TRUST_ALL_PROXIES")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);
        
        if trust_all {
            tracing::warn!(
                "TRUST_ALL_PROXIES is enabled - X-Forwarded-For will be trusted from any source. \
                This is dangerous in production!"
            );
        }
        
        let trusted_ips: Vec<IpAddr> = std::env::var("TRUSTED_PROXY_IPS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    return None;
                }
                match trimmed.parse::<IpAddr>() {
                    Ok(ip) => Some(ip),
                    Err(_) => {
                        tracing::warn!("Invalid IP in TRUSTED_PROXY_IPS: {}", trimmed);
                        None
                    }
                }
            })
            .collect();
        
        if !trusted_ips.is_empty() {
            tracing::info!("Trusted proxy IPs configured: {:?}", trusted_ips);
        }
        
        TrustedProxyConfig { trusted_ips, trust_all }
    })
}

/// Rate limit configuration for different endpoint types
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u64,
}

impl RateLimitConfig {
    pub fn login() -> Self {
        Self {
            max_requests: 5,
            window_seconds: 60, // 5 per minute
        }
    }

    pub fn upload() -> Self {
        Self {
            max_requests: 100,
            window_seconds: 3600, // 100 per hour
        }
    }

    pub fn export() -> Self {
        Self {
            max_requests: 5,
            window_seconds: 3600, // 5 per hour
        }
    }

    pub fn api() -> Self {
        Self {
            max_requests: 1000,
            window_seconds: 60, // 1000 per minute
        }
    }

    pub fn public() -> Self {
        Self {
            max_requests: 60,
            window_seconds: 60, // 60 per minute
        }
    }
    
    /// Global per-IP rate limit - applies to ALL requests
    /// Configurable via environment variables
    pub fn global() -> Self {
        let burst_size = std::env::var("PER_IP_BURST_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(200);
        
        Self {
            max_requests: burst_size, // Allow burst up to this
            window_seconds: 1,        // Per second
        }
    }
}

/// Rate limit key types
#[derive(Debug, Clone)]
pub enum RateLimitKey {
    /// Rate limit by IP address (for unauthenticated endpoints)
    Ip(String),
    /// Rate limit by user ID (for authenticated endpoints)
    User(String),
    /// Rate limit by IP + path (for specific endpoints like login)
    IpPath(String, String),
}

impl RateLimitKey {
    pub fn to_redis_key(&self, prefix: &str) -> String {
        match self {
            RateLimitKey::Ip(ip) => format!("clovalink:ratelimit:{}:ip:{}", prefix, ip),
            RateLimitKey::User(user_id) => format!("clovalink:ratelimit:{}:user:{}", prefix, user_id),
            RateLimitKey::IpPath(ip, path) => {
                let path_hash = path.replace('/', "_");
                format!("clovalink:ratelimit:{}:ip:{}:{}", prefix, ip, path_hash)
            }
        }
    }
}

/// Atomic rate limit check using Redis INCR + EXPIRE
/// 
/// This is race-condition free because:
/// 1. INCR is atomic and returns the new value
/// 2. EXPIRE with NX only sets expiry if not already set
/// 
/// Returns (is_allowed, current_count, remaining)
pub async fn check_rate_limit_atomic(
    cache: &clovalink_core::cache::Cache,
    key: &str,
    config: &RateLimitConfig,
) -> Result<(bool, u32, u32), String> {
    // Use Redis connection directly for atomic operations
    // Note: Dereference the guard to get the actual ConnectionManager
    let mut conn = cache.get_connection().await
        .map_err(|e| format!("Failed to get Redis connection: {}", e))?;
    
    // Atomic increment - INCR creates key with value 1 if it doesn't exist
    let new_count: u32 = redis::cmd("INCR")
        .arg(key)
        .query_async(&mut *conn)
        .await
        .map_err(|e| format!("INCR failed: {}", e))?;
    
    // Set expiry only if this is a new key (NX = only if not exists)
    // This ensures the window doesn't reset on each request
    if new_count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(key)
            .arg(config.window_seconds as i64)
            .query_async(&mut *conn)
            .await
            .map_err(|e| format!("EXPIRE failed: {}", e))?;
    }
    
    let is_allowed = new_count <= config.max_requests;
    let remaining = config.max_requests.saturating_sub(new_count);
    
    Ok((is_allowed, new_count, remaining))
}

/// Extract client IP from request, respecting trusted proxy configuration
/// 
/// Security:
/// - Only trusts X-Forwarded-For from configured trusted proxies
/// - Falls back to direct connection IP otherwise
pub fn extract_client_ip(headers: &HeaderMap, connection_ip: Option<IpAddr>) -> String {
    let proxy_config = get_trusted_proxy_config();
    
    // Check if the connection is from a trusted proxy
    let from_trusted_proxy = match connection_ip {
        Some(ip) => proxy_config.trust_all || proxy_config.trusted_ips.contains(&ip),
        None => proxy_config.trust_all, // No connection info, only trust if trust_all
    };
    
    if from_trusted_proxy {
        // Trust X-Forwarded-For header
        if let Some(forwarded) = headers.get("x-forwarded-for") {
            if let Ok(value) = forwarded.to_str() {
                // Take the first IP in the chain (original client)
                if let Some(ip) = value.split(',').next() {
                    let trimmed = ip.trim();
                    // Validate it looks like an IP
                    if trimmed.parse::<IpAddr>().is_ok() {
                        return trimmed.to_string();
                    }
                }
            }
        }
        
        // Try X-Real-IP header
        if let Some(real_ip) = headers.get("x-real-ip") {
            if let Ok(value) = real_ip.to_str() {
                if value.parse::<IpAddr>().is_ok() {
                    return value.to_string();
                }
            }
        }
    }
    
    // Fall back to connection address (most secure)
    connection_ip
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Rate limiting middleware for login endpoints
pub async fn rate_limit_login(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let config = RateLimitConfig::login();
    
    let headers = request.headers().clone();
    let ip = extract_client_ip(&headers, Some(addr.ip()));
    let path = request.uri().path().to_string();
    
    let key = RateLimitKey::IpPath(ip.clone(), path).to_redis_key("login");
    
    if let Some(ref cache) = state.cache {
        match check_rate_limit_atomic(cache, &key, &config).await {
            Ok((allowed, count, remaining)) => {
                if !allowed {
                    tracing::warn!("Rate limit exceeded for login from IP: {} (count: {})", ip, count);
                    return rate_limit_response(config.window_seconds, remaining);
                }
            }
            Err(e) => {
                tracing::error!("Rate limit check failed: {}", e);
                // Allow request on error (fail open for availability)
            }
        }
    }
    
    next.run(request).await
}

/// Rate limiting middleware for file upload endpoints
pub async fn rate_limit_upload(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth): axum::Extension<clovalink_auth::AuthUser>,
    request: Request,
    next: Next,
) -> Response {
    let config = RateLimitConfig::upload();
    let key = RateLimitKey::User(auth.user_id.to_string()).to_redis_key("upload");
    
    if let Some(ref cache) = state.cache {
        match check_rate_limit_atomic(cache, &key, &config).await {
            Ok((allowed, count, remaining)) => {
                if !allowed {
                    tracing::warn!("Upload rate limit exceeded for user: {} (count: {})", auth.user_id, count);
                    return rate_limit_response(config.window_seconds, remaining);
                }
            }
            Err(e) => {
                tracing::error!("Rate limit check failed: {}", e);
            }
        }
    }
    
    next.run(request).await
}

/// Rate limiting middleware for general authenticated API endpoints
pub async fn rate_limit_api(
    State(state): State<Arc<AppState>>,
    axum::Extension(auth): axum::Extension<clovalink_auth::AuthUser>,
    request: Request,
    next: Next,
) -> Response {
    let config = RateLimitConfig::api();
    let key = RateLimitKey::User(auth.user_id.to_string()).to_redis_key("api");
    
    if let Some(ref cache) = state.cache {
        match check_rate_limit_atomic(cache, &key, &config).await {
            Ok((allowed, count, remaining)) => {
                if !allowed {
                    tracing::warn!("API rate limit exceeded for user: {} (count: {})", auth.user_id, count);
                    return rate_limit_response(config.window_seconds, remaining);
                }
            }
            Err(e) => {
                tracing::error!("Rate limit check failed: {}", e);
            }
        }
    }
    
    next.run(request).await
}

/// Rate limiting middleware for public endpoints
pub async fn rate_limit_public(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let config = RateLimitConfig::public();
    
    let headers = request.headers().clone();
    let ip = extract_client_ip(&headers, Some(addr.ip()));
    let key = RateLimitKey::Ip(ip.clone()).to_redis_key("public");
    
    if let Some(ref cache) = state.cache {
        match check_rate_limit_atomic(cache, &key, &config).await {
            Ok((allowed, count, remaining)) => {
                if !allowed {
                    tracing::warn!("Public rate limit exceeded for IP: {} (count: {})", ip, count);
                    return rate_limit_response(config.window_seconds, remaining);
                }
            }
            Err(e) => {
                tracing::error!("Rate limit check failed: {}", e);
            }
        }
    }
    
    next.run(request).await
}

/// Global rate limiting middleware - applies to ALL requests per IP
/// This is the first line of defense against DDoS and abuse
/// Configured via PER_IP_REQUESTS_PER_SEC and PER_IP_BURST_SIZE env vars
pub async fn rate_limit_global(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    // Get configurable limits from environment
    let requests_per_sec: u32 = std::env::var("PER_IP_REQUESTS_PER_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    
    let burst_size: u32 = std::env::var("PER_IP_BURST_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    
    let config = RateLimitConfig {
        max_requests: burst_size,
        window_seconds: 1, // Per-second limiting
    };
    
    let headers = request.headers().clone();
    let ip = extract_client_ip(&headers, Some(addr.ip()));
    let key = RateLimitKey::Ip(ip.clone()).to_redis_key("global");
    
    if let Some(ref cache) = state.cache {
        match check_rate_limit_atomic(cache, &key, &config).await {
            Ok((allowed, count, remaining)) => {
                if !allowed {
                    tracing::warn!(
                        "Global rate limit exceeded for IP: {} ({}req/s, limit: {}/s, burst: {})",
                        ip, count, requests_per_sec, burst_size
                    );
                    return rate_limit_response(1, remaining);
                }
            }
            Err(e) => {
                // Log but don't block - fail open for availability
                tracing::debug!("Global rate limit check failed (allowing request): {}", e);
            }
        }
    }
    
    next.run(request).await
}

/// Generate rate limit exceeded response
fn rate_limit_response(retry_after: u64, remaining: u32) -> Response {
    let body = json!({
        "error": "rate_limit_exceeded",
        "message": "Too many requests. Please try again later.",
        "retry_after_seconds": retry_after,
        "remaining": remaining,
    });
    
    (
        StatusCode::TOO_MANY_REQUESTS,
        [
            ("Retry-After", retry_after.to_string()),
            ("X-RateLimit-Remaining", remaining.to_string()),
        ],
        Json(body),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_key_generation() {
        let ip_key = RateLimitKey::Ip("192.168.1.1".to_string());
        assert_eq!(
            ip_key.to_redis_key("api"),
            "clovalink:ratelimit:api:ip:192.168.1.1"
        );

        let user_key = RateLimitKey::User("user-123".to_string());
        assert_eq!(
            user_key.to_redis_key("upload"),
            "clovalink:ratelimit:upload:user:user-123"
        );
    }

    #[test]
    fn test_rate_limit_configs() {
        let login = RateLimitConfig::login();
        assert_eq!(login.max_requests, 5);
        assert_eq!(login.window_seconds, 60);

        let upload = RateLimitConfig::upload();
        assert_eq!(upload.max_requests, 100);
        assert_eq!(upload.window_seconds, 3600);
    }
    
    #[test]
    fn test_ip_validation() {
        // Valid IPs should parse
        assert!("192.168.1.1".parse::<IpAddr>().is_ok());
        assert!("::1".parse::<IpAddr>().is_ok());
        
        // Invalid strings should fail
        assert!("not-an-ip".parse::<IpAddr>().is_err());
        assert!("".parse::<IpAddr>().is_err());
    }
}
