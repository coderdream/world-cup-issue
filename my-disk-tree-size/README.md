# MyDiskTreeSize

MyDiskTreeSize 是一个基于 Tauri 2、React 和 Rust 的 Windows 磁盘空间树分析工具。

## 功能

- 扫描本地目录、盘符、映射盘和 UNC 网络路径。
- 按目录树展示大小、已分配空间、文件数、文件夹数和占上层百分比。
- 支持自动、MB、GB、TB 单位切换。
- 支持隐藏项开关和最大扫描深度设置。
- 提供浏览器预览 fallback，方便在没有 Tauri API 的环境下查看界面。

## 开发

```powershell
pnpm install
pnpm build
pnpm tauri build --target x86_64-pc-windows-gnu
```
