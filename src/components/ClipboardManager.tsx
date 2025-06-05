import React from "react";
import { Copy, Trash2, Clock } from "lucide-react";
import { Button } from "./ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { ClipboardItem } from "../App";

interface ClipboardManagerProps {
  items: ClipboardItem[];
  onClearHistory: () => void;
  onCopyToClipboard: (content: string) => void;
}

export const ClipboardManager: React.FC<ClipboardManagerProps> = ({
  items,
  onClearHistory,
  onCopyToClipboard,
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

  const truncateText = (text: string, maxLength: number = 100) => {
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + "...";
  };

  return (
    <div className="container mx-auto p-6 max-w-4xl">
      <div className="mb-6">
        <div className="flex items-center justify-between">
          <h1 className="text-3xl font-bold text-foreground">
            剪贴板管理器
          </h1>
          <div className="flex gap-2">
            <Button
              variant="outline"
              onClick={onClearHistory}
              className="flex items-center gap-2"
              disabled={items.length === 0}
            >
              <Trash2 className="h-4 w-4" />
              清空历史
            </Button>
          </div>
        </div>
        <p className="text-muted-foreground mt-2">
          共有 {items.length} 个剪贴板项目
        </p>
      </div>

      {items.length === 0 ? (
        <Card className="text-center py-12">
          <CardContent>
            <div className="text-muted-foreground">
              <Copy className="h-12 w-12 mx-auto mb-4" />
              <p className="text-lg">暂无剪贴板历史</p>
              <p className="text-sm mt-2">
                复制任何内容到剪贴板，它们会自动出现在这里
              </p>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {items.map((item) => (
            <Card
              key={item.id}
              className="hover:shadow-md transition-shadow cursor-pointer"
              onClick={() => onCopyToClipboard(item.content)}
            >
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                    <Clock className="h-3 w-3" />
                    {formatTimestamp(item.timestamp)}
                  </CardTitle>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      onCopyToClipboard(item.content);
                    }}
                    className="h-8 w-8 p-0"
                  >
                    <Copy className="h-4 w-4" />
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="pt-0">
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
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}; 