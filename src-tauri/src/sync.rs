use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use opendal::Operator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::storage::ClipboardItem;

/// LWW-Oplog 中的剪切板条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncClipboardItem {
    pub id: String,
    pub content_type: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub metadata: ItemMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMetadata {
    pub source_device: String,
    pub source_app: Option<String>,
    pub content_hash: Option<String>, // 用于大文件的内容引用
}

/// 操作类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OpType {
    #[serde(rename = "ADD")]
    Add,
    #[serde(rename = "DELETE")]
    Delete,
}

/// LWW-Oplog 中的操作记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub op_id: String,
    pub op_type: OpType,
    pub target_id: String,
    pub timestamp: DateTime<Utc>,
    pub device_id: String, // 用于打破时间戳平局
    pub payload: Option<SyncClipboardItem>, // ADD操作时包含完整数据，DELETE时为None
}

impl Operation {
    /// 创建新的 ADD 操作
    pub fn new_add(item: SyncClipboardItem, device_id: String) -> Self {
        Self {
            op_id: Uuid::new_v4().to_string(),
            op_type: OpType::Add,
            target_id: item.id.clone(),
            timestamp: Utc::now(),
            device_id,
            payload: Some(item),
        }
    }

    /// 创建新的 DELETE 操作
    pub fn new_delete(target_id: String, device_id: String) -> Self {
        Self {
            op_id: Uuid::new_v4().to_string(),
            op_type: OpType::Delete,
            target_id,
            timestamp: Utc::now(),
            device_id,
            payload: None,
        }
    }

    /// 比较两个操作的时间戳，实现 LWW 逻辑
    /// 返回 true 表示 self 比 other 更新（应该获胜）
    pub fn is_newer_than(&self, other: &Operation) -> bool {
        match self.timestamp.cmp(&other.timestamp) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => {
                // 时间戳相同时，比较设备ID的字典序
                self.device_id > other.device_id
            }
        }
    }
}

/// 状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub items: Vec<SyncClipboardItem>,
    pub snapshot_timestamp: DateTime<Utc>,
    pub last_op_timestamp: DateTime<Utc>,
    pub device_id: String,
}

/// 同步配置
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub user_id: String,
    pub device_id: String,
    pub storage_operator: Operator,
    pub sync_interval_seconds: u64,
}

/// 同步状态
#[derive(Debug, Default)]
pub struct SyncState {
    pub items: HashMap<String, SyncClipboardItem>, // 当前状态，key为item_id
    pub last_sync_timestamp: Option<DateTime<Utc>>, // 上次同步的时间戳
    pub pending_ops: Vec<Operation>, // 待上传的操作队列
}

/// 核心同步引擎
pub struct SyncEngine {
    config: SyncConfig,
    state: RwLock<SyncState>,
    is_syncing: Mutex<bool>,
}

