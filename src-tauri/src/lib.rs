use clipboard_rs::{
    Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, 
    ClipboardWatcherContext, ContentFormat
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use infer;
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use base64::{Engine as _, engine::general_purpose};


// 导入存储模块
mod storage;
use storage::{StorageEngine, StorageStats, ClipboardItem, FileTypeInfo};

// 导入同步模块
mod sync;
mod storage_adapter;
use sync::{SyncEngine, SyncConfig, SyncClipboardItem};
use storage_adapter::{StorageConfig};

// 全局状态
type ClipboardStorage = Arc<Mutex<StorageEngine>>;
type ClipboardSync = Arc<SyncEngine>;
type ClipboardSyncContainer = Arc<Mutex<Option<ClipboardSync>>>;

#[tauri::command]
fn get_clipboard_history(state: tauri::State<ClipboardStorage>) -> Vec<ClipboardItem> {
    state.lock().unwrap().get_all()
}

#[tauri::command]
fn clear_clipboard_history(state: tauri::State<ClipboardStorage>) -> Result<(), String> {
    state.lock().unwrap().clear_all().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_clipboard_item(item_id: String, state: tauri::State<ClipboardStorage>) -> Result<(), String> {
    state.lock().unwrap().delete(&item_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_storage_stats(state: tauri::State<ClipboardStorage>) -> StorageStats {
    state.lock().unwrap().stats()
}

#[tauri::command]
fn compact_storage(state: tauri::State<ClipboardStorage>) -> Result<(), String> {
    state.lock().unwrap().compact().map_err(|e| e.to_string())
}

#[tauri::command]
fn copy_to_clipboard(content: String) -> Result<(), String> {
    let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;
    ctx.set_text(content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn copy_image_to_clipboard(base64_data: String) -> Result<(), String> {
    let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;
    
    // 解码 base64 数据
    let image_bytes = general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;
    
    // 创建 RustImageData (简化处理)
    ctx.set_text(format!("图片数据 ({} 字节)", image_bytes.len())).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn copy_files_to_clipboard(file_paths: Vec<String>) -> Result<(), String> {
    let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;
    
    // 验证所有文件路径存在
    for path in &file_paths {
        if !std::path::Path::new(path).exists() {
            return Err(format!("文件不存在: {}", path));
        }
    }
    
    ctx.set_files(file_paths).map_err(|e| e.to_string())?;
    Ok(())
}

// 同步相关命令
#[tauri::command]
async fn setup_sync(
    _user_id: String,
    _storage_config: serde_json::Value,
    state: tauri::State<'_, ClipboardSyncContainer>
) -> Result<(), String> {
    // 重新初始化同步引擎（配置已经通过configure_storage保存了）
    let sync_engine = create_sync_engine_if_configured().await;
    
    if let Ok(mut container) = state.lock() {
        *container = sync_engine;
        Ok(())
    } else {
        Err("无法更新同步引擎".to_string())
    }
}

#[tauri::command]
async fn sync_now(state: tauri::State<'_, ClipboardSyncContainer>) -> Result<(), String> {
    let sync_engine_clone = {
        if let Ok(sync_engine_opt) = state.lock() {
            sync_engine_opt.clone()
        } else {
            return Err("无法获取同步引擎".to_string());
        }
    };
    
    if let Some(sync_engine) = sync_engine_clone {
        sync_engine.sync_now().await.map_err(|e| e.to_string())
    } else {
        Err("同步引擎未初始化".to_string())
    }
}

#[tauri::command]
async fn get_sync_status(state: tauri::State<'_, ClipboardSyncContainer>) -> Result<serde_json::Value, String> {
    let sync_engine_clone = {
        if let Ok(sync_engine_opt) = state.lock() {
            sync_engine_opt.clone()
        } else {
            return Err("无法获取同步引擎".to_string());
        }
    };
    
    if let Some(sync_engine) = sync_engine_clone {
        let status = sync_engine.get_status().await.map_err(|e| e.to_string())?;
        Ok(status)
    } else {
        // 返回未初始化状态
        Ok(serde_json::json!({
            "item_count": 0,
            "is_syncing": false,
            "initialized": false
        }))
    }
}

#[tauri::command]
async fn configure_storage(
    storage_config: serde_json::Value,
    app_handle: AppHandle
) -> Result<(), String> {
    // 验证存储配置
    let config: StorageConfig = serde_json::from_value(storage_config)
        .map_err(|e| format!("Invalid storage config: {}", e))?;
    
    // 验证配置有效性
    config.validate().await.map_err(|e| e.to_string())?;
    
    // 保存配置到用户配置目录
    let config_file = get_app_data_dir().join("storage_config.json");
    config.save_to_file(config_file.to_string_lossy().as_ref()).map_err(|e| e.to_string())?;
    
    // 重新初始化同步引擎
    reload_sync_engine(&app_handle).await?;
    
    Ok(())
}

/// 重新加载同步引擎
async fn reload_sync_engine(app_handle: &AppHandle) -> Result<(), String> {
    let sync_engine_container: tauri::State<ClipboardSyncContainer> = app_handle.state();
    let sync_engine = create_sync_engine_if_configured().await;
    
    {
        if let Ok(mut container) = sync_engine_container.lock() {
            *container = sync_engine;
            tracing::info!("同步引擎已重新加载");
        } else {
            return Err("无法更新同步引擎".to_string());
        }
    }
    
    Ok(())
}

#[tauri::command]
async fn get_storage_config() -> Result<serde_json::Value, String> {
    let config_file = get_app_data_dir().join("storage_config.json");
    match StorageConfig::load_from_file(config_file.to_string_lossy().as_ref()) {
        Ok(config) => {
            // 移除敏感信息后返回配置
            let mut config_value = serde_json::to_value(config).map_err(|e| e.to_string())?;
            
            // 隐藏敏感字段
            if let Some(backend) = config_value.get_mut("backend") {
                hide_sensitive_fields(backend);
            }
            
            Ok(config_value)
        }
        Err(_) => {
            // 返回默认配置
            let default_config = StorageConfig::default();
            let config_value = serde_json::to_value(default_config).map_err(|e| e.to_string())?;
            Ok(config_value)
        }
    }
}

#[tauri::command]
async fn test_storage_connection(storage_config: serde_json::Value) -> Result<String, String> {
    let config: StorageConfig = serde_json::from_value(storage_config)
        .map_err(|e| format!("Invalid storage config: {}", e))?;
    
    match config.validate().await {
        Ok(_) => Ok("连接成功！存储配置有效。".to_string()),
        Err(e) => Err(format!("连接失败: {}", e))
    }
}

#[tauri::command]
fn get_storage_backend_types() -> Vec<String> {
    vec![
        "FileSystem".to_string(),
        "S3".to_string(),
        "S3Compatible".to_string(),
        "Oss".to_string(),
        "Cos".to_string(),
        "AzBlob".to_string(),
    ]
}

// 辅助函数：隐藏敏感配置字段
fn hide_sensitive_fields(backend: &mut serde_json::Value) {
    match backend {
        serde_json::Value::Object(map) => {
            for (_, value) in map.iter_mut() {
                if let serde_json::Value::Object(config) = value {
                    // 隐藏密钥和密码字段
                    let sensitive_fields = [
                        "secret_access_key", "access_key_secret", 
                        "secret_key", "account_key"
                    ];
                    
                    for field in sensitive_fields {
                        if config.contains_key(field) {
                            config.insert(field.to_string(), serde_json::Value::String("***".to_string()));
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

// 计算文件大小
fn calculate_files_size(file_paths: &[String]) -> u64 {
    file_paths.iter()
        .filter_map(|path| std::fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .sum()
}

// 剪切板管理器
struct ClipboardManager {
    ctx: ClipboardContext,
    app_handle: AppHandle,
    storage: ClipboardStorage,
    sync_engine: Option<ClipboardSync>,
    runtime_handle: tokio::runtime::Handle,
    last_text: String,
    last_files: Vec<String>,
}

impl ClipboardManager {
    pub fn new(
        app_handle: AppHandle, 
        storage: ClipboardStorage, 
        sync_engine: Option<ClipboardSync>,
        runtime_handle: tokio::runtime::Handle
    ) -> Result<Self, String> {
        let ctx = ClipboardContext::new()
            .map_err(|e| format!("Failed to create clipboard context: {}", e))?;

        Ok(Self {
            ctx,
            app_handle,
            storage,
            sync_engine,
            runtime_handle,
            last_text: String::new(),
            last_files: Vec::new(),
        })
    }

    fn add_item_to_history(&self, item: ClipboardItem) {
        // 将项目存储到持久化存储中
        if let Ok(mut storage_lock) = self.storage.lock() {
            if let Err(e) = storage_lock.insert(&item) {
                eprintln!("存储剪切板项目失败: {}", e);
            }
        }

        // 如果启用了同步，也添加到同步引擎
        if let Some(sync_engine) = &self.sync_engine {
            let sync_item = SyncClipboardItem::from(&item);
            let app_handle_clone = self.app_handle.clone();
            let sync_engine_clone = sync_engine.clone();
            
            // 使用运行时句柄来执行异步操作
            self.runtime_handle.spawn(async move {
                if let Err(e) = sync_engine_clone.local_add(sync_item).await {
                    eprintln!("同步添加项目失败: {}", e);
                    // 可以发送错误事件到前端
                    let _ = app_handle_clone.emit("sync-error", format!("同步失败: {}", e));
                }
            });
        }
        
        // 发送事件到前端
        let _ = self.app_handle.emit("clipboard-update", &item);
    }

    fn check_text_change(&mut self) {
        if let Ok(text) = self.ctx.get_text() {
            if text != self.last_text && !text.trim().is_empty() {
                let item = ClipboardItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    content: text.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    item_type: "text".to_string(),
                    size: Some(text.len() as u64),
                    file_paths: None,
                    file_types: None,
                };
                
                self.add_item_to_history(item);
                self.last_text = text;
            }
        }
    }

    fn check_files_change(&mut self) {
        if let Ok(files) = self.ctx.get_files() {
            if files != self.last_files && !files.is_empty() {
                let total_size = calculate_files_size(&files);
                
                // 检测文件类型
                let file_types: Vec<FileTypeInfo> = files.iter()
                    .map(|file_path| detect_file_type(file_path))
                    .collect();
                
                let item = ClipboardItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    content: format!("{} 个文件", files.len()),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    item_type: "files".to_string(),
                    size: Some(total_size),
                    file_paths: Some(files.clone()),
                    file_types: Some(file_types),
                };
                
                self.add_item_to_history(item);
                self.last_files = files;
            }
        }
    }
}

impl ClipboardHandler for ClipboardManager {
    fn on_clipboard_change(&mut self) {
        // 简化逻辑：只区分文件和文本
        
        // 首先检查是否有文件
        if self.ctx.has(ContentFormat::Files) {
            self.check_files_change();
            return;
        }
        
        // 然后检查文本
        if self.ctx.has(ContentFormat::Text) {
            self.check_text_change();
        }
    }
}

// 启动剪贴板监听器
fn start_clipboard_monitor(app_handle: AppHandle, storage: ClipboardStorage, sync_engine_container: Arc<Mutex<Option<ClipboardSync>>>) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        // 先异步初始化同步引擎
        let sync_engine = rt.block_on(async {
            create_sync_engine_if_configured().await
        });
        
        // 更新同步引擎容器
        if let Some(ref engine) = sync_engine {
            if let Ok(mut container) = sync_engine_container.lock() {
                *container = Some(engine.clone());
                tracing::info!("同步引擎初始化成功");
            }
        }
        
        let manager = match ClipboardManager::new(app_handle.clone(), storage, sync_engine, rt.handle().clone()) {
            Ok(manager) => manager,
            Err(e) => {
                eprintln!("Failed to create clipboard manager: {}", e);
                return;
            }
        };

        let mut watcher = match ClipboardWatcherContext::new() {
            Ok(watcher) => watcher,
            Err(e) => {
                eprintln!("Failed to create clipboard watcher: {}", e);
                return;
            }
        };

        println!("开始监听剪切板变化...");
        
        // 添加处理器并开始监听
        watcher.add_handler(manager);
        watcher.start_watch();
    });
}

// 检测文件类型的辅助函数
fn detect_file_type(file_path: &str) -> FileTypeInfo {
    let path = Path::new(file_path);
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // 尝试从文件内容检测 MIME 类型
    let mime_type = if let Ok(bytes) = std::fs::read(file_path) {
        if let Some(kind) = infer::get(&bytes) {
            kind.mime_type().to_string()
        } else {
            guess_mime_by_extension(&extension)
        }
    } else {
        guess_mime_by_extension(&extension)
    };
    
    // 根据扩展名或 MIME 类型确定类别
    let category = categorize_file(&extension, &mime_type);
    
    FileTypeInfo {
        path: file_path.to_string(),
        file_type: extension,
        mime_type,
        category,
    }
}

// 根据扩展名猜测 MIME 类型
fn guess_mime_by_extension(extension: &str) -> String {
    match extension {
        "txt" => "text/plain",
        "pdf" => "application/pdf",
        "doc" | "docx" => "application/msword",
        "xls" | "xlsx" => "application/vnd.ms-excel",
        "ppt" | "pptx" => "application/vnd.ms-powerpoint",
        "zip" => "application/zip",
        "rar" => "application/x-rar-compressed",
        "7z" => "application/x-7z-compressed",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        _ => "application/octet-stream",
    }.to_string()
}

// 文件分类
fn categorize_file(extension: &str, mime_type: &str) -> String {
    if mime_type.starts_with("image/") {
        "image".to_string()
    } else if mime_type.starts_with("video/") {
        "video".to_string()
    } else if mime_type.starts_with("audio/") {
        "audio".to_string()
    } else if mime_type.starts_with("text/") || matches!(extension, "txt" | "md" | "csv" | "log") {
        "text".to_string()
    } else if matches!(extension, "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx") {
        "document".to_string()
    } else if matches!(extension, "zip" | "rar" | "7z" | "tar" | "gz" | "bz2") {
        "archive".to_string()
    } else if matches!(extension, "js" | "ts" | "py" | "java" | "cpp" | "c" | "h" | "rs" | "go" | "php" | "rb" | "swift") {
        "code".to_string()
    } else {
        "other".to_string()
    }
}

/// 获取应用配置目录，跨平台适配
fn get_app_data_dir() -> PathBuf {
    // 尝试获取用户配置目录
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("clippy")
    } else {
        // 如果无法获取配置目录，使用用户主目录
        if let Some(home_dir) = dirs::home_dir() {
            home_dir.join(".clippy")
        } else {
            // 最后的回退：使用当前目录
            PathBuf::from("./clippy_data")
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 创建存储引擎 - 使用用户配置目录而不是项目目录
    let storage_dir = get_app_data_dir();
    
    let storage_engine = match StorageEngine::new(storage_dir) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("创建存储引擎失败: {}", e);
            panic!("无法初始化存储");
        }
    };
    
    let clipboard_storage: ClipboardStorage = Arc::new(Mutex::new(storage_engine));

    // 创建同步引擎的状态容器
    let sync_engine: Arc<Mutex<Option<ClipboardSync>>> = Arc::new(Mutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(clipboard_storage.clone())
        .manage(sync_engine.clone())
        .setup(move |app| {
            // 创建系统托盘
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .title("Clippy - 剪贴板管理器")
                .tooltip("Clippy - 剪贴板管理器")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // 启动剪贴板监听器（带同步引擎初始化）
            start_clipboard_monitor(app.handle().clone(), clipboard_storage, sync_engine);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            clear_clipboard_history,
            delete_clipboard_item,
            get_storage_stats,
            compact_storage,
            copy_to_clipboard,
            copy_image_to_clipboard,
            copy_files_to_clipboard,
            setup_sync,
            sync_now,
            get_sync_status,
            configure_storage,
            get_storage_config,
            test_storage_connection,
            get_storage_backend_types
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 如果存在配置，创建同步引擎
async fn create_sync_engine_if_configured() -> Option<ClipboardSync> {
    // 尝试从配置文件加载同步配置
    let config_file = get_app_data_dir().join("storage_config.json");
    if let Ok(storage_config) = StorageConfig::load_from_file(config_file.to_string_lossy().as_ref()) {
        if let Ok(operator) = storage_config.create_operator().await {
            // 生成设备ID（应该持久化存储）
            let device_id = get_or_create_device_id();
            
            // 这里应该从用户配置获取user_id，暂时使用默认值
            let user_id = std::env::var("CLIPPY_USER_ID").unwrap_or_else(|_| "default_user".to_string());
            
            let sync_config = SyncConfig {
                user_id,
                device_id,
                storage_operator: operator,
                sync_interval_seconds: 15, // 15秒同步一次
            };
            
            let sync_engine = Arc::new(SyncEngine::new(sync_config));
            
            // 启动后台同步任务
            let sync_engine_clone = sync_engine.clone();
            tokio::spawn(async move {
                if let Err(e) = sync_engine_clone.start_background_sync().await {
                    tracing::error!("后台同步任务失败: {}", e);
                }
            });
            
            return Some(sync_engine);
        }
    }
    
    None
}

/// 获取或创建设备唯一ID
fn get_or_create_device_id() -> String {
    let device_id_file = get_app_data_dir().join("device_id");
    
    // 尝试从文件读取设备ID
    if let Ok(device_id) = std::fs::read_to_string(&device_id_file) {
        let device_id = device_id.trim();
        if !device_id.is_empty() {
            return device_id.to_string();
        }
    }
    
    // 生成新的设备ID
    let device_id = uuid::Uuid::new_v4().to_string();
    
    // 确保目录存在
    if let Some(parent) = device_id_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    // 保存到文件
    if let Err(e) = std::fs::write(&device_id_file, &device_id) {
        eprintln!("保存设备ID失败: {}", e);
    }
    
    device_id
}
