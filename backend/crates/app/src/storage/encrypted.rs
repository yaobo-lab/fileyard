use super::local::LocalStorage;
use super::traits::{FileMetadata, Storage, StorageByteStream};
use async_trait::async_trait;
use bytes::Bytes;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use std::error::Error;
use std::path::Path;

/// ChaCha20-Poly1305 Nonce 大小 (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;

/// 本地透明落盘加密存储驱动包装器
///
/// 包装底层 `LocalStorage`，通过 ChaCha20-Poly1305 AEAD 对落盘文件进行透明加解密：
/// - 文件落盘格式：`[12字节随机 Nonce][密文 Ciphertext][16字节 Auth Tag]`
/// - 上传时自动加密，下载时自动解密
/// - 向后兼容模式：若读取到历史未加密明文文件，自动降级回传明文，不破坏存量数据
pub struct EncryptedLocalStorage {
    inner: LocalStorage,
    cipher: ChaCha20Poly1305,
}

impl EncryptedLocalStorage {
    /// 初始化加密存储
    ///
    /// # 参数
    /// - `base_path`: 文件落盘基准路径
    /// - `key`: 32 字节主加密密钥 (256 bits)
    pub fn new(base_path: &str, key: &[u8; 32]) -> Self {
        tracing::info!("Initializing encrypted local storage at: {}", base_path);
        Self {
            inner: LocalStorage::new(base_path),
            cipher: ChaCha20Poly1305::new(key.into()),
        }
    }

    /// 从 Base64 字符串解析并初始化加密存储
    pub fn from_base64_key(
        base_path: &str,
        key_base64: &str,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        use base64::Engine;
        let key_bytes = base64::engine::general_purpose::STANDARD
            .decode(key_base64)
            .map_err(|e| format!("Invalid base64 encryption key: {}", e))?;

        if key_bytes.len() != 32 {
            return Err(format!(
                "Encryption key must be exactly 32 bytes, got {} bytes",
                key_bytes.len()
            )
            .into());
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);

        Ok(Self::new(base_path, &key))
    }

    /// 使用随机 Nonce 对明文数据进行 ChaCha20-Poly1305 加密
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend(ciphertext);

        Ok(result)
    }

    /// 解密带有 Nonce 前缀的密文数据，若解密失败则向后兼容直接回传原始数据
    fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        if data.len() < NONCE_SIZE + 16 {
            tracing::debug!("File too short to be encrypted, returning as plaintext");
            return data.to_vec();
        }

        let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
        let ciphertext = &data[NONCE_SIZE..];

        match self.cipher.decrypt(nonce, ciphertext) {
            Ok(plaintext) => {
                tracing::debug!("Successfully decrypted file ({} bytes)", plaintext.len());
                plaintext
            }
            Err(_) => {
                tracing::debug!(
                    "Decryption failed, assuming plaintext file (backwards compatibility)"
                );
                data.to_vec()
            }
        }
    }
}

#[async_trait]
impl Storage for EncryptedLocalStorage {
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let encrypted = self.encrypt(&data)?;
        tracing::debug!(
            "Encrypting upload: {} -> {} bytes (key: {})",
            data.len(),
            encrypted.len(),
            key
        );
        self.inner.upload(key, encrypted).await
    }

    async fn upload_from_path(
        &self,
        key: &str,
        path: &Path,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let data = tokio::fs::read(path).await?;
        let encrypted = self.encrypt(&data)?;
        tracing::debug!(
            "Encrypting upload from path: {} -> {} bytes (key: {})",
            data.len(),
            encrypted.len(),
            key
        );
        self.inner.upload(key, encrypted).await
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let encrypted = self.inner.download(key).await?;
        Ok(self.decrypt(&encrypted))
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.inner.delete(key).await
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>, Box<dyn Error + Send + Sync>> {
        self.inner.list(prefix).await
    }

    async fn create_folder(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.inner.create_folder(key).await
    }

    async fn rename(&self, from: &str, to: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.inner.rename(from, to).await
    }

    async fn exists(&self, key: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
        self.inner.exists(key).await
    }

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
        let encrypted = self.inner.download(key).await?;
        let decrypted = self.decrypt(&encrypted);
        let size = decrypted.len() as u64;

        let stream =
            futures::stream::once(
                async move { Ok::<Bytes, std::io::Error>(Bytes::from(decrypted)) },
            );

        Ok((Box::pin(stream), size))
    }

    async fn health_check(&self) -> Result<u64, Box<dyn Error + Send + Sync>> {
        self.inner.health_check().await
    }
}
