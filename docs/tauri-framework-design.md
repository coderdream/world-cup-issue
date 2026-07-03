# Tauri 框架开发设计文档

## 范围

`tauri-framework` 是从 WorldCupIssue 中抽取出来的可复用桌面应用框架。它面向 Windows 优先的 Tauri 2 + React 19 + TypeScript 桌面工具，包含：

- 首页、配置页、关于页。
- 深色绿黑主题、左侧导航、顶部栏、系统托盘和自定义窗口按钮。
- 基础控件优先使用组件库默认实现，例如 Ant Design 的 `Switch`。
- 可复用的 AI 模型配置和兼容 OpenAI 的聊天补全调用。
- 可复用的飞书群机器人 Webhook 配置、连通性测试和消息发送能力。
- 本地设置持久化、操作日志、SQLite 数据库和安装包构建能力。

## 存储布局

框架参考 CupWatch 的本地文件布局：

- 漫游数据目录保存需要长期保留的用户数据。
- 本地数据目录保存日志和 WebView 运行缓存。

框架在 Windows 上的路径：

- 设置文件：`%APPDATA%/com.tauriframework.app/settings.json`
- SQLite 数据库：`%APPDATA%/com.tauriframework.app/app.db`
- 文本日志：`%LOCALAPPDATA%/com.tauriframework.app/logs/info_YYYY_MM_DD.log`

CupWatch 参考路径：

- `%APPDATA%/com.cupwatch.app/settings.json`
- `%APPDATA%/com.cupwatch.app/app.db`
- `%LOCALAPPDATA%/com.cupwatch.app/logs/CupWatch.log`

## 设置持久化

框架启动时读取设置，调用 `set_settings` 时保存设置。

当前设置包含：

- `theme`
- `launchOnBoot`
- `notificationsEnabled`
- `apiBaseUrl`
- `apiKey`
- `aiProfile`
- `feishuProfile`

`aiProfile` 包含：

- `provider`
- `name`
- `baseURL`
- `model`
- `apiKey`

`feishuProfile` 包含：

- `webhookUrl`
- `title`
- `testMessage`

桌面安装版把设置保存到 `settings.json`。浏览器预览模式使用 `localStorage` 作为兜底。

## AI 能力

框架提供可复用的兼容 OpenAI 的 AI 能力：

- 在配置页维护 AI 名称、模型名、接口地址和 API Key。
- `测试连接` 会发送 `你好，请只回复 ok`。
- `复制当前配置分享` 会生成 `ai.profile` JSON。
- `生成 AI 评估` 会调用配置好的 `/chat/completions` 接口。

Rust 进程间命令：

- `test_ai_profile`
- `generate_ai_text`

配置的 `baseURL` 可以是接口根地址，也可以已经包含 `/chat/completions`。

## 飞书能力

框架提供可复用的飞书群机器人 Webhook 能力：

- 在配置页维护飞书 Webhook 地址、消息标题和测试消息。
- `测试飞书连通性` 会向配置好的 Webhook 发送一条真实文本消息。
- 测试成功时显示成功状态；失败时显示 HTTP、网络、响应解析或飞书返回码错误。
- 后续业务模块可以复用同一套发送命令给飞书推送通知。

Rust 进程间命令：

- `test_feishu_profile`
- `send_feishu_message`

飞书请求体采用群机器人文本消息格式：

```json
{
  "msg_type": "text",
  "content": {
    "text": "【Tauri Framework】\n飞书连通性测试成功。"
  }
}
```

成功判定规则：

- HTTP 请求成功。
- 飞书响应 JSON 可解析。
- 飞书响应 `code` 为 `0`。

## 操作日志

操作日志采用双写机制：

- 写入人类可读的每日文本日志。
- 写入结构化 SQLite 表 `operate_log`。

文本日志格式参考 CupWatch：

```text
[YYYY-MM-DD][HH:mm:ss][module][LEVEL] message
```

示例：

```text
[2026-06-18][11:45:00][ai][INFO] AI 配置测试成功
```

SQLite 表结构：

```sql
CREATE TABLE IF NOT EXISTS operate_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created_at TEXT NOT NULL,
  level TEXT NOT NULL,
  module TEXT NOT NULL,
  action TEXT NOT NULL,
  message TEXT NOT NULL,
  detail TEXT,
  trace_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_operate_log_created_at ON operate_log(created_at);
CREATE INDEX IF NOT EXISTS idx_operate_log_module_action ON operate_log(module, action);
CREATE INDEX IF NOT EXISTS idx_operate_log_trace_id ON operate_log(trace_id);
```

当前记录的事件包括：

- 应用启动。
- 读取应用状态。
- 读取配置。
- 保存配置。
- 检查更新。
- AI 配置测试成功或失败。
- AI 文本生成成功或失败。
- 飞书配置测试成功或失败。
- 飞书消息发送成功或失败。

### 结构化任务日志

框架日志器 `OperationLogger` 提供以下通用写入方法：

- `info(module, action, message)`：普通信息日志。
- `error(module, action, message, detail)`：普通错误日志。
- `debug(module, action, message, detail, trace_id)`：带任务 ID 的调试日志。
- `trace_info(module, action, message, detail, trace_id)`：带任务 ID 的信息日志。
- `warn(module, action, message, detail, trace_id)`：带任务 ID 的警告日志。
- `trace_error(module, action, message, detail, trace_id)`：带任务 ID 的错误日志。
- `log(level, module, action, message, detail, trace_id)`：底层通用入口。

`trace_id` 用于把一次后台任务串起来，例如素材生成、批量扫描、文件移动或 AI 分析。框架在初始化时会自动给旧数据库补 `trace_id` 列，并创建索引。

前端新增“操作日志”菜单，读取 `get_operation_logs` IPC：

- 默认显示最近 `1000` 条框架日志。
- 支持按 `traceId` 查询，派生应用可以传入当前任务 ID。
- 支持 `DEBUG / INFO / WARN / ERROR` 样式。
- 支持单行或多行选择、复制、右键菜单、软换行、滚动到底部和清空当前显示。
- 清空当前显示只影响前端状态，不删除 SQLite 或文本 log 文件。
- 支持搜索、大小写、全词、正则、上一处/下一处、只显示匹配项和搜索历史。

## 界面组件规则

基础控件应优先使用组件库默认实现。只有明确产品需求时，才允许手写控件几何尺寸。

开关组件使用 Ant Design 的 `Switch`，并通过 `ConfigProvider` 主题配置控制主色。不要再手工调整开关轨道、圆点尺寸和位移。

## 打包规则

框架代码或打包配置发生变更后，需要执行：

- 补丁版本递增 `0.0.1`。
- 执行 `pnpm build`。
- 执行 `cargo check --target x86_64-pc-windows-gnu --jobs 1`。
- 执行 `scripts/package-windows.ps1`。
- 在回复中给出生成的 NSIS 安装包路径。

当前安装包输出路径模式：

```text
tauri-framework/src-tauri/target/x86_64-pc-windows-gnu/release/bundle/nsis/Tauri Framework_<version>_x64-setup.exe
```

## 日报规则

本仓库每一轮助手对话都必须追加记录到：

```text
docs/daily/YYYY-MM-DD.md
```

每条日报至少记录：

- 用户要求。
- 执行动作。
- 验证命令和结果。
- 生成产物。
- 遗留问题或后续事项。
