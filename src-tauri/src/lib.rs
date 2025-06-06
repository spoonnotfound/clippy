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

// 全局状态
type ClipboardStorage = Arc<Mutex<StorageEngine>>;

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
    last_text: String,
    last_files: Vec<String>,
}

impl ClipboardManager {
    pub fn new(app_handle: AppHandle, storage: ClipboardStorage) -> Result<Self, String> {
        let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;
        Ok(Self {
            ctx,
            app_handle,
            storage,
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
fn start_clipboard_monitor(app_handle: AppHandle, storage: ClipboardStorage) {
    thread::spawn(move || {
        let manager = match ClipboardManager::new(app_handle, storage) {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 创建存储引擎
    let storage_dir = PathBuf::from("./clippy_data");
    
    let storage_engine = match StorageEngine::new(storage_dir) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("创建存储引擎失败: {}", e);
            panic!("无法初始化存储");
        }
    };
    
    let clipboard_storage: ClipboardStorage = Arc::new(Mutex::new(storage_engine));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(clipboard_storage.clone())
        .setup(|app| {
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

            // 启动剪贴板监听器
            start_clipboard_monitor(app.handle().clone(), clipboard_storage);

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
            copy_files_to_clipboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
