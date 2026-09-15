use super::traits::{FileMetadata, Storage, StorageByteStream};
use async_trait::async_trait;
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio_util::io::ReaderStream;

/// 本地文件系统存储驱动后端
pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    /// 初始化本地存储，若目录不存在则自动创建
    pub fn new(base_path: &str) -> Self {
        std::fs::create_dir_all(base_path).unwrap_or_default();
        Self {
            base_path: PathBuf::from(base_path),
        }
    }

    /// 获取存储基准路径引用
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let path = self.base_path.join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, data).await?;
        Ok(format!("local://{}", path.display()))
    }

    async fn upload_from_path(
        &self,
        key: &str,
        source_path: &Path,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let dest_path = self.base_path.join(key);
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // 复制文件（流式）
        tokio::fs::copy(source_path, &dest_path).await?;
        Ok(format!("local://{}", dest_path.display()))
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let path = self.base_path.join(key);
        let data = tokio::fs::read(path).await?;
        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = self.base_path.join(key);
        if path.is_dir() {
            tokio::fs::remove_dir_all(path).await?;
        } else {
            tokio::fs::remove_file(path).await?;
        }
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>, Box<dyn Error + Send + Sync>> {
        let mut files = Vec::new();
        let path = self.base_path.join(prefix);

        if !path.exists() {
            return Ok(files);
        }

        let mut entries = tokio::fs::read_dir(path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = metadata.is_dir();
            let size = metadata.len();
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default();

            files.push(FileMetadata {
                name,
                size,
                modified,
                is_dir,
            });
        }
        Ok(files)
    }

    async fn create_folder(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = self.base_path.join(key);
        tokio::fs::create_dir_all(path).await?;
        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let from_path = self.base_path.join(from);
        let to_path = self.base_path.join(to);

        // 确保目标父级目录存在
        if let Some(parent) = to_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::rename(from_path, to_path).await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let path = self.base_path.join(key);
        Ok(path.exists())
    }

    /// 本地存储不支持预签名 URL，返回 None 触发代理回退下载
    async fn presigned_download_url(
        &self,
        _key: &str,
        _expires_in_secs: u64,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        Ok(None)
    }

    fn supports_presigned_urls(&self) -> bool {
        false
    }

    async fn download_stream(
        &self,
        key: &str,
    ) -> Result<(StorageByteStream, u64), Box<dyn Error + Send + Sync>> {
        let path = self.base_path.join(key);
        let file = tokio::fs::File::open(&path).await?;
        let metadata = file.metadata().await?;
        let size = metadata.len();

        // 基于 Tokio File 构建 ReaderStream（默认 ~8KB 分块流式返回）
        let stream = ReaderStream::new(file);

        Ok((Box::pin(stream), size))
    }

    async fn health_check(&self) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let start = std::time::Instant::now();
        if self.base_path.exists() && self.base_path.is_dir() {
            Ok(start.elapsed().as_millis() as u64)
        } else {
            Err("Local storage path does not exist or is not a directory".into())
        }
    }
}
