# Clippy 同步功能快速开始指南

本指南将帮你快速启用 Clippy 的多端同步功能。

## 🚀 快速开始

### 1. 选择存储后端

#### 选项A：本地文件系统（开发/测试）

```bash
# 创建配置文件
cat > storage_config.json << EOF
{
  "backend": {
    "FileSystem": {
      "root_path": "./clippy_sync_data"
    }
  },
  "retry_attempts": 3,
  "timeout_seconds": 30
}
EOF
```

#### 选项B：MinIO（推荐用于自建）

1. 启动 MinIO 服务器：
```bash
# 使用 Docker 启动 MinIO
docker run -p 9000:9000 -p 9001:9001 \
  -e "MINIO_ROOT_USER=admin" \
  -e "MINIO_ROOT_PASSWORD=password123" \
  quay.io/minio/minio server /data --console-address ":9001"
```

2. 创建配置文件：
```bash
cat > storage_config.json << EOF
{
  "backend": {
    "S3Compatible": {
      "bucket": "clippy",
      "endpoint": "http://localhost:9000",
      "access_key_id": "admin",
      "secret_access_key": "password123",
      "region": "us-east-1"
    }
  },
  "retry_attempts": 3,
  "timeout_seconds": 30
}
EOF
```

3. 使用 MinIO 控制台创建存储桶：
   - 访问 http://localhost:9001
   - 登录：用户名 `admin`，密码 `password123`
   - 创建名为 `clippy` 的存储桶

#### 选项C：AWS S3

```bash
# 设置环境变量
export AWS_S3_BUCKET=my-clippy-bucket
export AWS_REGION=us-east-1
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key

# 或者创建配置文件
cat > storage_config.json << EOF
{
  "backend": {
    "S3": {
      "bucket": "my-clippy-bucket",
      "region": "us-east-1",
      "access_key_id": "your_access_key",
      "secret_access_key": "your_secret_key",
      "endpoint": null
    }
  },
  "retry_attempts": 3,
  "timeout_seconds": 30
}
EOF
```

### 2. 设置用户ID

```bash
# 设置你的唯一用户ID（可以是邮箱、UUID等）
export CLIPPY_USER_ID=your_email@example.com

# 或者使用随机生成的ID
export CLIPPY_USER_ID=$(uuidgen)
```

### 3. 启动应用

```bash
# 编译并运行
cargo tauri dev

# 或者构建生产版本
cargo tauri build
```

### 4. 验证同步功能

1. 在第一台设备上复制一些文本
2. 在第二台设备上启动 Clippy（使用相同的存储配置和用户ID）
3. 等待15秒左右，第二台设备应该会同步第一台设备的剪切板历史

## 🧪 运行演示

我们提供了一个完整的演示程序：

```bash
# 进入示例目录
cd examples

# 运行同步演示（需要先添加测试依赖）
cargo add tempfile --dev
cargo test test_basic_sync_functionality
```

## 🔧 高级配置

### 自定义同步间隔

通过环境变量设置同步间隔（秒）：
```bash
export CLIPPY_SYNC_INTERVAL=10  # 每10秒同步一次
```

### 启用调试日志

```bash
export RUST_LOG=debug
cargo tauri dev
```

### 配置存储验证

在启动应用前验证存储配置：

```bash
# 使用 Tauri 命令验证
# （需要在前端调用 configure_storage 命令）
```

## 🌐 多种存储后端配置

### 阿里云 OSS

```json
{
  "backend": {
    "Oss": {
      "bucket": "my-clippy-bucket",
      "region": "oss-cn-hangzhou",
      "access_key_id": "LTAI4G***",
      "access_key_secret": "your_secret_key",
      "endpoint": null
    }
  },
  "retry_attempts": 3,
  "timeout_seconds": 30
}
```

### 腾讯云 COS

```json
{
  "backend": {
    "Cos": {
      "bucket": "my-clippy-bucket-1234567890",
      "region": "ap-guangzhou",
      "secret_id": "AKIDrAr7***",
      "secret_key": "your_secret_key"
    }
  },
  "retry_attempts": 3,
  "timeout_seconds": 30
}
```

### Azure Blob Storage

```json
{
  "backend": {
    "AzBlob": {
      "container": "clippy",
      "account_name": "mystorageaccount",
      "account_key": "your_account_key"
    }
  },
  "retry_attempts": 3,
  "timeout_seconds": 30
}
```

## 📱 多设备使用

### 在第二台设备上设置

1. **使用相同的存储配置**：复制 `storage_config.json` 文件到第二台设备
2. **使用相同的用户ID**：设置相同的 `CLIPPY_USER_ID` 环境变量
3. **启动应用**：第二台设备会自动同步所有历史记录

### 注意事项

- 每台设备会自动生成唯一的设备ID（存储在 `./device_id` 文件中）
- 不要在多台设备上使用相同的设备ID
- 首次同步可能需要更长时间，后续会进行增量同步

## 🛠️ 故障排除

### 同步不工作

1. 检查网络连接
2. 验证存储配置：
   ```bash
   # 手动测试存储连接
   cargo run --bin test_storage_connection
   ```
3. 检查日志输出中的错误信息

### 存储权限问题

1. 确保存储桶存在且有正确权限
2. 验证访问密钥有效
3. 检查防火墙设置

### 冲突解决问题

- 同步使用 LWW (Last-Write-Wins) 算法
- 较新的时间戳获胜
- 时间戳相同时，设备ID字典序更大的获胜

## 📋 前端集成

### 调用同步命令

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// 手动触发同步
await invoke('sync_now');

// 获取同步状态
const status = await invoke('get_sync_status');
console.log('同步状态:', status);

// 配置存储后端
await invoke('configure_storage', {
  storageConfig: {
    backend: {
      FileSystem: {
        root_path: './my_sync_data'
      }
    },
    retry_attempts: 3,
    timeout_seconds: 30
  }
});
```

### 监听同步事件

```typescript
import { listen } from '@tauri-apps/api/event';

// 监听同步错误
await listen('sync-error', (event) => {
  console.error('同步错误:', event.payload);
});

// 监听剪切板更新（包括来自其他设备的更新）
await listen('clipboard-update', (event) => {
  console.log('剪切板更新:', event.payload);
});
```

## 🔒 安全建议

1. **不要硬编码凭据**：使用环境变量或安全密钥管理
2. **定期轮换密钥**：定期更新访问密钥
3. **网络加密**：始终使用 HTTPS 端点
4. **端到端加密**：考虑实现内容加密（计划中的功能）
5. **访问控制**：限制存储桶访问权限

## 🚧 已知限制

1. 目前不支持端到端加密（计划中）
2. 大文件支持有限（推荐小于10MB）
3. 同步间隔最短为5秒
4. 不支持实时推送（使用轮询机制）

## 📚 更多信息

- 查看 `SYNC_ARCHITECTURE.md` 了解详细架构
- 查看 `examples/storage_config_examples.json` 了解更多配置选项
- 运行 `examples/sync_demo.rs` 查看完整演示

## 🐛 问题反馈

如果遇到问题，请：

1. 检查日志输出
2. 验证存储配置
3. 提交 Issue 并包含错误日志和配置信息（移除敏感信息）

祝你使用愉快！🎉 