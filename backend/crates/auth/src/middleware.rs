use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use sha2::{Sha256, Digest};
use sqlx::PgPool;
use uuid::Uuid;
use crate::jwt::verify_token;
use clovalink_core::security_service;

/// Authenticated user context that gets inserted into request extensions
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role: String,
    pub email: String,
    pub ip_address: Option<String>,
}

/// Generate session fingerprint from request headers
/// Combines: User-Agent + Accept-Language + partial IP (first 3 octets)
fn generate_fingerprint(req: &Request, ip: Option<&str>) -> String {
    let user_agent = req.headers()
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    let accept_language = req.headers()
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    // Extract first 3 octets of IP (for privacy)
    let partial_ip = ip
        .map(|ip_str| {
            let parts: Vec<&str> = ip_str.split('.').take(3).collect();
            if parts.len() == 3 {
                parts.join(".")
            } else {
                // IPv6 or invalid - use first segment
                ip_str.split(':').next().unwrap_or("unknown").to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    
    let fingerprint_data = format!("{}|{}|{}", user_agent, accept_language, partial_ip);
    
    let mut hasher = Sha256::new();
    hasher.update(fingerprint_data.as_bytes());
    hex::encode(hasher.finalize())
}

/// Extract client IP address from request headers
/// Priority: X-Forwarded-For (first IP) > X-Real-IP > ConnectInfo
fn extract_client_ip(req: &Request) -> Option<String> {
    // Try X-Forwarded-For header first (common when behind proxy/load balancer)
    // Format: "client, proxy1, proxy2" - we want the first (original client) IP
    if let Some(forwarded_for) = req.headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
    {
        if let Some(first_ip) = forwarded_for.split(',').next() {
            let ip = first_ip.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }
    
    // Try X-Real-IP header (alternative)
    if let Some(real_ip) = req.headers()
        .get("x-real-ip")
        .and_then(|h| h.to_str().ok())
    {
        let ip = real_ip.trim();
        if !ip.is_empty() {
            return Some(ip.to_string());
        }
    }
    
    // Fallback: try to get from connection info extension (direct connection)
    // This requires axum's ConnectInfo extractor to be configured
    if let Some(connect_info) = req.extensions().get::<axum::extract::ConnectInfo<std::net::SocketAddr>>() {
        return Some(connect_info.0.ip().to_string());
    }
    
    None
}

/// Check if an IP address matches any entry in a list (supports CIDR notation)
fn ip_matches_any(ip: &str, list: &[String]) -> bool {
    use std::net::IpAddr;
    
    // Parse the client IP
    let client_ip: IpAddr = match ip.parse() {
        Ok(ip) => ip,
        Err(_) => return false, // Invalid IP, can't match
    };

    for entry in list {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        // Check if it's a CIDR notation (contains /)
        if entry.contains('/') {
            // Parse CIDR
            let parts: Vec<&str> = entry.split('/').collect();
            if parts.len() != 2 {
                continue;
            }
            
            let network_ip: IpAddr = match parts[0].parse() {
                Ok(ip) => ip,
                Err(_) => continue,
            };
            
            let prefix_len: u8 = match parts[1].parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Check if IPs are same type (both v4 or both v6)
            match (client_ip, network_ip) {
                (IpAddr::V4(client), IpAddr::V4(network)) => {
                    if prefix_len > 32 {
                        continue;
                    }
                    let mask = if prefix_len == 0 { 0 } else { !0u32 << (32 - prefix_len) };
                    let client_bits = u32::from(client);
                    let network_bits = u32::from(network);
                    if (client_bits & mask) == (network_bits & mask) {
                        return true;
                    }
                }
                (IpAddr::V6(client), IpAddr::V6(network)) => {
                    if prefix_len > 128 {
                        continue;
                    }
                    let client_bits = u128::from(client);
                    let network_bits = u128::from(network);
                    let mask = if prefix_len == 0 { 0 } else { !0u128 << (128 - prefix_len) };
                    if (client_bits & mask) == (network_bits & mask) {
                        return true;
                    }
                }
                _ => continue, // Type mismatch
            }
        } else {
            // Exact IP match
            let list_ip: IpAddr = match entry.parse() {
                Ok(ip) => ip,
                Err(_) => continue,
            };
            if client_ip == list_ip {
                return true;
            }
        }
    }
    
    false
}

/// Authentication middleware with database validation
/// Validates JWT token, checks user status (not suspended), and extracts user context
/// 
/// SECURITY: 
/// - Only accepts tokens from Authorization header (not URL params)
/// - Checks database to ensure user is not suspended
/// - Suspended users are immediately denied access even with valid JWT
pub async fn auth_middleware_with_db(
    State(pool): State<PgPool>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // SECURITY: Only accept tokens from Authorization header, NOT from URL query params
    let token = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| {
            // Check if they tried to use token in URL and give helpful error
            if req.uri().query().map(|q| q.contains("token=")).unwrap_or(false) {
                tracing::warn!(
                    "Rejected token-in-URL authentication attempt for path: {}",
                    req.uri().path()
                );
            }
            StatusCode::UNAUTHORIZED
        })?;

    // Decode and validate token using centralized logic
    let claims = verify_token(token).map_err(|e| {
        tracing::warn!("JWT decode error: {:?}", e);
        StatusCode::UNAUTHORIZED
    })?;

    // Parse UUIDs from string claims
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Extract client IP address early for security alerts
    let ip_address = extract_client_ip(&req);

    // SECURITY: Check if user is suspended or inactive in database
    // This ensures suspended users are kicked out immediately, not just on next login
    let user_status: Option<(String, Option<chrono::DateTime<chrono::Utc>>, String)> = sqlx::query_as(
        "SELECT status, suspended_at, email FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error checking user status: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match user_status {
        Some((status, suspended_at, email)) => {
            // Check if user is active
            if status != "active" {
                tracing::warn!("Rejected request from inactive user: {}", user_id);
                return Err(StatusCode::UNAUTHORIZED);
            }
            // Check if user is suspended
            if suspended_at.is_some() {
                tracing::warn!("Rejected request from suspended user: {}", user_id);
                // Create security alert for suspended user access attempt
                let _ = security_service::alert_suspended_access_attempt(
                    &pool,
                    tenant_id,
                    user_id,
                    &email,
                    req.uri().path(),
                    ip_address.as_deref(),
                ).await;
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        None => {
            // User doesn't exist
            tracing::warn!("Rejected request from non-existent user: {}", user_id);
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // SECURITY: Check if the session has been revoked
    // Hash the token to look up the session in the database
    let token_hash = {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    };

    let session_status: Option<(bool,)> = sqlx::query_as(
        "SELECT is_revoked FROM user_sessions WHERE token_hash = $1 AND user_id = $2 AND expires_at > NOW()"
    )
    .bind(&token_hash)
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error checking session status: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match session_status {
        Some((true,)) => {
            // Session has been revoked
            tracing::warn!("Rejected request from revoked session for user: {}", user_id);
            return Err(StatusCode::UNAUTHORIZED);
        }
        None => {
            // Session not found or expired - this can happen for older tokens
            // before session tracking was implemented, so we allow it
            tracing::debug!("Session not found in database for user: {}", user_id);
        }
        Some((false,)) => {
            // Session is valid, continue
        }
    }

    // SECURITY: Validate session fingerprint to detect token theft
    // If fingerprint is present in token, verify it matches current request
    // Note: Only log mismatches, don't block requests (fingerprint is for detection, not prevention)
    if let Some(ref expected_fingerprint) = claims.fingerprint {
        let current_fingerprint = generate_fingerprint(&req, ip_address.as_deref());
        if &current_fingerprint != expected_fingerprint {
            // Log at debug level to avoid log spam - fingerprint can vary due to 
            // browser updates, extension changes, or network changes
            tracing::debug!(
                "Fingerprint mismatch for user {}: expected {}, got {}",
                user_id,
                &expected_fingerprint[..8], // Log only first 8 chars for privacy
                &current_fingerprint[..8]
            );
            
            // Note: We intentionally don't create security alerts for every fingerprint mismatch
            // because legitimate causes include:
            // - Browser updates changing User-Agent
            // - Network changes (mobile -> wifi)
            // - VPN connections changing apparent IP
            // 
            // Instead, we rely on other signals (failed logins, unusual activity patterns)
            // for security alerting. The fingerprint is stored for forensic analysis if needed.
        }
    }

    // SECURITY: Check IP restrictions for tenant
    if let Some(ref client_ip) = ip_address {
        let ip_restrictions: Option<(String, Vec<String>, Vec<String>)> = sqlx::query_as(
            "SELECT ip_restriction_mode, ip_allowlist, ip_blocklist FROM tenants WHERE id = $1"
        )
        .bind(tenant_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking IP restrictions: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if let Some((mode, allowlist, blocklist)) = ip_restrictions {
            let is_blocked = match mode.as_str() {
                "allowlist_only" => {
                    // Only allow IPs in the allowlist
                    !allowlist.is_empty() && !ip_matches_any(client_ip, &allowlist)
                }
                "blocklist_only" => {
                    // Block IPs in the blocklist
                    ip_matches_any(client_ip, &blocklist)
                }
                "both" => {
                    // Must be in allowlist AND not in blocklist
                    let in_allowlist = allowlist.is_empty() || ip_matches_any(client_ip, &allowlist);
                    let in_blocklist = ip_matches_any(client_ip, &blocklist);
                    !in_allowlist || in_blocklist
                }
                _ => false, // "disabled" or unknown
            };

            if is_blocked {
                tracing::warn!(
                    "IP {} blocked by tenant {} restrictions (mode: {})",
                    client_ip, tenant_id, mode
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // Create AuthUser and insert into request extensions
    let auth_user = AuthUser {
        user_id,
        tenant_id,
        role: claims.role.clone(),
        email: String::new(), // Will be populated from DB if needed
        ip_address,
    };

    req.extensions_mut().insert(auth_user.clone());
    
    // Run the request
    let mut response = next.run(req).await;
    
    // Also add AuthUser to response extensions for outer middleware (like API usage tracking)
    response.extensions_mut().insert(auth_user);
    
    Ok(response)
}

/// Legacy authentication middleware (JWT only, no DB check)
/// Use auth_middleware_with_db instead for full security
/// 
/// SECURITY: Only accepts tokens from Authorization header.
/// Token-in-URL (?token=...) is NOT supported as it's a security risk.
pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // SECURITY: Only accept tokens from Authorization header, NOT from URL query params
    let token = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| {
            if req.uri().query().map(|q| q.contains("token=")).unwrap_or(false) {
                tracing::warn!(
                    "Rejected token-in-URL authentication attempt for path: {}",
                    req.uri().path()
                );
            }
            StatusCode::UNAUTHORIZED
        })?;

    let claims = verify_token(token).map_err(|e| {
        tracing::warn!("JWT decode error: {:?}", e);
        StatusCode::UNAUTHORIZED
    })?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Extract client IP address
    let ip_address = extract_client_ip(&req);

    let auth_user = AuthUser {
        user_id,
        tenant_id,
        role: claims.role.clone(),
        email: String::new(),
        ip_address,
    };

    req.extensions_mut().insert(auth_user.clone());
    
    // Run the request
    let mut response = next.run(req).await;
    
    // Also add AuthUser to response extensions for outer middleware
    response.extensions_mut().insert(auth_user);
    
    Ok(response)
}


/// Optional authentication middleware
/// Similar to auth_middleware but doesn't fail if no token is present
/// 
/// SECURITY: Only accepts tokens from Authorization header (same as auth_middleware)
pub async fn optional_auth_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    // SECURITY: Only accept tokens from Authorization header
    let token = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let mut auth_user_for_response: Option<AuthUser> = None;
    
    if let Some(token) = token {
        if let Ok(claims) = verify_token(&token) {
             if let (Ok(user_id), Ok(tenant_id)) = (
                Uuid::parse_str(&claims.sub),
                Uuid::parse_str(&claims.tenant_id)
            ) {
                // Extract client IP address
                let ip_address = extract_client_ip(&req);
                
                let auth_user = AuthUser {
                    user_id,
                    tenant_id,
                    role: claims.role.clone(),
                    email: String::new(),
                    ip_address,
                };
                auth_user_for_response = Some(auth_user.clone());
                req.extensions_mut().insert(auth_user);
            }
        }
    }
    
    let mut response = next.run(req).await;
    
    // Also add AuthUser to response extensions for outer middleware
    if let Some(auth_user) = auth_user_for_response {
        response.extensions_mut().insert(auth_user);
    }
    
    response
}

/// Helper function to check if user has required role
pub fn has_role(auth_user: &AuthUser, allowed_roles: &[&str]) -> bool {
    allowed_roles.contains(&auth_user.role.as_str())
}

/// Helper function to require SuperAdmin role
pub fn require_super_admin(auth_user: &AuthUser) -> Result<(), StatusCode> {
    if auth_user.role == "SuperAdmin" {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Helper function to require Admin or higher role
pub fn require_admin(auth_user: &AuthUser) -> Result<(), StatusCode> {
    if has_role(auth_user, &["SuperAdmin", "Admin"]) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Helper function to require Manager or higher role
pub fn require_manager(auth_user: &AuthUser) -> Result<(), StatusCode> {
    if has_role(auth_user, &["SuperAdmin", "Admin", "Manager"]) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}