impl SyncEngine {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            state: RwLock::new(SyncState::default()),
            is_syncing: Mutex::new(false),
        }
    }

    /// 本地添加操作（当用户复制新内容时）
    pub async fn local_add(&self, item: SyncClipboardItem) -> Result<()> {
        let op = Operation::new_add(item.clone(), self.config.device_id.clone());
        
        // 立即更新本地状态
        {
            let mut state = self.state.write().await;
            state.items.insert(item.id.clone(), item);
            state.pending_ops.push(op);
        }

        // 异步上传操作
        self.upload_pending_ops().await?;
        
        Ok(())
    }

    /// 本地删除操作（当用户删除历史记录时）
    pub async fn local_delete(&self, item_id: String) -> Result<()> {
        let op = Operation::new_delete(item_id.clone(), self.config.device_id.clone());
        
        // 立即更新本地状态
        {
            let mut state = self.state.write().await;
            state.items.remove(&item_id);
            state.pending_ops.push(op);
        }

        // 异步上传操作
        self.upload_pending_ops().await?;
        
        Ok(())
    }

    /// 获取当前所有剪切板项目
    pub async fn get_all_items(&self) -> Vec<SyncClipboardItem> {
        let state = self.state.read().await;
        state.items.values().cloned().collect()
    }

    /// 上传待处理的操作到对象存储
    async fn upload_pending_ops(&self) -> Result<()> {
        let ops_to_upload = {
            let mut state = self.state.write().await;
            let ops = state.pending_ops.clone();
            state.pending_ops.clear();
            ops
        };

        for op in ops_to_upload {
            let path = format!("{}/oplog/{}.json", self.config.user_id, op.op_id);
            let content = serde_json::to_vec(&op)
                .context("Failed to serialize operation")?;
            
            self.config.storage_operator
                .write(&path, content)
                .await
                .context("Failed to upload operation")?;
        }

        Ok(())
    }

    /// 立即同步（前端调用）
    pub async fn sync_now(&self) -> Result<()> {
        self.sync().await
    }

    /// 获取同步状态（前端调用）
    pub async fn get_status(&self) -> Result<serde_json::Value> {
        let state = self.state.read().await;
        let is_syncing = *self.is_syncing.lock().await;
        
        Ok(serde_json::json!({
            "item_count": state.items.len(),
            "is_syncing": is_syncing,
            "initialized": true,
            "last_sync": state.last_sync_timestamp,
            "pending_ops": state.pending_ops.len()
        }))
    }

    /// 执行完整同步
    pub async fn sync(&self) -> Result<()> {
        // 防止并发同步
        let _lock = self.is_syncing.lock().await;

        tracing::info!("开始同步");

        // 1. 首次同步：加载快照
        if self.state.read().await.last_sync_timestamp.is_none() {
            self.initial_sync().await?;
        }

        // 2. 增量同步：拉取新的操作日志
        self.incremental_sync().await?;

        tracing::info!("同步完成");
        Ok(())
    }

    /// 首次同步：加载快照和增量操作
    async fn initial_sync(&self) -> Result<()> {
        tracing::info!("执行首次同步");

        // 1. 获取最新快照
        let snapshot = self.load_latest_snapshot().await?;

        // 2. 应用快照到本地状态
        if let Some(snapshot) = snapshot {
            let mut state = self.state.write().await;
            state.items.clear();
            for item in snapshot.items {
                state.items.insert(item.id.clone(), item);
            }
            state.last_sync_timestamp = Some(snapshot.last_op_timestamp);
        }

        // 3. 拉取快照之后的增量操作
        self.incremental_sync().await?;

        Ok(())
    }

    /// 增量同步：拉取并应用新的操作日志
    async fn incremental_sync(&self) -> Result<()> {
        let last_sync_time = self.state.read().await.last_sync_timestamp;

        // 列出远端操作日志
        let ops = self.fetch_operations_since(last_sync_time).await?;

        if ops.is_empty() {
            return Ok(());
        }

        tracing::info!("发现 {} 个新操作", ops.len());

        // 应用操作到本地状态（LWW 冲突解决）
        self.apply_operations(ops).await?;

        Ok(())
    }

    /// 从对象存储获取最新快照
    async fn load_latest_snapshot(&self) -> Result<Option<Snapshot>> {
        let latest_path = format!("{}/snapshots/latest.json", self.config.user_id);
        
        match self.config.storage_operator.read(&latest_path).await {
            Ok(data) => {
                let snapshot_info: serde_json::Value = serde_json::from_slice(data.to_bytes().as_ref())?;
                let snapshot_path = snapshot_info["snapshot_path"].as_str()
                    .context("Invalid snapshot info")?;
                
                let snapshot_data = self.config.storage_operator.read(snapshot_path).await?;
                let snapshot: Snapshot = serde_json::from_slice(snapshot_data.to_bytes().as_ref())?;
                
                tracing::info!("加载快照，包含 {} 个项目", snapshot.items.len());
                Ok(Some(snapshot))
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => {
                tracing::info!("未找到快照，将从头开始同步");
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// 获取指定时间之后的操作日志
    async fn fetch_operations_since(&self, since: Option<DateTime<Utc>>) -> Result<Vec<Operation>> {
        let oplog_path = format!("{}/oplog/", self.config.user_id);
        
        // 列出所有操作文件
        let mut ops = Vec::new();
        let entries = self.config.storage_operator
            .list(&oplog_path)
            .await?;

        for entry in entries {
            let op_data = self.config.storage_operator
                .read(&entry.path())
                .await?;
            
            let op: Operation = serde_json::from_slice(op_data.to_bytes().as_ref())?;
            
            // 过滤出指定时间之后的操作
            if let Some(since_time) = since {
                if op.timestamp <= since_time {
                    continue;
                }
            }
            
            ops.push(op);
        }

        // 按时间戳排序
        ops.sort_by(|a, b| {
            a.timestamp.cmp(&b.timestamp)
                .then_with(|| a.device_id.cmp(&b.device_id))
        });

        Ok(ops)
    }

    /// 应用操作到本地状态，实现 LWW 冲突解决
    async fn apply_operations(&self, ops: Vec<Operation>) -> Result<()> {
        let mut state = self.state.write().await;
        let mut latest_timestamp = state.last_sync_timestamp;

        for op in ops {
            match op.op_type {
                OpType::Add => {
                    if let Some(item) = &op.payload {
                        // 检查是否存在冲突
                        if let Some(existing_item) = state.items.get(&op.target_id) {
                            // 需要比较时间戳来决定保留哪个版本
                            // 这里简化处理，假设较新的时间戳获胜
                            if item.created_at >= existing_item.created_at {
                                state.items.insert(op.target_id.clone(), item.clone());
                            }
                        } else {
                            state.items.insert(op.target_id.clone(), item.clone());
                        }
                    }
                }
                OpType::Delete => {
                    // 检查删除操作是否应该被应用
                    if let Some(existing_item) = state.items.get(&op.target_id) {
                        // 如果删除操作的时间戳晚于项目的创建时间，则删除
                        if op.timestamp >= existing_item.created_at {
                            state.items.remove(&op.target_id);
                        }
                    }
                }
            }

            // 更新最后同步时间戳
            if latest_timestamp.is_none() || op.timestamp > latest_timestamp.unwrap() {
                latest_timestamp = Some(op.timestamp);
            }
        }

        state.last_sync_timestamp = latest_timestamp;
        Ok(())
    }

    /// 生成快照（通常由后台任务调用）
    pub async fn create_snapshot(&self) -> Result<()> {
        let state = self.state.read().await;
        
        let snapshot = Snapshot {
            items: state.items.values().cloned().collect(),
            snapshot_timestamp: Utc::now(),
            last_op_timestamp: state.last_sync_timestamp.unwrap_or_else(Utc::now),
            device_id: self.config.device_id.clone(),
        };

        let timestamp_str = snapshot.snapshot_timestamp.format("%Y%m%d_%H%M%S").to_string();
        let snapshot_path = format!("{}/snapshots/{}_snapshot.json", self.config.user_id, timestamp_str);
        
        // 上传快照
        let snapshot_data = serde_json::to_vec(&snapshot)?;
        self.config.storage_operator
            .write(&snapshot_path, snapshot_data)
            .await?;

        // 更新 latest.json
        let latest_info = serde_json::json!({
            "snapshot_path": snapshot_path,
            "timestamp": snapshot.snapshot_timestamp
        });
        let latest_path = format!("{}/snapshots/latest.json", self.config.user_id);
        self.config.storage_operator
            .write(&latest_path, serde_json::to_vec(&latest_info)?)
            .await?;

        tracing::info!("快照已创建: {}", snapshot_path);
        Ok(())
    }

    /// 启动后台同步任务
    pub async fn start_background_sync(&self) -> Result<()> {
        let interval = tokio::time::Duration::from_secs(self.config.sync_interval_seconds);
        let mut timer = tokio::time::interval(interval);

        loop {
            timer.tick().await;
            
            if let Err(e) = self.sync().await {
                tracing::error!("同步失败: {}", e);
            }
        }
    }
}

/// 辅助函数：从本地 ClipboardItem 转换为 SyncClipboardItem
impl From<&crate::storage::ClipboardItem> for SyncClipboardItem {
    fn from(item: &crate::storage::ClipboardItem) -> Self {
        Self {
            id: item.id.clone(),
            content_type: item.item_type.clone(),
            content: item.content.clone(),
            created_at: DateTime::from_timestamp(item.timestamp as i64, 0)
                .unwrap_or_else(Utc::now),
            metadata: ItemMetadata {
                source_device: "unknown".to_string(), // TODO: 从系统获取设备名
                source_app: None,
                content_hash: None,
            },
        }
    }
}

/// 辅助函数：从 SyncClipboardItem 转换为本地 ClipboardItem
impl From<&SyncClipboardItem> for ClipboardItem {
    fn from(item: &SyncClipboardItem) -> Self {
        Self {
            id: item.id.clone(),
            content: item.content.clone(),
            timestamp: item.created_at.timestamp() as u64,
            item_type: item.content_type.clone(),
            size: Some(item.content.len() as u64),
            file_paths: None,
            file_types: None,
        }
    }
} 