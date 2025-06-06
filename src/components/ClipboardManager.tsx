import React from "react";
import { Copy, Trash2, Clock, File, Files, FileImage, FileText, FileCode, Archive, Music, Video, Database, HardDrive } from "lucide-react";
import { Button } from "./ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { ClipboardItem, FileTypeInfo, StorageStats } from "../App";

interface ClipboardManagerProps {
  items: ClipboardItem[];
  storageStats: StorageStats | null;
  onClearHistory: () => void;
  onDeleteItem: (itemId: string) => void;
  onCompactStorage: () => void;
  onCopyToClipboard: (content: string) => void;
  onCopyImageToClipboard: (base64Data: string) => void;
  onCopyFilesToClipboard: (filePaths: string[]) => void;
}

export const ClipboardManager: React.FC<ClipboardManagerProps> = ({
  items,
  storageStats,
  onClearHistory,
  onDeleteItem,
  onCompactStorage,
  onCopyToClipboard,
  onCopyImageToClipboard,
  onCopyFilesToClipboard,
}) => {
  const formatTimestamp = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diffInHours = (now.getTime() - date.getTime()) / (1000 * 60 * 60);
    
    if (diffInHours < 1) {
      const diffInMinutes = Math.floor(diffInHours * 60);
      return diffInMinutes === 0 ? "刚刚" : `${diffInMinutes} 分钟前`;
    } else if (diffInHours < 24) {
      return `${Math.floor(diffInHours)} 小时前`;
    } else {
      return date.toLocaleDateString("zh-CN");
    }
  };

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const handleDeleteItem = (itemId: string, event: React.MouseEvent) => {
    event.stopPropagation(); // 防止触发复制操作
    onDeleteItem(itemId);
  };

  const truncateText = (text: string, maxLength: number = 100) => {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "...";
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "image":
        return <FileImage className="h-4 w-4" />;
      case "video":
        return <Video className="h-4 w-4" />;
      case "audio":
        return <Music className="h-4 w-4" />;
      case "document":
        return <FileText className="h-4 w-4" />;
      case "archive":
        return <Archive className="h-4 w-4" />;
      case "code":
        return <FileCode className="h-4 w-4" />;
      default:
        return <File className="h-4 w-4" />;
    }
  };

  const getItemIcon = (itemType: string) => {
    switch (itemType) {
      case "files":
        return <Files className="h-4 w-4" />;
      default:
        return <Copy className="h-4 w-4" />;
    }
  };

  const getItemTypeLabel = (itemType: string) => {
    switch (itemType) {
      case "files":
        return "文件";
      default:
        return "文本";
    }
  };

  const handleItemClick = (item: ClipboardItem) => {
    if (item.item_type === "files" && item.file_paths) {
      onCopyFilesToClipboard(item.file_paths);
    } else {
      onCopyToClipboard(item.content);
    }
  };

  const renderFileTypeInfo = (fileTypes: FileTypeInfo[]) => {
    // 按类别分组
    const categorizedFiles = fileTypes.reduce((acc, file) => {
      const category = file.category;
      if (!acc[category]) acc[category] = [];
      acc[category].push(file);
      return acc;
    }, {} as Record<string, FileTypeInfo[]>);

    return (
      <div className="space-y-2">
        {Object.entries(categorizedFiles).map(([category, files]) => (
          <div key={category} className="flex items-center gap-2">
            {getCategoryIcon(category)}
            <span className="text-xs text-muted-foreground capitalize">
              {category} ({files.length})
            </span>
          </div>
        ))}
      </div>
    );
  };

  const renderItemContent = (item: ClipboardItem) => {
    switch (item.item_type) {
      case "files":
        return (
          <div className="space-y-3">
            <div className="text-sm font-medium">
              {item.file_paths?.length} 个文件
              {item.size && ` • ${formatFileSize(item.size)}`}
            </div>
            
            {/* 文件类型分类显示 */}
            {item.file_types && item.file_types.length > 0 && (
              <div className="border-l-4 border-primary/20 pl-3">
                {renderFileTypeInfo(item.file_types)}
              </div>
            )}
            
            {/* 文件列表 */}
            <div className="space-y-1">
              {item.file_paths?.slice(0, 5).map((path, index) => {
                const fileType = item.file_types?.find(ft => ft.path === path);
                const fileName = path.split('/').pop() || path;
                
                return (
                  <div key={index} className="text-xs text-muted-foreground flex items-center gap-2">
                    {fileType ? getCategoryIcon(fileType.category) : <File className="h-3 w-3" />}
                    <span className="truncate" title={path}>{fileName}</span>
                    {fileType && (
                      <span className="text-xs bg-muted px-1 rounded">
                        {fileType.file_type || fileType.category}
                      </span>
                    )}
                  </div>
                );
              })}
              {item.file_paths && item.file_paths.length > 5 && (
                <div className="text-xs text-muted-foreground">
                  还有 {item.file_paths.length - 5} 个文件...
                </div>
              )}
            </div>
          </div>
        );
      
      default:
        return (
          <div className="text-sm">
            <pre className="whitespace-pre-wrap font-mono text-xs bg-muted p-3 rounded-md overflow-hidden">
              {truncateText(item.content)}
            </pre>
            {item.content.length > 100 && (
              <p className="text-xs text-muted-foreground mt-2">
                点击查看完整内容并复制
              </p>
            )}
          </div>
        );
    }
  };

  return (
    <div className="container mx-auto p-6 max-w-4xl">
      <div className="mb-6">
        <div className="flex items-center justify-between">
          <h1 className="text-3xl font-bold text-foreground">
            剪贴板管理器
          </h1>
          <div className="flex items-center gap-2">
            <Button
              onClick={onCompactStorage}
              variant="outline"
              size="sm"
              className="flex items-center gap-2"
            >
              <Database className="h-4 w-4" />
              压缩存储
            </Button>
            <Button
              onClick={onClearHistory}
              variant="outline"
              size="sm"
              className="flex items-center gap-2"
            >
              <Trash2 className="h-4 w-4" />
              清空历史
            </Button>
          </div>
        </div>
        <p className="text-muted-foreground mt-2">
          管理和重复使用你的剪贴板历史记录
        </p>
        
        {/* 存储统计信息 */}
        {storageStats && (
          <div className="mt-4 grid grid-cols-3 gap-4">
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Files className="h-4 w-4" />
              <span>总计: {storageStats.total_items} 个项目</span>
            </div>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Trash2 className="h-4 w-4" />
              <span>已删除: {storageStats.deleted_items} 个项目</span>
            </div>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <HardDrive className="h-4 w-4" />
              <span>存储大小: {formatFileSize(storageStats.file_size)}</span>
            </div>
          </div>
        )}
      </div>

      {items.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16">
            <Copy className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold text-muted-foreground mb-2">
              还没有剪贴板历史
            </h3>
            <p className="text-muted-foreground text-center max-w-sm">
              复制一些文本或文件，它们会自动出现在这里
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {items.map((item) => (
            <Card 
              key={item.id} 
              className="cursor-pointer hover:bg-accent/50 transition-colors"
              onClick={() => handleItemClick(item)}
            >
              <CardContent className="p-4">
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-center gap-2">
                    {getItemIcon(item.item_type)}
                    <span className="text-sm font-medium text-muted-foreground">
                      {getItemTypeLabel(item.item_type)}
                    </span>
                  </div>
                  <div className="flex items-center gap-3">
                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                      <Clock className="h-3 w-3" />
                      {formatTimestamp(item.timestamp)}
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 hover:bg-destructive hover:text-destructive-foreground"
                      onClick={(e) => handleDeleteItem(item.id, e)}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </div>

                {renderItemContent(item)}
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}; 