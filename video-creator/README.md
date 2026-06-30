# 视频工坊

`video-creator` 是基于 Tauri 2、React 19、TypeScript 和 Vite 的桌面工作台。

第一版复刻 `D:\04_GitHub\video-easy-creator` 的核心桌面体验，前端负责执行中心、步骤跟踪、执行日志、历史记录、Quark 同步和 Skills 管理，业务能力通过旧 Java 项目命令入口复用。

## 开发

```bash
pnpm install
pnpm tauri:dev
```

## 检查

```bash
pnpm build
cargo check --manifest-path src-tauri\Cargo.toml --target x86_64-pc-windows-gnu --jobs 1
```

## Windows 打包

打包前先提交并推送代码，然后运行：

```powershell
.\scripts\package-windows.ps1
```
