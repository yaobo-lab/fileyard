pub mod jwt;
pub mod middleware;

// 重新导出常用的 JWT 与鉴权中间件组件
pub use jwt::{generate_token, generate_token_with_fingerprint, verify_token, Claims};
pub use middleware::{
    auth_middleware, auth_middleware_with_db, has_role, optional_auth_middleware, require_admin,
    require_manager, require_super_admin, AuthDatabaseState, AuthUser,
};
