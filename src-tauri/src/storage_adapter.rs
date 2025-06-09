use anyhow::{Context, Result};
use opendal::{services, Operator};
use serde::{Deserialize, Serialize};

/// 存储后端类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StorageBackend {
    /// 本地文件系统（用于开发和测试）
    FileSystem { root_path: String },
    /// Amazon S3
    S3 {
        bucket: String,
        region: String,
        access_key_id: String,
        secret_access_key: String,
        endpoint: Option<String>, // 用于兼容 S3 API 的其他服务
    },
    /// 阿里云 OSS
    Oss {
        bucket: String,
        endpoint: String,
        access_key_id: String,
        access_key_secret: String,
    },
    /// MinIO 或其他 S3 兼容存储
    S3Compatible {
        bucket: String,
        endpoint: String,
        access_key_id: String,
        secret_access_key: String,
        region: Option<String>,
    },
    /// 腾讯云 COS
    Cos {
        bucket: String,
        endpoint: String,
        secret_id: String,
        secret_key: String,
    },
    /// Azure Blob Storage
    AzBlob {
        container: String,
        account_name: String,
        account_key: String,
    },
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub retry_attempts: usize,
    pub timeout_seconds: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::FileSystem {
                root_path: "./clippy_sync_data".to_string(),
            },
            retry_attempts: 3,
            timeout_seconds: 30,
        }
    }
}

impl StorageConfig {
    /// 创建 OpenDAL Operator
    pub async fn create_operator(&self) -> Result<Operator> {
        match &self.backend {
            StorageBackend::FileSystem { root_path } => {
                let builder = services::Fs::default().root(root_path);
                Ok(Operator::new(builder)?.finish())
            }
            
            StorageBackend::S3 {
                bucket,
                region,
                access_key_id,
                secret_access_key,
                endpoint,
            } => {
                let mut builder = services::S3::default()
                    .bucket(bucket)
                    .region(region)
                    .access_key_id(access_key_id)
                    .secret_access_key(secret_access_key);
                
                if let Some(endpoint) = endpoint {
                    builder = builder.endpoint(endpoint);
                }
                
                Ok(Operator::new(builder)?.finish())
            }
            
            StorageBackend::S3Compatible {
                bucket,
                endpoint,
                access_key_id,
                secret_access_key,
                region,
            } => {
                let builder = services::S3::default()
                    .bucket(bucket)
                    .endpoint(endpoint)
                    .access_key_id(access_key_id)
                    .secret_access_key(secret_access_key)
                    .region(region.as_deref().unwrap_or("us-east-1"));
                
                Ok(Operator::new(builder)?.finish())
            }
            
            StorageBackend::Oss {
                bucket,
                endpoint,
                access_key_id,
                access_key_secret,
            } => {
                let builder = services::Oss::default()
                    .bucket(bucket)
                    .endpoint(endpoint)
                    .access_key_id(access_key_id)
                    .access_key_secret(access_key_secret);
                
                Ok(Operator::new(builder)?.finish())
            }
            
            StorageBackend::Cos {
                bucket,
                endpoint,
                secret_id,
                secret_key,
            } => {
                let builder = services::Cos::default()
                    .bucket(bucket)
                    .endpoint(endpoint)
                    .secret_id(secret_id)
                    .secret_key(secret_key);
                
                Ok(Operator::new(builder)?.finish())
            }
            
            StorageBackend::AzBlob {
                container,
                account_name,
                account_key,
            } => {
                let builder = services::Azblob::default()
                    .container(container)
                    .account_name(account_name)
                    .account_key(account_key);
                
                Ok(Operator::new(builder)?.finish())
            }
        }
    }

