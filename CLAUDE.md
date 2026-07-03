# CLAUDE.md — 项目工作规范

## 语言要求
- 所有分析、思考、解释、回复一律使用**中文**。

## 日报规范
- 每一轮对话的工作内容必须记录到 `docs/daily_cc/YYYY-MM-DD.md` 日报。
- 每完成一个步骤就**立即追加**日报，不要等整个任务结束后再集中补写（执行可能报错或 app 意外退出）。
- 每个事项标题前必须带时间，格式：`## HH:mm 标题`，精确到分钟，例如 `## 15:33 提交并推送代码`。
- 每条日报记录应包含：用户要求、执行动作、结果、验证方式、生成产物、遗留问题。

## 版本与构建规范
- 每次代码或打包相关变更后，必须将**补丁版本号递增 0.0.1**（`src-tauri/tauri.conf.json` 中的 `version`）。
- 每次打包测试前，代码必须先**推送到 GitHub**。
- 默认只构建 release exe（不含安装包）：
  ```
  pnpm -C a-book-in-30-minutes tauri build --ci --target x86_64-pc-windows-gnu --no-bundle
  ```
- 只有用户明确要求"安装包 / installer / NSIS / setup exe"时才构建安装包。
- 每生成一版新构建产物后，必须提交代码并推送 GitHub，保留该版本对应的代码状态。

## 构建前清理
- 每次打包前必须先清理历史构建产物，避免占满磁盘：
  - `a-book-in-30-minutes/src-tauri/target`
  - `a-book-in-30-minutes/dist`
  - `a-book-in-30-minutes/release`
  - 其他临时打包目录和安装包输出

## 实现规范
- 实现方式要尽量**框架化**：能通过配置、领域模块、共享服务或通用组件解决的，不要把页面逻辑写死。
- 不要引入超出任务范围的重构、抽象或额外功能。

## 项目路径
- 主项目：`D:\04_GitHub\world-cup-issue\a-book-in-30-minutes`
- 文档目录：`D:\04_GitHub\world-cup-issue\docs\daily_cc\`
