//! Circuit Breaker Pattern for External Services
//!
//! Implements the circuit breaker pattern to prevent cascading failures
//! when external services (Redis, S3, etc.) become unavailable.
//!
//! States:
//! - Closed: Normal operation, requests pass through
//! - Open: Fail fast, requests rejected immediately  
//! - HalfOpen: Test mode, allow limited requests to check recovery

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed = 0,
    /// Failing fast - requests rejected immediately
    Open = 1,
    /// Testing recovery - allow limited requests
    HalfOpen = 2,
}

impl From<u32> for CircuitState {
    fn from(val: u32) -> Self {
        match val {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }
}

/// Circuit breaker for protecting against cascading failures
/// 
/// Thread-safe implementation using atomics for lock-free operation
pub struct CircuitBreaker {
    name: String,
    /// Current state (0=Closed, 1=Open, 2=HalfOpen)
    state: AtomicU32,
    /// Number of consecutive failures
    failure_count: AtomicU32,
    /// Timestamp of last failure (Unix seconds)
    last_failure_time: AtomicU64,
    /// Number of failures before opening circuit
    failure_threshold: u32,
    /// Seconds before attempting recovery (transition to half-open)
    recovery_timeout_secs: u64,
    /// Number of successful requests needed in half-open to close circuit
    success_threshold: u32,
    /// Count of successes in half-open state
    half_open_successes: AtomicU32,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(
        name: impl Into<String>,
        failure_threshold: u32,
        recovery_timeout_secs: u64,
        success_threshold: u32,
    ) -> Self {
        Self {
            name: name.into(),
            state: AtomicU32::new(CircuitState::Closed as u32),
            failure_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            failure_threshold,
            recovery_timeout_secs,
            success_threshold,
            half_open_successes: AtomicU32::new(0),
        }
    }

    /// Create with default settings (5 failures, 30s recovery, 3 successes to close)
    pub fn with_defaults(name: impl Into<String>) -> Self {
        Self::new(name, 5, 30, 3)
    }

    /// Get the current state of the circuit breaker
    pub fn state(&self) -> CircuitState {
        self.check_state_transition();
        CircuitState::from(self.state.load(Ordering::SeqCst))
    }

    /// Check if a request should be allowed through
    /// Returns true if request can proceed, false if circuit is open
    pub fn allow_request(&self) -> bool {
        self.check_state_transition();
        
        let state = CircuitState::from(self.state.load(Ordering::SeqCst));
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                debug!("Circuit breaker '{}' is OPEN - rejecting request", self.name);
                false
            }
            CircuitState::HalfOpen => {
                debug!("Circuit breaker '{}' is HALF-OPEN - allowing test request", self.name);
                true
            }
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let state = CircuitState::from(self.state.load(Ordering::SeqCst));
        
        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let successes = self.half_open_successes.fetch_add(1, Ordering::SeqCst) + 1;
                if successes >= self.success_threshold {
                    // Enough successes - close the circuit
                    self.state.store(CircuitState::Closed as u32, Ordering::SeqCst);
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.half_open_successes.store(0, Ordering::SeqCst);
                    info!("Circuit breaker '{}' CLOSED after {} successful test requests", self.name, successes);
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                self.failure_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let state = CircuitState::from(self.state.load(Ordering::SeqCst));
        let now = current_timestamp();
        
        self.last_failure_time.store(now, Ordering::SeqCst);
        
        match state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if failures >= self.failure_threshold {
                    // Too many failures - open the circuit
                    self.state.store(CircuitState::Open as u32, Ordering::SeqCst);
                    warn!(
                        "Circuit breaker '{}' OPENED after {} consecutive failures",
                        self.name, failures
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Failure during test - go back to open
                self.state.store(CircuitState::Open as u32, Ordering::SeqCst);
                self.half_open_successes.store(0, Ordering::SeqCst);
                warn!("Circuit breaker '{}' reopened after failure during test", self.name);
            }
            CircuitState::Open => {
                // Already open, just update timestamp
            }
        }
    }

    /// Check if state should transition based on time
    fn check_state_transition(&self) {
        let state = CircuitState::from(self.state.load(Ordering::SeqCst));
        
        if state == CircuitState::Open {
            let last_failure = self.last_failure_time.load(Ordering::SeqCst);
            let now = current_timestamp();
            
            if now - last_failure >= self.recovery_timeout_secs {
                // Recovery timeout elapsed - try half-open
                self.state.store(CircuitState::HalfOpen as u32, Ordering::SeqCst);
                self.half_open_successes.store(0, Ordering::SeqCst);
                info!(
                    "Circuit breaker '{}' transitioning to HALF-OPEN after {}s",
                    self.name, self.recovery_timeout_secs
                );
            }
        }
    }

    /// Execute a fallible operation with circuit breaker protection
    /// 
    /// Returns Err(CircuitBreakerError::Open) if circuit is open,
    /// or the result of the operation otherwise
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        if !self.allow_request() {
            return Err(CircuitBreakerError::Open);
        }

        match operation.await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                self.record_failure();
                Err(CircuitBreakerError::ServiceError(e))
            }
        }
    }

    /// Get metrics for monitoring
    pub fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            name: self.name.clone(),
            state: self.state(),
            failure_count: self.failure_count.load(Ordering::SeqCst),
            last_failure_time: self.last_failure_time.load(Ordering::SeqCst),
        }
    }
}

/// Error type for circuit breaker operations
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open - request not attempted
    Open,
    /// Service returned an error
    ServiceError(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::Open => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::ServiceError(e) => write!(f, "Service error: {}", e),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for CircuitBreakerError<E> {}

/// Metrics for monitoring circuit breaker state
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub name: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub last_failure_time: u64,
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::with_defaults("test");
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_circuit_opens_after_threshold() {
        let cb = CircuitBreaker::new("test", 3, 10, 2);
        
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_success_resets_failure_count() {
        let cb = CircuitBreaker::new("test", 3, 10, 2);
        
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        
        // Should be back to 0 failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}

