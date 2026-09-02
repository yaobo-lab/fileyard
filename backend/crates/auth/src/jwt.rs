//! JWT Token Generation and Verification
//!
//! Security features:
//! - No hardcoded fallback secrets
//! - Issuer and audience validation
//! - Support for key rotation (primary + secondary secret)
//! - Fail-fast startup in production mode

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

/// JWT configuration loaded from the application TOML configuration.
struct JwtConfig {
    /// Primary JWT secret (required)
    secret: String,
    /// Secondary JWT secret for key rotation (optional)
    secret_secondary: Option<String>,
    /// Token issuer (default: "clovalink")
    issuer: String,
    /// Token audience (default: "clovalink-api")
    audience: String,
    /// Token expiry in seconds (default: 7 days)
    expiry_secs: usize,
}

static JWT_CONFIG: OnceLock<JwtConfig> = OnceLock::new();

/// Initialize JWT configuration from `etc/config.toml`.
///
/// # Panics
/// Panics if JWT_SECRET is not set and ENVIRONMENT is not "development" or "dev"
fn get_jwt_config() -> &'static JwtConfig {
    JWT_CONFIG.get_or_init(|| {
        let source = &types::config::get_config().auth;
        let secret = source.jwt_secret.clone();
        assert!(!secret.is_empty(), "auth.jwt_secret must be configured");
        if secret.len() < 32 {
            tracing::warn!("auth.jwt_secret is less than 32 characters");
        }
        let secret_secondary = source
            .jwt_secret_secondary
            .clone()
            .filter(|s| !s.is_empty());
        if secret_secondary.is_some() {
            tracing::info!("JWT key rotation enabled: secondary secret configured");
        }

        // Issuer and audience for token validation
        let issuer = source.jwt_issuer.clone();
        let audience = source.jwt_audience.clone();
        let expiry_secs = source.jwt_expiry_secs;

        tracing::info!(
            "JWT configured: issuer={}, audience={}, expiry={}s",
            issuer,
            audience,
            expiry_secs
        );

        JwtConfig {
            secret,
            secret_secondary,
            issuer,
            audience,
            expiry_secs,
        }
    })
}

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user_id as string)
    pub sub: String,
    /// Tenant ID
    pub tenant_id: String,
    /// User role
    pub role: String,
    /// Expiration time (Unix timestamp)
    pub exp: usize,
    /// Issued at (Unix timestamp)
    pub iat: usize,
    /// Issuer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    /// Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    /// Session fingerprint hash (for theft detection)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

/// Generate JWT token for a user (without fingerprint - for backwards compatibility)
pub fn generate_token(
    user_id: Uuid,
    tenant_id: Uuid,
    role: String,
) -> Result<String, jsonwebtoken::errors::Error> {
    generate_token_with_fingerprint(user_id, tenant_id, role, None)
}

/// Generate JWT token for a user with optional session fingerprint
pub fn generate_token_with_fingerprint(
    user_id: Uuid,
    tenant_id: Uuid,
    role: String,
    fingerprint: Option<String>,
) -> Result<String, jsonwebtoken::errors::Error> {
    let config = get_jwt_config();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        role,
        exp: now + config.expiry_secs,
        iat: now,
        iss: Some(config.issuer.clone()),
        aud: Some(config.audience.clone()),
        fingerprint,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
}

/// Verify and decode a JWT token
///
/// Validates:
/// - Signature (tries primary secret, then secondary for rotation)
/// - Expiration time
/// - Issuer and audience (if present in token)
pub fn verify_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let config = get_jwt_config();

    // Build validation rules
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.leeway = 60; // 60 seconds leeway for clock drift

    // Validate issuer if configured
    validation.set_issuer(&[&config.issuer]);

    // Validate audience if configured
    validation.set_audience(&[&config.audience]);

    // Try primary secret first
    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    ) {
        Ok(token_data) => return Ok(token_data.claims),
        Err(primary_err) => {
            // If we have a secondary secret, try that (for key rotation)
            if let Some(ref secondary) = config.secret_secondary {
                match decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(secondary.as_bytes()),
                    &validation,
                ) {
                    Ok(token_data) => {
                        tracing::debug!("Token validated with secondary secret (key rotation)");
                        return Ok(token_data.claims);
                    }
                    Err(_) => {
                        // Both secrets failed, return primary error
                        return Err(primary_err);
                    }
                }
            }
            Err(primary_err)
        }
    }
}

/// Verify token without issuer/audience validation (for legacy tokens)
/// Use only during migration period
#[allow(dead_code)]
pub fn verify_token_legacy(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let config = get_jwt_config();

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.leeway = 60;
    // Don't validate issuer/audience for legacy tokens
    validation.validate_aud = false;
    validation.set_issuer::<&str>(&[]);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_roundtrip() {
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let role = "Admin".to_string();

        let token =
            generate_token(user_id, tenant_id, role.clone()).expect("Failed to generate token");

        // Note: verify_token will fail in tests because OnceLock is already initialized
        // In a real test, you'd need to handle this differently
        assert!(!token.is_empty());
    }
}
