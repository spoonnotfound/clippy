use anyhow::Result;
use clippy_lib::sync::{SyncEngine, SyncConfig, SyncClipboardItem, ItemMetadata};
use clippy_lib::storage_adapter::{StorageConfig, StorageBackend};
use chrono::Utc;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    println!("🚀 Clippy 同步功能演示");
    
    // 创建本地文件系统存储配置（用于演示）
    let storage_config = StorageConfig {
        backend: StorageBackend::FileSystem {
            root_path: "./demo_sync_data".to_string(),
        },
        retry_attempts: 3,
        timeout_seconds: 30,
    };

    // 验证存储配置
    println!("📁 验证存储配置...");
    storage_config.validate().await?;
    println!("✅ 存储配置验证成功");

    // 创建两个模拟设备的同步引擎
    let device1_config = SyncConfig {
        user_id: "demo_user".to_string(),
        device_id: "device_1".to_string(),
        storage_operator: storage_config.create_operator()?,
        sync_interval_seconds: 5,
    };

    let device2_config = SyncConfig {
        user_id: "demo_user".to_string(),
        device_id: "device_2".to_string(),
        storage_operator: storage_config.create_operator()?,
        sync_interval_seconds: 5,
    };

    let sync_engine1 = Arc::new(SyncEngine::new(device1_config));
    let sync_engine2 = Arc::new(SyncEngine::new(device2_config));

    println!("🔄 创建了两个同步引擎（模拟两台设备）");

    // 设备1添加一些剪切板项目
    println!("\n📋 设备1 添加剪切板内容...");
    
    let item1 = SyncClipboardItem {
        id: uuid::Uuid::new_v4().to_string(),
        content_type: "text/plain".to_string(),
        content: "Hello from Device 1!".to_string(),
        created_at: Utc::now(),
        metadata: ItemMetadata {
            source_device: "device_1".to_string(),
            source_app: Some("Terminal".to_string()),
            content_hash: None,
        },
    };

    sync_engine1.local_add(item1.clone()).await?;
    println!("✅ 设备1 添加了: {}", item1.content);

    // 等待一下，然后添加更多内容
    sleep(Duration::from_secs(1)).await;

    let item2 = SyncClipboardItem {
        id: uuid::Uuid::new_v4().to_string(),
        content_type: "text/plain".to_string(),
        content: "Second item from Device 1".to_string(),
        created_at: Utc::now(),
        metadata: ItemMetadata {
            source_device: "device_1".to_string(),
            source_app: Some("Browser".to_string()),
            content_hash: None,
        },
    };

    sync_engine1.local_add(item2.clone()).await?;
    println!("✅ 设备1 添加了: {}", item2.content);

    // 设备2执行同步
    println!("\n🔄 设备2 执行同步...");
    sync_engine2.sync().await?;

    let device2_items = sync_engine2.get_all_items().await;
    println!("📱 设备2 同步后有 {} 个项目:", device2_items.len());
    for item in &device2_items {
        println!("  - {}", item.content);
    }

    // 设备2添加新内容
    println!("\n📋 设备2 添加新内容...");
    
    let item3 = SyncClipboardItem {
        id: uuid::Uuid::new_v4().to_string(),
        content_type: "text/plain".to_string(),
        content: "Hello from Device 2!".to_string(),
        created_at: Utc::now(),
        metadata: ItemMetadata {
            source_device: "device_2".to_string(),
            source_app: Some("Code Editor".to_string()),
            content_hash: None,
        },
    };

    sync_engine2.local_add(item3.clone()).await?;
    println!("✅ 设备2 添加了: {}", item3.content);

    // 设备1再次同步
    println!("\n🔄 设备1 执行同步...");
    sync_engine1.sync().await?;

    let device1_items = sync_engine1.get_all_items().await;
    println!("💻 设备1 同步后有 {} 个项目:", device1_items.len());
    for item in &device1_items {
        println!("  - {} (来自 {})", item.content, item.metadata.source_device);
    }

    // 演示删除操作
    println!("\n🗑️  设备1 删除一个项目...");
    sync_engine1.local_delete(item1.id.clone()).await?;
    println!("✅ 设备1 删除了: {}", item1.content);

    // 设备2同步删除操作
    println!("\n🔄 设备2 同步删除操作...");
    sync_engine2.sync().await?;

    let device2_items_after_delete = sync_engine2.get_all_items().await;
    println!("📱 设备2 同步后有 {} 个项目:", device2_items_after_delete.len());
    for item in &device2_items_after_delete {
        println!("  - {} (来自 {})", item.content, item.metadata.source_device);
    }

    // 创建快照演示
    println!("\n📸 创建快照...");
    sync_engine1.create_snapshot().await?;
    println!("✅ 快照创建成功");

    // 验证最终状态一致性
    println!("\n🔍 验证最终状态一致性...");
    let final_items1 = sync_engine1.get_all_items().await;
    let final_items2 = sync_engine2.get_all_items().await;

    println!("设备1 最终有 {} 个项目", final_items1.len());
    println!("设备2 最终有 {} 个项目", final_items2.len());

    if final_items1.len() == final_items2.len() {
        println!("✅ 两设备的项目数量一致！");
        
        // 检查内容是否一致
        let mut content_match = true;
        for item1 in &final_items1 {
            if !final_items2.iter().any(|item2| item2.id == item1.id && item2.content == item1.content) {
                content_match = false;
                break;
            }
        }
        
        if content_match {
            println!("✅ 两设备的项目内容完全一致！");
            println!("🎉 同步演示成功完成！");
        } else {
            println!("❌ 两设备的项目内容不一致");
        }
    } else {
        println!("❌ 两设备的项目数量不一致");
    }

    println!("\n📊 演示完成。你可以查看 './demo_sync_data' 目录了解存储结构。");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_basic_sync_functionality() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage_config = StorageConfig {
            backend: StorageBackend::FileSystem {
                root_path: temp_dir.path().to_string_lossy().to_string(),
            },
            retry_attempts: 1,
            timeout_seconds: 10,
        };

        let sync_config1 = SyncConfig {
            user_id: "test_user".to_string(),
            device_id: "test_device_1".to_string(),
            storage_operator: storage_config.create_operator()?,
            sync_interval_seconds: 1,
        };

        let sync_config2 = SyncConfig {
            user_id: "test_user".to_string(),
            device_id: "test_device_2".to_string(),
            storage_operator: storage_config.create_operator()?,
            sync_interval_seconds: 1,
        };

        let engine1 = Arc::new(SyncEngine::new(sync_config1));
        let engine2 = Arc::new(SyncEngine::new(sync_config2));

        // 测试添加项目
        let test_item = SyncClipboardItem {
            id: uuid::Uuid::new_v4().to_string(),
            content_type: "text/plain".to_string(),
            content: "Test content".to_string(),
            created_at: Utc::now(),
            metadata: ItemMetadata {
                source_device: "test_device_1".to_string(),
                source_app: None,
                content_hash: None,
            },
        };

        engine1.local_add(test_item.clone()).await?;
        engine2.sync().await?;

        let items = engine2.get_all_items().await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].content, "Test content");

        // 测试删除项目
        engine1.local_delete(test_item.id).await?;
        engine2.sync().await?;

        let items_after_delete = engine2.get_all_items().await;
        assert_eq!(items_after_delete.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_conflict_resolution() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage_config = StorageConfig {
            backend: StorageBackend::FileSystem {
                root_path: temp_dir.path().to_string_lossy().to_string(),
            },
            retry_attempts: 1,
            timeout_seconds: 10,
        };

        let sync_config1 = SyncConfig {
            user_id: "test_user".to_string(),
            device_id: "device_a".to_string(), // 字典序较小
            storage_operator: storage_config.create_operator()?,
            sync_interval_seconds: 1,
        };

        let sync_config2 = SyncConfig {
            user_id: "test_user".to_string(),
            device_id: "device_b".to_string(), // 字典序较大
            storage_operator: storage_config.create_operator()?,
            sync_interval_seconds: 1,
        };

        let engine1 = Arc::new(SyncEngine::new(sync_config1));
        let engine2 = Arc::new(SyncEngine::new(sync_config2));

        // 创建两个有相同ID但内容不同的项目（模拟冲突）
        let item_id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();

        let item_from_device1 = SyncClipboardItem {
            id: item_id.clone(),
            content_type: "text/plain".to_string(),
            content: "Content from device A".to_string(),
            created_at: timestamp,
            metadata: ItemMetadata {
                source_device: "device_a".to_string(),
                source_app: None,
                content_hash: None,
            },
        };

        let item_from_device2 = SyncClipboardItem {
            id: item_id.clone(),
            content_type: "text/plain".to_string(),
            content: "Content from device B".to_string(),
            created_at: timestamp, // 相同的时间戳
            metadata: ItemMetadata {
                source_device: "device_b".to_string(),
                source_app: None,
                content_hash: None,
            },
        };

        // 两个设备同时添加冲突的项目
        engine1.local_add(item_from_device1).await?;
        engine2.local_add(item_from_device2).await?;

        // 同步
        engine1.sync().await?;
        engine2.sync().await?;

        // 验证冲突解决结果（device_b 应该获胜，因为字典序更大）
        let items1 = engine1.get_all_items().await;
        let items2 = engine2.get_all_items().await;

        assert_eq!(items1.len(), 1);
        assert_eq!(items2.len(), 1);
        assert_eq!(items1[0].content, "Content from device B");
        assert_eq!(items2[0].content, "Content from device B");

        Ok(())
    }
} 