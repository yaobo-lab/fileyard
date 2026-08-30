//! Password hashing utilities with tuned Argon2 parameters
//! 
//! Provides consistent, secure password hashing across the application.
//! Uses Argon2id with OWASP-recommended parameters for increased security.

use argon2::{Argon2, Algorithm, Version, Params};

/// Get a configured Argon2 hasher with production-ready parameters.
/// 
/// Parameters are tuned for security while remaining performant:
/// - Algorithm: Argon2id (hybrid, resistant to both side-channel and GPU attacks)
/// - Memory: 64MB (64 * 1024 KB)
/// - Iterations: 3
/// - Parallelism: 4
/// 
/// These parameters meet OWASP recommendations and provide good protection
/// against offline brute-force attacks.
pub fn get_argon2<'a>() -> Argon2<'a> {
    // OWASP recommended parameters for Argon2id
    // https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
    let params = Params::new(
        64 * 1024,  // 64 MiB memory cost
        3,          // 3 iterations (time cost)
        4,          // 4 lanes (parallelism)
        None,       // Default output length (32 bytes)
    ).unwrap_or_else(|_| {
        // Fallback to default params if custom params fail
        tracing::warn!("Failed to create custom Argon2 params, using defaults");
        Params::default()
    });
    
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
    
    #[test]
    fn test_argon2_hashing() {
        let argon2 = get_argon2();
        let salt = SaltString::generate(&mut OsRng);
        let password = "test_password_123";
        
        let hash = argon2.hash_password(password.as_bytes(), &salt);
        assert!(hash.is_ok());
        
        let hash_str = hash.unwrap().to_string();
        assert!(hash_str.starts_with("$argon2id$"));
    }
}

