//! Redis caching utilities for ClovaLink
//! 
//! Provides a simple API for caching serializable data with TTL support.
//! Includes circuit breaker protection to prevent cascading failures.

use crate::circuit_breaker::{CircuitBreaker, CircuitState};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Cache key prefixes for different data types
pub mod keys {
    use uuid::Uuid;
    
    pub fn compliance(tenant_id: Uuid) -> String {
        format!("clovalink:compliance:{}", tenant_id)
    }
    
    pub fn user(user_id: Uuid) -> String {
        format!("clovalink:user:{}", user_id)
    }
    
    pub fn user_tenants(user_id: Uuid) -> String {
        format!("clovalink:tenants:{}", user_id)
    }
    
    pub fn files(company_id: Uuid, path_hash: &str) -> String {
        format!("clovalink:files:{}:{}", company_id, path_hash)
    }
    
    pub fn tenant(tenant_id: Uuid) -> String {
        format!("clovalink:tenant:{}", tenant_id)
    }
    
    pub fn tenant_settings(tenant_id: Uuid) -> String {
        format!("clovalink:tenant_settings:{}", tenant_id)
    }
    
    pub fn dashboard_stats(tenant_id: Uuid, role: &str) -> String {
        format!("clovalink:dashboard:{}:{}", tenant_id, role)
    }
    
    pub fn global_settings() -> String {
        "clovalink:global_settings".to_string()
    }
    
    pub fn user_permissions(user_id: Uuid) -> String {
        format!("clovalink:user_perms:{}", user_id)
    }
}

/// Default TTL values in seconds
pub mod ttl {
    pub const COMPLIANCE: u64 = 300;      // 5 minutes
    pub const USER: u64 = 120;            // 2 minutes
    pub const TENANTS: u64 = 300;         // 5 minutes
    pub const FILES: u64 = 60;            // 1 minute
    pub const TENANT: u64 = 300;          // 5 minutes
    pub const TENANT_SETTINGS: u64 = 300; // 5 minutes
    pub const DASHBOARD: u64 = 60;        // 1 minute (dashboard data changes frequently)
    pub const GLOBAL_SETTINGS: u64 = 600; // 10 minutes
    pub const USER_PERMISSIONS: u64 = 300; // 5 minutes
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Redis connection error: {0}")]
    ConnectionError(String),
    #[error("Redis command error: {0}")]
    CommandError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Cache miss")]
    CacheMiss,
    #[error("Circuit breaker open - Redis unavailable")]
    CircuitOpen,
}

