# Clippy - 剪贴板管理器

一个基于 Tauri + React 的跨平台剪贴板管理器，具有系统托盘功能和实时剪贴板监听。

## ✨ 功能特点

- 🎯 **实时监听剪贴板** - 自动捕获复制的内容
- 📱 **系统托盘集成** - 后台运行，点击托盘图标显示窗口
- 🎨 **现代化 UI** - 使用 Tailwind CSS + shadcn/ui 组件
- 💾 **历史记录管理** - 保存最近 100 个剪贴板项目
- ⚡ **快速复制** - 点击任意历史项目即可复制
- 🌙 **深色/浅色主题** - 支持系统主题切换
- 🔄 **跨平台支持** - Windows、macOS、Linux

## 🛠️ 技术栈

### 前端
- **React 18** - 用户界面框架
- **TypeScript** - 类型安全的 JavaScript
- **Tailwind CSS** - 实用程序优先的 CSS 框架
- **shadcn/ui** - 可复用的 React 组件
- **Lucide React** - 美观的图标库

### 后端
- **Tauri** - 构建桌面应用的 Rust 框架
- **arboard** - 跨平台剪贴板库
- **tokio** - 异步运行时

## 🚀 开发环境设置

### 前置要求

1. **Node.js** (>= 16.0.0)
2. **Rust** (最新稳定版)
3. **系统依赖**:
   - **Windows**: Microsoft C++ Build Tools
   - **macOS**: Xcode Command Line Tools
   - **Linux**: `build-essential`, `libwebkit2gtk-4.0-dev`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### 安装与运行

```bash
# 克隆项目
git clone <your-repo-url>
cd clippy

# 安装依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## 📱 使用说明

1. **启动应用** - 运行后应用会显示主窗口并在系统托盘创建图标
2. **复制内容** - 在任何地方复制文本，内容会自动添加到剪贴板历史
3. **查看历史** - 在主窗口查看所有复制的内容
4. **重新复制** - 点击任意历史项目即可将其复制到剪贴板
5. **清空历史** - 点击"清空历史"按钮清除所有记录
6. **系统托盘** - 关闭窗口后应用继续在后台运行，点击托盘图标重新显示

## 🔧 配置

项目配置文件位于 `src-tauri/tauri.conf.json`，可以调整：

- 窗口大小和位置
- 系统托盘设置
- 应用图标和标题
- 安全策略

## 📂 项目结构

```
clippy/
├── src/                    # React 前端代码
│   ├── components/         # React 组件
│   │   ├── ui/            # UI 基础组件
│   │   └── ClipboardManager.tsx
│   ├── lib/               # 工具函数
│   ├── App.tsx            # 主应用组件
│   └── main.tsx           # 应用入口
├── src-tauri/             # Tauri 后端代码
│   ├── src/
│   │   ├── lib.rs         # 主要应用逻辑
│   │   └── main.rs        # 应用入口
│   ├── Cargo.toml         # Rust 依赖配置
│   └── tauri.conf.json    # Tauri 配置
└── package.json           # Node.js 依赖配置
```

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

[MIT License](LICENSE)
