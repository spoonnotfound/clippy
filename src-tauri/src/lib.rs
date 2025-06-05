use arboard::Clipboard;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};

// 剪贴板历史数据结构
#[derive(Debug, Clone, serde::Serialize)]
struct ClipboardItem {
    id: String,
    content: String,
    timestamp: u64,
    item_type: String,
}

// 全局状态
type ClipboardHistory = Arc<Mutex<Vec<ClipboardItem>>>;

#[tauri::command]
fn get_clipboard_history(state: tauri::State<ClipboardHistory>) -> Vec<ClipboardItem> {
    state.lock().unwrap().clone()
}

#[tauri::command]
fn clear_clipboard_history(state: tauri::State<ClipboardHistory>) {
    state.lock().unwrap().clear();
}

#[tauri::command]
fn copy_to_clipboard(content: String) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(content).map_err(|e| e.to_string())?;
    Ok(())
}

// 启动剪贴板监听器
fn start_clipboard_monitor(app_handle: AppHandle, history: ClipboardHistory) {
    thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                eprintln!("Failed to create clipboard: {}", e);
                return;
            }
        };

        let mut last_content = String::new();

        loop {
            thread::sleep(Duration::from_millis(500));

            if let Ok(content) = clipboard.get_text() {
                if content != last_content && !content.trim().is_empty() {
                    let item = ClipboardItem {
                        id: format!("{}", std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()),
                        content: content.clone(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        item_type: "text".to_string(),
                    };

                    // 添加到历史记录
                    {
                        let mut history_lock = history.lock().unwrap();
                        history_lock.insert(0, item.clone());
                        
                        // 限制历史记录数量
                        if history_lock.len() > 100 {
                            history_lock.truncate(100);
                        }
                    }

                    // 发送事件到前端
                    let _ = app_handle.emit("clipboard-update", &item);
                    
                    last_content = content;
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 创建剪贴板历史状态
    let clipboard_history: ClipboardHistory = Arc::new(Mutex::new(Vec::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(clipboard_history.clone())
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
            start_clipboard_monitor(app.handle().clone(), clipboard_history);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            clear_clipboard_history,
            copy_to_clipboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
