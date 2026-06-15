# 3Q语言助手

一款 Windows 优先的免费翻译与外语学习桌面软件。首版使用 Tauri 2、React、TypeScript 和 SQLite 实现，包含查词、长文本翻译、单词本、每日学习、快捷键入口和截图翻译接口。

## 本机依赖

- Node.js 已安装时可直接使用 `npm.cmd`，避免 PowerShell 执行策略拦截 `npm.ps1`。
- 需要安装 Rustup、Microsoft C++ Build Tools 和 WebView2 Runtime 才能运行 Tauri 桌面端。
- 可选安装 pnpm；当前项目脚本也支持 `npm.cmd`。

## 开发命令

```powershell
npm.cmd install
npm.cmd run dev
npm.cmd run tauri:dev
```

## 说明

- 浏览器开发模式会使用 localStorage 和公开 API fallback。
- Tauri 桌面模式会调用 Rust 命令，并使用 SQLite 保存单词本和设置。
- 截图翻译入口已接入命令，OCR 需要在 Windows + Rust 环境安装完成后继续验证系统 OCR 绑定。
