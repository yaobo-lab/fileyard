use super::traits::{FileMetadata, Storage, StorageByteStream};
use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{config::Region, Client};
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use tokio_util::io::ReaderStream;

/// S3 兼容对象存储驱动后端 (支持 AWS S3, MinIO, Wasabi, Cloudflare R2, Backblaze B2 等)
pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    /// 基于指定 Bucket 名称与环境变量默认配置初始化 S3 客户端
    pub async fn new(bucket: String) -> Self {
        let region_provider =
            RegionProviderChain::default_provider().or_else(Region::new("us-east-1"));
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&config);
        Self { client, bucket }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let body = ByteStream::from(data);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await?;
        Ok(format!("s3://{}/{}", self.bucket, key))
    }

    async fn upload_from_path(
        &self,
        key: &str,
        path: &Path,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Stream from file to S3
        let body = ByteStream::from_path(path).await?;
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await?;
        Ok(format!("s3://{}/{}", self.bucket, key))
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        let data = resp.body.collect().await?;
        Ok(data.into_bytes().to_vec())
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>, Box<dyn Error + Send + Sync>> {
        let prefix = if prefix.is_empty() {
            "".to_string()
        } else if prefix.ends_with('/') {
            prefix.to_string()
        } else {
            format!("{}/", prefix)
        };

        let mut response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .delimiter("/")
            .send()
            .await?;

        let mut files = Vec::new();

        // Process directories (CommonPrefixes)
        if let Some(prefixes) = response.common_prefixes {
            for prefix in prefixes {
                if let Some(p) = prefix.prefix {
                    files.push(FileMetadata {
                        name: p,
                        size: 0,
                        modified: String::new(),
                        is_dir: true,
                    });
                }
            }
        }

        // Process files (Contents)
        if let Some(objects) = response.contents {
            for obj in objects {
                if let Some(key) = obj.key {
                    // Skip the prefix itself if it appears in contents (e.g. the folder placeholder)
                    if key == prefix {
                        continue;
                    }

                    let size = obj.size.unwrap_or(0) as u64;
                    let modified = obj.last_modified.map(|d| d.to_string()).unwrap_or_default();

                    files.push(FileMetadata {
                        name: key,
                        size,
                        modified,
                        is_dir: false,
                    });
                }
            }
        }

        while response.is_truncated.unwrap_or(false) {
            let next_token = response.next_continuation_token.clone();
            response = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&prefix)
                .delimiter("/")
                .continuation_token(next_token.unwrap())
                .send()
                .await?;

            if let Some(prefixes) = response.common_prefixes {
                for prefix in prefixes {
                    if let Some(p) = prefix.prefix {
                        files.push(FileMetadata {
                            name: p,
                            size: 0,
                            modified: String::new(),
                            is_dir: true,
                        });
                    }
                }
            }

            if let Some(objects) = response.contents {
                for obj in objects {
                    if let Some(key) = obj.key {
                        if key == prefix {
                            continue;
                        }
                        let size = obj.size.unwrap_or(0) as u64;
                        let modified = obj.last_modified.map(|d| d.to_string()).unwrap_or_default();

                        files.push(FileMetadata {
                            name: key,
                            size,
                            modified,
                            is_dir: false,
                        });
                    }
                }
            }
        }

        Ok(files)
    }

    async fn create_folder(&self, key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let key = if key.ends_with('/') {
            key.to_string()
        } else {
            format!("{}/", key)
        };
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(vec![]))
            .send()
            .await?;
        Ok(())
    }

    async fn rename(&self, from: &str, to: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Copy object
        self.client
            .copy_object()
            .bucket(&self.bucket)
            .copy_source(format!("{}/{}", self.bucket, from))
            .key(to)
            .send()
            .await?;

        // Delete original
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(from)
            .send()
            .await?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 生成 S3 直传/直接下载的预签名 URL
    async fn presigned_download_url(
        &self,
        key: &str,
        expires_in_secs: u64,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(expires_in_secs))
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

        Ok(Some(presigned_request.uri().to_string()))
    }

    fn supports_presigned_urls(&self) -> bool {
        true
    }

    async fn download_stream(
        &self,
        key: &str,
    ) -> Result<(StorageByteStream, u64), Box<dyn Error + Send + Sync>> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        let size = resp.content_length().unwrap_or(0) as u64;

        let async_reader = resp.body.into_async_read();
        let reader_stream = ReaderStream::new(async_reader);

        Ok((Box::pin(reader_stream), size))
    }

    async fn health_check(&self) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let start = std::time::Instant::now();
        self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .max_keys(1)
            .send()
            .await?;
        Ok(start.elapsed().as_millis() as u64)
    }
}
