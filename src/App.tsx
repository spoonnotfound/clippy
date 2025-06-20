import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ClipboardManager } from "./components/ClipboardManager";
import { StorageConfig } from "./components/StorageConfig";
import { ConfigGuide } from "./components/ConfigGuide";
import "./App.css";

export interface FileTypeInfo {
  path: string;
  file_type: string;
  mime_type: string;
  category: string;
}

export interface ClipboardItem {
  id: string;
  content: string;
  timestamp: number;
  item_type: string; // "text" 或 "files"
  size?: number;
  file_paths?: string[];
  file_types?: FileTypeInfo[];
}

export interface StorageStats {
  total_items: number;
  deleted_items: number;
  file_size: number;
}

function App() {
  const [currentView, setCurrentView] = useState<'clipboard' | 'config' | 'guide'>('clipboard');
  const [clipboardHistory, setClipboardHistory] = useState<ClipboardItem[]>([]);
  const [storageStats, setStorageStats] = useState<StorageStats | null>(null);

  useEffect(() => {
    // 获取初始剪贴板历史
    const fetchHistory = async () => {
      try {
        const history = await invoke<ClipboardItem[]>("get_clipboard_history");
        setClipboardHistory(history);
      } catch (error) {
        console.error("Failed to fetch clipboard history:", error);
      }
    };

    // 获取存储统计信息
    const fetchStats = async () => {
      try {
        const stats = await invoke<StorageStats>("get_storage_stats");
        setStorageStats(stats);
      } catch (error) {
        console.error("Failed to fetch storage stats:", error);
      }
    };

    fetchHistory();
    fetchStats();

    // 监听剪贴板更新事件
    const unlisten = listen<ClipboardItem>("clipboard-update", (event) => {
      setClipboardHistory((prev) => [event.payload, ...prev.slice(0, 99)]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleClearHistory = async () => {
    try {
      await invoke("clear_clipboard_history");
      setClipboardHistory([]);
      // 重新获取统计信息
      const stats = await invoke<StorageStats>("get_storage_stats");
      setStorageStats(stats);
    } catch (error) {
      console.error("Failed to clear clipboard history:", error);
    }
  };

  const handleDeleteItem = async (itemId: string) => {
    try {
      await invoke("delete_clipboard_item", { itemId });
      // 从本地状态中移除项目
      setClipboardHistory(prev => prev.filter(item => item.id !== itemId));
      // 重新获取统计信息
      const stats = await invoke<StorageStats>("get_storage_stats");
      setStorageStats(stats);
    } catch (error) {
      console.error("Failed to delete clipboard item:", error);
    }
  };

  const handleCompactStorage = async () => {
    try {
      await invoke("compact_storage");
      // 重新获取统计信息
      const stats = await invoke<StorageStats>("get_storage_stats");
      setStorageStats(stats);
    } catch (error) {
      console.error("Failed to compact storage:", error);
    }
  };

  const handleCopyToClipboard = async (content: string) => {
    try {
      await invoke("copy_to_clipboard", { content });
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  };

  const handleCopyImageToClipboard = async (base64Data: string) => {
    try {
      await invoke("copy_image_to_clipboard", { base64Data });
    } catch (error) {
      console.error("Failed to copy image to clipboard:", error);
    }
  };

  const handleCopyFilesToClipboard = async (filePaths: string[]) => {
    try {
      await invoke("copy_files_to_clipboard", { filePaths });
    } catch (error) {
      console.error("Failed to copy files to clipboard:", error);
    }
  };



  return (
    <div className="min-h-screen bg-background">
      <nav className="bg-white border-b border-gray-200 px-4 py-2">
        <div className="flex space-x-4">
          <button
            onClick={() => setCurrentView('clipboard')}
            className={`px-3 py-2 rounded-md text-sm font-medium ${
              currentView === 'clipboard'
                ? 'bg-blue-100 text-blue-700'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            剪切板历史
          </button>
          <button
            onClick={() => setCurrentView('config')}
            className={`px-3 py-2 rounded-md text-sm font-medium ${
              currentView === 'config'
                ? 'bg-blue-100 text-blue-700'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            同步配置
          </button>
          <button
            onClick={() => setCurrentView('guide')}
            className={`px-3 py-2 rounded-md text-sm font-medium ${
              currentView === 'guide'
                ? 'bg-blue-100 text-blue-700'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            配置指南
          </button>
        </div>
      </nav>

      <div className="p-4">
        {currentView === 'clipboard' && (
          <ClipboardManager
            items={clipboardHistory}
            storageStats={storageStats}
            onClearHistory={handleClearHistory}
            onDeleteItem={handleDeleteItem}
            onCompactStorage={handleCompactStorage}
            onCopyToClipboard={handleCopyToClipboard}
            onCopyImageToClipboard={handleCopyImageToClipboard}
            onCopyFilesToClipboard={handleCopyFilesToClipboard}
          />
        )}
        
        {currentView === 'config' && (
          <StorageConfig />
        )}
        
        {currentView === 'guide' && (
          <ConfigGuide />
        )}
      </div>
    </div>
  );
}

export default App;
