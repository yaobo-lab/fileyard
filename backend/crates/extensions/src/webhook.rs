//! Webhook dispatch and signature verification

use hmac::{Hmac, Mac};
use sha2::Sha256;
use ed25519_dalek::{SigningKey, Signer, VerifyingKey, Verifier, Signature};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::{Duration, Instant};
use thiserror::Error;
use crate::models::Extension;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    HmacSha256,
    Ed25519,
}

impl SignatureAlgorithm {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hmac_sha256" => Some(SignatureAlgorithm::HmacSha256),
            "ed25519" => Some(SignatureAlgorithm::Ed25519),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SignatureAlgorithm::HmacSha256 => "hmac_sha256",
            SignatureAlgorithm::Ed25519 => "ed25519",
        }
    }
}

#[derive(Debug, Error)]
pub enum WebhookError {
    #[error("No webhook URL configured")]
    NoWebhookUrl,
    #[error("No signing key configured")]
    NoSigningKey,
    #[error("Invalid signing key: {0}")]
    InvalidSigningKey(String),
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("Webhook returned error status: {0}")]
    ErrorStatus(u16),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Generate a signature for a payload
pub fn sign_payload(payload: &[u8], secret: &str, algo: SignatureAlgorithm) -> Result<String, WebhookError> {
    match algo {
        SignatureAlgorithm::HmacSha256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
                .map_err(|e| WebhookError::InvalidSigningKey(e.to_string()))?;
            mac.update(payload);
            let result = mac.finalize();
            Ok(format!("sha256={}", hex::encode(result.into_bytes())))
        }
        SignatureAlgorithm::Ed25519 => {
            // For Ed25519, the secret should be the hex-encoded private key seed (32 bytes)
            let key_bytes = hex::decode(secret)
                .map_err(|e| WebhookError::InvalidSigningKey(format!("Invalid hex: {}", e)))?;
            
            if key_bytes.len() != 32 {
                return Err(WebhookError::InvalidSigningKey(
                    "Ed25519 key must be 32 bytes".to_string(),
                ));
            }

            let mut seed = [0u8; 32];
            seed.copy_from_slice(&key_bytes);
            let signing_key = SigningKey::from_bytes(&seed);
            let signature = signing_key.sign(payload);
            Ok(format!("ed25519={}", hex::encode(signature.to_bytes())))
        }
    }
}

/// Verify a signature (for extension developers to use)
pub fn verify_signature(
    payload: &[u8],
    signature: &str,
    public_key: &str,
    algo: SignatureAlgorithm,
) -> Result<bool, WebhookError> {
    match algo {
        SignatureAlgorithm::HmacSha256 => {
            // For HMAC, the "public key" is actually the shared secret
            let expected = sign_payload(payload, public_key, algo)?;
            Ok(expected == signature)
        }
        SignatureAlgorithm::Ed25519 => {
            let sig_hex = signature
                .strip_prefix("ed25519=")
                .ok_or_else(|| WebhookError::InvalidSigningKey("Invalid signature format".to_string()))?;
            
            let sig_bytes = hex::decode(sig_hex)
                .map_err(|e| WebhookError::InvalidSigningKey(format!("Invalid signature hex: {}", e)))?;
            
            if sig_bytes.len() != 64 {
                return Err(WebhookError::InvalidSigningKey(
                    "Ed25519 signature must be 64 bytes".to_string(),
                ));
            }

            let mut sig_arr = [0u8; 64];
            sig_arr.copy_from_slice(&sig_bytes);
            let signature = Signature::from_bytes(&sig_arr);

            let key_bytes = hex::decode(public_key)
                .map_err(|e| WebhookError::InvalidSigningKey(format!("Invalid public key hex: {}", e)))?;
            
            if key_bytes.len() != 32 {
                return Err(WebhookError::InvalidSigningKey(
                    "Ed25519 public key must be 32 bytes".to_string(),
                ));
            }

            let mut key_arr = [0u8; 32];
            key_arr.copy_from_slice(&key_bytes);
            let verifying_key = VerifyingKey::from_bytes(&key_arr)
                .map_err(|e| WebhookError::InvalidSigningKey(e.to_string()))?;

            Ok(verifying_key.verify(payload, &signature).is_ok())
        }
    }
}

