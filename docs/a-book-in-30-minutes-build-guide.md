# A Book in 30 Minutes 打包指南

本文档用于让其他协作者在 Windows 机器上正确构建 `a-book-in-30-minutes`。仓库默认目标是更新桌面快捷方式使用的 Release exe；只有明确要求“安装包 / installer / NSIS / setup exe”时才生成安装包。

## 1. 打包前原则

1. 每次收到打包或构建相关要求，先更新 `docs/daily/YYYY-MM-DD.md` 日报。
2. 每次代码、流程、界面、数据结构或打包相关变更后，先把 `a-book-in-30-minutes` 版本号递增 `0.0.1`。
3. 版本号至少同步修改：
   - `a-book-in-30-minutes/package.json`
   - `a-book-in-30-minutes/src-tauri/Cargo.toml`
   - `a-book-in-30-minutes/src-tauri/tauri.conf.json`
   - 构建后如果 `a-book-in-30-minutes/src-tauri/Cargo.lock` 当前包版本变化，也要一起提交。
4. `a-book-in-30-minutes` 的功能、流程、界面、数据结构或打包相关更新后，同步更新 `docs/a-book-in-30-minutes-design.md`。
5. 每次打包测试前，必须先提交并推送代码到 GitHub，保证构建产物对应的代码状态可追溯。
6. 每次生成新版本或新构建产物后，必须再次提交并推送构建后产生的必要版本/锁文件/日报变更。

## 2. 固定 GNU 构建环境

本仓库在 Windows 上构建 Tauri/Rust 必须使用已验证的 GNU 工具链，不要默认走 MSVC。出现 `link.exe not found` 时，通常说明环境没有套对。

每次执行 `cargo check` 或 Tauri 打包前，在当前 PowerShell 会话设置：

```powershell
$env:RUSTUP_HOME=(Resolve-Path '.tooling\rustup').Path
$env:CARGO_HOME=(Resolve-Path '.tooling\cargo').Path
$env:RUSTUP_TOOLCHAIN='stable-x86_64-pc-windows-gnu'

$mingw='C:\Users\Administrator\scoop\apps\mingw\current\bin'
$rustlib=(Resolve-Path '.tooling\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin').Path
$env:PATH="$mingw;$rustlib;$env:PATH"
```

注意：Scoop MinGW 的 `bin` 必须排在 Rust self-contained linker 前面，否则可能出现 `gcc` 找不到 `crt2.o`、`libkernel32.a` 等 MinGW 运行库的问题。

## 3. 打包前清理

打包前先关闭正在运行的旧版应用，否则旧 exe 或 DLL 可能被占用，导致清理失败。

检查并结束旧进程：

```powershell
Get-Process | Where-Object {
  $_.ProcessName -like '*a_book*' -or $_.Path -like '*a-book-in-30-minutes*'
} | Select-Object Id,ProcessName,Path

# 如确认是旧版 A Book in 30 Minutes，可结束：
Stop-Process -Id <PID> -Force
```

清理可重新生成的构建产物：

```powershell
if (Test-Path 'a-book-in-30-minutes\dist') {
  Remove-Item -LiteralPath 'a-book-in-30-minutes\dist' -Recurse -Force
}
if (Test-Path 'a-book-in-30-minutes\src-tauri\target') {
  Remove-Item -LiteralPath 'a-book-in-30-minutes\src-tauri\target' -Recurse -Force
}
if (Test-Path 'a-book-in-30-minutes\target') {
  Remove-Item -LiteralPath 'a-book-in-30-minutes\target' -Recurse -Force
}
```

验证清理结果：

```powershell
@{
  DistExists = (Test-Path 'a-book-in-30-minutes\dist')
  SrcTauriTargetExists = (Test-Path 'a-book-in-30-minutes\src-tauri\target')
  TargetExists = (Test-Path 'a-book-in-30-minutes\target')
}
```

三个值都应为 `False`。

## 4. 提交并推送源码

打包前必须提交并推送当前代码：

```powershell
git status --short
git add <本次相关文件>
git commit -m "chore: prepare a-book-in-30-minutes release"
git push origin main
git rev-parse HEAD
git ls-remote origin refs/heads/main
```

`git rev-parse HEAD` 和 `git ls-remote origin refs/heads/main` 应指向同一个提交。

如果 `git push` 超时但不确定是否成功，先用 `git ls-remote origin refs/heads/main` 确认远端提交，不要盲目重复处理代码。

## 5. 构建前检查

固定检查命令：

```powershell
cargo check --manifest-path a-book-in-30-minutes\src-tauri\Cargo.toml --target x86_64-pc-windows-gnu --jobs 1
```

允许存在既有 `unused` 警告；如果出现 MSVC `link.exe not found`，回到第 2 节重新设置 GNU 环境后再跑。

## 6. 默认流程：只生成 Release exe

这是默认打包方式，用于更新桌面快捷方式《A Book in 30 Minutes 开发版》访问的 release exe。不要生成安装包。

