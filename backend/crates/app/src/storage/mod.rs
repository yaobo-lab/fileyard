//! 统一文件存储抽象与多驱动实现
//!
//! 支持以下存储驱动：
//! - `local`: 本地文件系统原始存储驱动
//! - `encrypted`: 基于 ChaCha20-Poly1305 的透明落盘加密存储驱动
//! - `s3`: AWS S3, MinIO, Wasabi, R2 等兼容对象存储驱动

pub mod encrypted;
pub mod local;
pub mod s3;
pub mod traits;

// 重新导出核心抽象与后端驱动类型，提供干净一致的对外 API
pub use encrypted::EncryptedLocalStorage;
pub use local::LocalStorage;
pub use s3::S3Storage;
pub use traits::{FileMetadata, ProgressCallback, Storage, StorageByteStream};
