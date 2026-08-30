pub mod middleware;
pub mod jwt;

// Re-export commonly used items
pub use middleware::{
    auth_middleware,
    optional_auth_middleware,
    AuthUser,
    has_role,
    require_admin,
    require_manager,
    require_super_admin,
};

pub use jwt::{
    generate_token,
    generate_token_with_fingerprint,
    verify_token,
    Claims,
};