```powershell
pnpm -C a-book-in-30-minutes tauri build --ci --target x86_64-pc-windows-gnu --no-bundle
```

成功后产物在：

```text
D:\04_GitHub\world-cup-issue\a-book-in-30-minutes\src-tauri\target\x86_64-pc-windows-gnu\release\a_book_in_30_minutes.exe
```

不要用裸 `cargo build --release` 替代这个命令；裸 Cargo 构建不会正确嵌入 Tauri 前端资源，应用可能尝试访问 `http://127.0.0.1:1421` 并显示连接被拒绝。

## 7. 明确要求安装包时的流程

只有用户明确说要“安装包 / installer / NSIS / setup exe”时，才运行不带 `--no-bundle` 的命令：

```powershell
pnpm -C a-book-in-30-minutes tauri build --ci --target x86_64-pc-windows-gnu
```

成功后通常会同时有：

```text
a-book-in-30-minutes\src-tauri\target\x86_64-pc-windows-gnu\release\a_book_in_30_minutes.exe
a-book-in-30-minutes\src-tauri\target\x86_64-pc-windows-gnu\release\bundle\nsis\*setup*.exe
```

生成安装包时，还要同步检查更新元数据、发布目录、安装包清单和用户要求的发布流程是否一致。

## 8. 构建后验证

验证 exe 存在、大小和时间：

```powershell
Get-Item 'a-book-in-30-minutes\src-tauri\target\x86_64-pc-windows-gnu\release\a_book_in_30_minutes.exe' |
  Format-List FullName,Length,LastWriteTime
```

验证版本号：

```powershell
(Get-Item 'a-book-in-30-minutes\src-tauri\target\x86_64-pc-windows-gnu\release\a_book_in_30_minutes.exe').VersionInfo |
  Format-List FileVersion,ProductVersion,ProductName
```

只构建 Release exe 时，确认没有生成安装包：

```powershell
Get-ChildItem 'a-book-in-30-minutes\src-tauri\target\x86_64-pc-windows-gnu\release' -Recurse -Filter '*setup*.exe' -ErrorAction SilentlyContinue |
  Select-Object FullName,Length
```

该命令应无输出。

安装包流程则反过来，应确认 `bundle\nsis\*setup*.exe` 存在。

## 9. 构建后提交并推送

构建后再次检查工作区：

```powershell
git status --short
```

常见需要提交的文件：

- `a-book-in-30-minutes/src-tauri/Cargo.lock`，如果当前包版本被更新。
- `docs/daily/YYYY-MM-DD.md`，记录构建命令、结果、验证方式、产物和遗留问题。
- 其它本次明确修改的版本、配置、设计文档。

提交并推送：

```powershell
git add <构建后需要提交的文件>
git commit -m "chore: record a-book-in-30-minutes release build"
git push origin main
```

最后确认远端：

```powershell
git rev-parse HEAD
git ls-remote origin refs/heads/main
git status --short
```

正常情况下，本地 HEAD 与远端 `main` 相同；除用户明确保留的其它工作外，工作区应干净。

## 10. 常见问题

### 10.1 `link.exe not found`

原因：误走 MSVC 工具链。

处理：重新执行第 2 节 GNU 环境设置，确认命令包含 `--target x86_64-pc-windows-gnu`。

### 10.2 找不到 `crt2.o` 或 `libkernel32.a`

原因：PATH 顺序不对，Rust self-contained linker 排在 MinGW 前面。

处理：确保 `C:\Users\Administrator\scoop\apps\mingw\current\bin` 在 PATH 最前面。

### 10.3 清理 `src-tauri\target` 失败，提示 exe 或 DLL 被占用

原因：旧版应用仍在运行。

处理：按第 3 节定位并结束 `a_book_in_30_minutes` 进程后重试清理。

### 10.4 只想更新桌面快捷方式却生成了安装包

原因：误用了不带 `--no-bundle` 的命令。

处理：默认始终使用：

```powershell
pnpm -C a-book-in-30-minutes tauri build --ci --target x86_64-pc-windows-gnu --no-bundle
```

### 10.5 Release exe 打开后访问 `127.0.0.1:1421`

原因：可能用了裸 `cargo build --release`，前端资源没有嵌入。

处理：重新清理后使用 Tauri 命令构建，见第 6 节。

## 11. 本次已验证示例

2026-07-03 验证过的 Release exe 流程：

```powershell
cargo check --manifest-path a-book-in-30-minutes\src-tauri\Cargo.toml --target x86_64-pc-windows-gnu --jobs 1
pnpm -C a-book-in-30-minutes tauri build --ci --target x86_64-pc-windows-gnu --no-bundle
```

结果：

- `cargo check` 通过，仅有既有 unused 警告。
- Release exe 构建成功。
- 产物版本为 `0.1.126`。
- 未生成 `setup.exe` / installer。

