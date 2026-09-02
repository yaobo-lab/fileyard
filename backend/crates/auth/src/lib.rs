pub mod jwt;
pub mod middleware;

// Re-export commonly used items
pub use middleware::{
    auth_middleware, has_role, optional_auth_middleware, require_admin, require_manager,
    require_super_admin, AuthUser,
};

pub use jwt::{generate_token, generate_token_with_fingerprint, verify_token, Claims};