    /// 从环境变量或配置文件加载配置
    pub async fn from_env() -> Result<Self> {
        // 优先从环境变量读取配置
        if let Ok(config_str) = std::env::var("CLIPPY_STORAGE_CONFIG") {
            return serde_json::from_str(&config_str)
                .context("Failed to parse storage config from env");
        }

        // 检查是否有各种存储后端的环境变量
        if let Ok(bucket) = std::env::var("AWS_S3_BUCKET") {
            return Ok(Self {
                backend: StorageBackend::S3 {
                    bucket,
                    region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
                    access_key_id: std::env::var("AWS_ACCESS_KEY_ID")
                        .context("AWS_ACCESS_KEY_ID not found")?,
                    secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY")
                        .context("AWS_SECRET_ACCESS_KEY not found")?,
                    endpoint: std::env::var("AWS_ENDPOINT").ok(),
                },
                ..Default::default()
            });
        }

        if let Ok(bucket) = std::env::var("MINIO_BUCKET") {
            return Ok(Self {
                backend: StorageBackend::S3Compatible {
                    bucket,
                    endpoint: std::env::var("MINIO_ENDPOINT")
                        .context("MINIO_ENDPOINT not found")?,
                    access_key_id: std::env::var("MINIO_ACCESS_KEY")
                        .context("MINIO_ACCESS_KEY not found")?,
                    secret_access_key: std::env::var("MINIO_SECRET_KEY")
                        .context("MINIO_SECRET_KEY not found")?,
                    region: std::env::var("MINIO_REGION").ok(),
                },
                ..Default::default()
            });
        }

        // 默认使用本地文件系统
        Ok(Self::default())
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content).context("Failed to save config file")?;
        Ok(())
    }

    /// 从文件加载配置
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read config file")?;
        let config: Self = serde_json::from_str(&content).context("Failed to parse config")?;
        Ok(config)
    }

    /// 验证存储配置是否有效
    pub async fn validate(&self) -> Result<()> {
        let op = self.create_operator().await?;
        
        // 进行基本的连接测试
        let test_path = "test_clippy_connection.txt";
        let test_content = b"Clippy storage test";
        
        op.write(test_path, test_content.to_vec())
            .await
            .context("Failed to write test file")?;
        
        let content = op
            .read(test_path)
            .await
            .context("Failed to read test file")?;
        
        if content.to_bytes().as_ref() != test_content {
            anyhow::bail!("Storage validation failed: content mismatch");
        }
        
        // 清理测试文件
        let _ = op.delete(test_path).await;
        
        Ok(())
    }
}

/// 存储管理器，提供统一的存储接口
pub struct StorageManager {
    operator: Operator,
    config: StorageConfig,
}

impl StorageManager {
    pub async fn new(config: StorageConfig) -> Result<Self> {
        let operator = config.create_operator().await?;
        Ok(Self { operator, config })
    }

    pub fn operator(&self) -> &Operator {
        &self.operator
    }

    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    /// 确保用户目录结构存在
    pub async fn ensure_user_directories(&self, user_id: &str) -> Result<()> {
        let directories = [
            format!("{}/oplog/", user_id),
            format!("{}/snapshots/", user_id),
            format!("{}/data/", user_id),
        ];

        for dir in &directories {
            // 创建一个空文件来确保目录存在（某些存储后端需要）
            let placeholder_path = format!("{}/.keep", dir);
            self.operator
                .write(&placeholder_path, Vec::<u8>::new())
                .await
                .context("Failed to create directory placeholder")?;
        }

        Ok(())
    }

    /// 清理用户数据（用于删除账户）
    pub async fn cleanup_user_data(&self, user_id: &str) -> Result<()> {
        let user_prefix = format!("{}/", user_id);
        
        // 列出用户的所有文件
        let entries = self.operator.list(&user_prefix).await?;
        
        // 删除所有文件
        for entry in entries {
            self.operator.delete(&entry.path()).await?;
        }
        
        Ok(())
    }

    /// 获取存储使用统计
    pub async fn get_storage_stats(&self, user_id: &str) -> Result<StorageStats> {
        let user_prefix = format!("{}/", user_id);
        let entries = self.operator.list(&user_prefix).await?;
        
        let mut stats = StorageStats::default();
        
        for entry in entries {
            let path = entry.path();
            if let Ok(metadata) = self.operator.stat(&path).await {
                stats.total_files += 1;
                stats.total_size += metadata.content_length();
                
                if path.contains("/oplog/") {
                    stats.oplog_files += 1;
                    stats.oplog_size += metadata.content_length();
                } else if path.contains("/snapshots/") {
                    stats.snapshot_files += 1;
                    stats.snapshot_size += metadata.content_length();
                } else if path.contains("/data/") {
                    stats.data_files += 1;
                    stats.data_size += metadata.content_length();
                }
            }
        }
        
        Ok(stats)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_files: u64,
    pub total_size: u64,
    pub oplog_files: u64,
    pub oplog_size: u64,
    pub snapshot_files: u64,
    pub snapshot_size: u64,
    pub data_files: u64,
    pub data_size: u64,
} 