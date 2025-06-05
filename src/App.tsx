import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ClipboardManager } from "./components/ClipboardManager";
import "./App.css";

export interface ClipboardItem {
  id: string;
  content: string;
  timestamp: number;
  item_type: string;
}

function App() {
  const [clipboardHistory, setClipboardHistory] = useState<ClipboardItem[]>([]);

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

    fetchHistory();

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
    } catch (error) {
      console.error("Failed to clear clipboard history:", error);
    }
  };

  const handleCopyToClipboard = async (content: string) => {
    try {
      await invoke("copy_to_clipboard", { content });
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  };

  return (
    <div className="min-h-screen bg-background">
      <ClipboardManager
        items={clipboardHistory}
        onClearHistory={handleClearHistory}
        onCopyToClipboard={handleCopyToClipboard}
      />
    </div>
  );
}

export default App;
