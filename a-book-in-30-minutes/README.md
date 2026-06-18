# Tauri Framework

一个从 WorldCupIssue 抽出的 Windows 优先 Tauri 2 桌面应用框架。

包含：

- React 19 + TypeScript + Vite
- Tauri 2 桌面壳
- 左侧导航、顶部标题栏、窗口最小化/最大化/关闭、顶部栏拖拽
- 首页、配置页、关于页
- Zustand 状态管理
- 简体中文 NSIS 安装器配置
- 基础 IPC：`get_app_state`、`get_settings`、`set_settings`、`check_update_mock`

运行：

```bash
pnpm install
pnpm tauri:dev
```

打包：

```bash
pnpm tauri:build --target x86_64-pc-windows-gnu
```
