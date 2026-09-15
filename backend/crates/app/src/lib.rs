#![allow(deprecated)]

pub mod api;
pub mod app;
pub mod auth;
pub mod middleware;
pub mod router;
pub mod storage;

// 重新导出 api 下的子模块，确保 crate::xxx 路径完全向后兼容
pub use api::*;
pub use app::AppState;