/// Generate a new Ed25519 keypair for an extension
pub fn generate_ed25519_keypair() -> (String, String) {
    use rand::rngs::OsRng;
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    
    (
        hex::encode(signing_key.to_bytes()),    // Private key (keep secret)
        hex::encode(verifying_key.to_bytes()),  // Public key (share with ClovaLink)
    )
}

/// Generate a random HMAC secret
pub fn generate_hmac_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

/// Webhook payload for file events
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEventPayload {
    pub company_id: String,
    pub user_id: String,
    pub file_id: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub size_bytes: i64,
    pub event: String,
    pub metadata: serde_json::Value,
    pub timestamp: String,
}

/// Webhook payload for automation events
#[derive(Debug, Serialize, Deserialize)]
pub struct AutomationEventPayload {
    pub company_id: String,
    pub extension_id: String,
    pub job_id: String,
    pub event: String,
    pub config: serde_json::Value,
    pub timestamp: String,
}

/// Dispatch a webhook to an extension
pub async fn dispatch_webhook<T: Serialize>(
    pool: &PgPool,
    extension: &Extension,
    event_type: &str,
    payload: &T,
    timeout_ms: u64,
) -> Result<(u16, String), WebhookError> {
    let webhook_url = extension
        .webhook_url
        .as_ref()
        .ok_or(WebhookError::NoWebhookUrl)?;

    let public_key = extension
        .public_key
        .as_ref()
        .ok_or(WebhookError::NoSigningKey)?;

    let algo = SignatureAlgorithm::from_str(&extension.signature_algorithm)
        .unwrap_or(SignatureAlgorithm::HmacSha256);

    let payload_json = serde_json::to_vec(payload)
        .map_err(|e| WebhookError::SerializationError(e.to_string()))?;

    let signature = sign_payload(&payload_json, public_key, algo)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| WebhookError::RequestFailed(e.to_string()))?;

    let start = Instant::now();

    let response = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .header("X-ClovaLink-Signature", &signature)
        .header("X-ClovaLink-Event", event_type)
        .header("X-ClovaLink-Extension-Id", extension.id.to_string())
        .header("X-ClovaLink-Timestamp", chrono::Utc::now().to_rfc3339())
        .body(payload_json.clone())
        .send()
        .await;

    let duration_ms = start.elapsed().as_millis() as i32;

    let (status, body, error) = match response {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            let error = if status >= 400 {
                Some(format!("HTTP {}", status))
            } else {
                None
            };
            (Some(status as i32), Some(body), error)
        }
        Err(e) => (None, None, Some(e.to_string())),
    };

    // Log the webhook call
    let _ = sqlx::query!(
        r#"
        INSERT INTO extension_webhook_logs 
        (extension_id, tenant_id, event_type, payload, request_headers, response_status, response_body, duration_ms, error_message)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        extension.id,
        extension.tenant_id,
        event_type,
        serde_json::to_value(payload).ok(),
        serde_json::json!({
            "X-ClovaLink-Signature": signature,
            "X-ClovaLink-Event": event_type,
        }),
        status,
        body.clone(),
        duration_ms,
        error.clone()
    )
    .execute(pool)
    .await;

    if let Some(err) = error {
        return Err(WebhookError::RequestFailed(err));
    }

    let final_status = status.unwrap_or(0) as u16;
    let final_body = body.unwrap_or_default();

    if final_status >= 400 {
        return Err(WebhookError::ErrorStatus(final_status));
    }

    Ok((final_status, final_body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_signature() {
        let payload = b"test payload";
        let secret = "test-secret-key";
        
        let sig = sign_payload(payload, secret, SignatureAlgorithm::HmacSha256).unwrap();
        assert!(sig.starts_with("sha256="));
        
        let verified = verify_signature(payload, &sig, secret, SignatureAlgorithm::HmacSha256).unwrap();
        assert!(verified);
    }

    #[test]
    fn test_ed25519_signature() {
        let (private_key, public_key) = generate_ed25519_keypair();
        let payload = b"test payload";
        
        let sig = sign_payload(payload, &private_key, SignatureAlgorithm::Ed25519).unwrap();
        assert!(sig.starts_with("ed25519="));
        
        let verified = verify_signature(payload, &sig, &public_key, SignatureAlgorithm::Ed25519).unwrap();
        assert!(verified);
    }
}

