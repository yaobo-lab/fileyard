use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::error::Error;
use std::path::Path;
use std::pin::Pin;

/// 用于零拷贝流式文件下载的固定盒装字节流
pub type StorageByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>;

/// 存储文件元数据
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// 文件名或路径 Key
    pub name: String,
    /// 文件字节大小
    pub size: u64,
    /// 最后修改时间 (ISO 8601 或时间戳字符串)
    pub modified: String,
    /// 是否为目录
    pub is_dir: bool,
}

/// 上传进度追踪回调函数类型
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// 统一文件存储抽象接口
///
/// 屏蔽底层存储介质（本地存储、落盘加密存储、S3 兼容对象存储）差异，提供一致的文件操作抽象：
#[async_trait]
pub trait Storage: Send + Sync {
    /// 将内存中的字节数据上传至存储介质
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// 从本地文件路径流式上传（对于大文件比在内存中全量缓冲更高效）
    async fn upload_from_path(
        &self,
        key: &str,
        path: &Path,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// 下载指定 Key 的完整文件内容到内存字节向量
    async fn download(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>>;

    /// 删除指定 Key 的文件或目录
    async fn delete(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// 列出指定前缀下的所有文件元数据列表
    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>, Box<dyn Error + Send + Sync>>;

    /// 创建目录/文件夹
    async fn create_folder(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// 重命名或移动对象
    async fn rename(&self, from: &str, to: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// 检查指定 Key 的对象是否存在
    async fn exists(&self, key: &str) -> Result<bool, Box<dyn Error + Send + Sync>>;

    /// 生成直接下载的预签名 URL（仅 S3 兼容对象存储支持，本地存储返回 None）
    async fn presigned_download_url(
        &self,
        key: &str,
        expires_in_secs: u64,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>>;

    /// 查询当前存储驱动是否支持预签名 URL
    fn supports_presigned_urls(&self) -> bool;

    /// 流式分块下载 - 返回字节流与文件字节大小（用于设置 Content-Length），避免全量装入内存
    async fn download_stream(
        &self,
        key: &str,
    ) -> Result<(StorageByteStream, u64), Box<dyn Error + Send + Sync>>;

    /// 存储连通性与健康检查 - 返回响应耗时（毫秒）
    async fn health_check(&self) -> Result<u64, Box<dyn Error + Send + Sync>>;
}