/// Redis cache client with connection pooling and circuit breaker protection
#[derive(Clone)]
pub struct Cache {
    conn: Arc<RwLock<ConnectionManager>>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl Cache {
    /// Create a new cache instance from a Redis URL
    pub async fn new(redis_url: &str) -> Result<Self, CacheError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| CacheError::ConnectionError(e.to_string()))?;
        
        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| CacheError::ConnectionError(e.to_string()))?;
        
        // Circuit breaker: open after 5 failures, recover after 30s, need 3 successes
        let circuit_breaker = Arc::new(CircuitBreaker::new("redis", 5, 30, 3));
        
        Ok(Self {
            conn: Arc::new(RwLock::new(conn)),
            circuit_breaker,
        })
    }
    
    /// Check if circuit breaker is open (for monitoring)
    pub fn is_circuit_open(&self) -> bool {
        self.circuit_breaker.state() == CircuitState::Open
    }
    
    /// Get circuit breaker metrics for monitoring
    pub fn circuit_metrics(&self) -> crate::circuit_breaker::CircuitBreakerMetrics {
        self.circuit_breaker.metrics()
    }
    
    /// Get a value from cache
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<T, CacheError> {
        // Check circuit breaker first
        if !self.circuit_breaker.allow_request() {
            return Err(CacheError::CircuitOpen);
        }
        
        let mut conn = self.conn.write().await;
        
        let result: Result<Option<String>, _> = conn.get(key).await;
        
        match result {
            Ok(Some(json)) => {
                self.circuit_breaker.record_success();
                debug!("Cache hit for key: {}", key);
                serde_json::from_str(&json)
                    .map_err(|e| CacheError::SerializationError(e.to_string()))
            }
            Ok(None) => {
                self.circuit_breaker.record_success();
                debug!("Cache miss for key: {}", key);
                Err(CacheError::CacheMiss)
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                warn!("Redis get error for key {}: {}", key, e);
                Err(CacheError::CommandError(e.to_string()))
            }
        }
    }
    
    /// Set a value in cache with TTL (time-to-live in seconds)
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_seconds: u64) -> Result<(), CacheError> {
        // Check circuit breaker first
        if !self.circuit_breaker.allow_request() {
            return Err(CacheError::CircuitOpen);
        }
        
        let json = serde_json::to_string(value)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;
        
        let mut conn = self.conn.write().await;
        
        let result: Result<(), _> = conn.set_ex(key, json, ttl_seconds).await;
        
        match result {
            Ok(()) => {
                self.circuit_breaker.record_success();
                debug!("Cache set for key: {} (TTL: {}s)", key, ttl_seconds);
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                warn!("Redis set error for key {}: {}", key, e);
                Err(CacheError::CommandError(e.to_string()))
            }
        }
    }
    
    /// Delete a key from cache
    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        // Check circuit breaker first
        if !self.circuit_breaker.allow_request() {
            return Err(CacheError::CircuitOpen);
        }
        
        let mut conn = self.conn.write().await;
        
        let result: Result<(), _> = conn.del(key).await;
        
        match result {
            Ok(()) => {
                self.circuit_breaker.record_success();
                debug!("Cache deleted key: {}", key);
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                warn!("Redis delete error for key {}: {}", key, e);
                Err(CacheError::CommandError(e.to_string()))
            }
        }
    }
    
    /// Delete all keys matching a pattern (use with caution)
    pub async fn delete_pattern(&self, pattern: &str) -> Result<u64, CacheError> {
        // Check circuit breaker first
        if !self.circuit_breaker.allow_request() {
            return Err(CacheError::CircuitOpen);
        }
        
        let mut conn = self.conn.write().await;
        
        // Get all matching keys
        let keys: Result<Vec<String>, _> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut *conn)
            .await;
        
        let keys = match keys {
            Ok(k) => {
                self.circuit_breaker.record_success();
                k
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                return Err(CacheError::CommandError(e.to_string()));
            }
        };
        
        if keys.is_empty() {
            return Ok(0);
        }
        
        let count = keys.len() as u64;
        
        // Delete all matching keys
        for key in keys {
            let result: Result<(), _> = conn.del(&key).await;
            if let Err(e) = result {
                self.circuit_breaker.record_failure();
                return Err(CacheError::CommandError(e.to_string()));
            }
            self.circuit_breaker.record_success();
        }
        
        debug!("Cache deleted {} keys matching pattern: {}", count, pattern);
        Ok(count)
    }
    
    /// Get or set - returns cached value or computes and caches it
    pub async fn get_or_set<T, F, Fut>(
        &self,
        key: &str,
        ttl_seconds: u64,
        compute: F,
    ) -> Result<T, CacheError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, CacheError>>,
    {
        // Try to get from cache first
        match self.get(key).await {
            Ok(value) => return Ok(value),
            Err(CacheError::CacheMiss) => {
                // Compute the value
                let value = compute().await?;
                // Cache it (ignore errors, just warn)
                if let Err(e) = self.set(key, &value, ttl_seconds).await {
                    warn!("Failed to cache value for key {}: {}", key, e);
                }
                Ok(value)
            }
            Err(e) => {
                // On other errors, try to compute anyway
                warn!("Cache error for key {}: {}, computing fresh value", key, e);
                compute().await
            }
        }
    }
    
    /// Check if cache is available (ping)
    /// Also updates circuit breaker state based on result
    pub async fn is_available(&self) -> bool {
        // If circuit is open, check if it's time to test
        if !self.circuit_breaker.allow_request() {
            return false;
        }
        
        let mut conn = self.conn.write().await;
        let result: Result<String, _> = redis::cmd("PING")
            .query_async(&mut *conn)
            .await;
        
        if result.is_ok() {
            self.circuit_breaker.record_success();
            true
        } else {
            self.circuit_breaker.record_failure();
            false
        }
    }
    
    /// Get a raw Redis connection for advanced operations (e.g., atomic INCR+EXPIRE)
    /// 
    /// Use this when you need to run raw Redis commands that aren't exposed
    /// through the high-level API.
    pub async fn get_connection(&self) -> Result<impl std::ops::DerefMut<Target = ConnectionManager> + '_, CacheError> {
        Ok(self.conn.write().await)
    }
}

/// Helper to compute a hash for file paths (for cache keys)
pub fn hash_path(path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_path() {
        let hash1 = hash_path("/documents/reports");
        let hash2 = hash_path("/documents/reports");
        let hash3 = hash_path("/documents/other");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
    
    #[test]
    fn test_cache_keys() {
        let tenant_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        
        let key = keys::compliance(tenant_id);
        assert!(key.starts_with("clovalink:compliance:"));
        
        let key = keys::user(user_id);
        assert!(key.starts_with("clovalink:user:"));
    }
}
