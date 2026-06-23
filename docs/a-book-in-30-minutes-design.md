# A Book in 30 Minutes 详细设计文档

## 文档维护规则

- 本文档是 `a-book-in-30-minutes` 的长期设计基线。每次修改功能、流程、界面、数据结构、提示词、日志、通知或打包配置时，都必须同步更新本文档。
- 日报仍写入 `docs/daily/YYYY-MM-DD.md`，本文档负责沉淀稳定设计，避免只靠聊天记录传递背景。
- 所有新增中文文案、日志、配置和文档必须保持 UTF-8 正常显示，不得出现乱码。

## 产品定位

`A Book in 30 Minutes` 是一个 Tauri 桌面工具，用于把小说或书籍源文件转换成 YouTube 听书视频素材。核心工作台命名为“流水线”，按素材、音频、视频三个阶段逐步处理。当前阶段已覆盖“文本素材生成”和“旁白音频生成”，视频阶段的设计基线已明确为“高清原图资产 + 字幕时间轴 + 剪映草稿/ffmpeg 渲染”，后续实现必须沿该链路落地。

- 中文频道名：`半小时听完一本书`
- 英文产品名：`A Book in 30 Minutes`
- 应用目录：`a-book-in-30-minutes`
- Tauri 标识：`com.abookin30minutes.desktop`
- Rust crate：`a_book_in_30_minutes`
- 当前版本：`0.1.68`

核心输出包括视频标题、简介、标签、旁白稿、字幕文本、SRT/ASS 字幕、生成提示词、源书概览、结构化素材 JSON、微软语音 SSML、旁白 mp3、AI 原始高清图片、图片资产清单和图片-字幕时间轴。

## 用户流程

1. 用户先在“配置”页维护素材生成默认配置，包括分类/播放列表、目标语言、最少字数、最多字数和生成方向。分类对应后续 YouTube 播放列表名称，默认提供 `半小时听完一本书`、`睡前听完一本书`、`A Book in 30 Minutes` 三个分类，并支持新增自定义分类。
2. 用户在“流水线”页输入文件路径，或使用“选择文件”“通过素材文件定位文件夹”按钮。两个按钮都会打开素材文件选择器，并只显示 `EPUB`、`PDF`、`TXT`、`DOCX`；选择任一文件后，后端自动扫描它所在目录。手动输入文件夹路径后点击扫描仍然支持。
3. 文件夹扫描阶段只保留 `EPUB`、`PDF`、`TXT` 和 `DOCX` 文件，其它扩展名不会进入任务列表。
3. 用户选择一个文件后点击“素材”。流水线页只保留素材路径输入、文件选择/扫描入口、三个阶段按钮、任务列表和最近生成结果，生成参数从配置读取。“素材”按钮按当前勾选任务或当前任务执行“只补缺失”：已完成且有成稿字数的任务会跳过，待处理、失败、生成中断或缺失成稿字数的任务才会重新生成；如果全部已完成，则提示无需重复生成。
4. 文件列表是流水线页的主区域，以任务表格呈现，列包括勾选、任务、格式、素材、进度、成稿字数、音频、时长和文件大小；表头支持全选，行首支持单选/多选。任务会保存到 SQLite，包含分类、状态、进度、成稿字数和失败信息；应用重启或切换菜单后可以恢复任务列表。界面状态全部显示中文，例如“待处理”“生成中”“已完成”“失败”“不可解析”“不支持”；进度只使用 `0%`、`25%`、`50%`、`75%`、`100%` 五档显示。素材生成按 4 个真实步骤刷新：`1/4 解析源书正文` 为 `25%`，`2/4 请求 AI 生成素材` 为 `50%`，`3/4 整理结果和字幕` 为 `75%`，`4/4 生成完成` 为 `100%`。
5. 任务列表支持右键菜单，提供继续生成、批量继续生成、打开文件、打开文件夹、打开素材文件夹、取消所选任务、从列表中移除、清理状态等操作；右键菜单使用与应用一致的绿色深色主题，不使用蓝色背景；双语字幕和同步类菜单先预留为禁用项，待后续音频/字幕/同步流程接入。“打开文件”使用资源管理器定位源文件，“打开文件夹”继续定位源文件；“打开素材文件夹”打开生成后的素材包目录，也就是包含 `title.txt`、`narration.txt`、`subtitles.txt`、`materials.json` 等文件的目录。该目录保存到 SQLite `material_tasks.material_output_dir`，由后端命令 `open_material_output_dir` 通过系统资源管理器打开；如果旧任务没有保存该字段，后端只会按源文件名或书名在默认 `exports` 目录查找匹配的素材包并回填，禁止回退到其它书的最近素材包；如果仍找不到，界面会显示明确错误。
6. 后端在生成阶段校验格式：`EPUB` 和 `TXT` 可解析；`DOCX` 和 `PDF` 可识别但正文解析暂未接入。
7. 后端解析源书正文，构建书籍概览、章节列表和章节素材包。
8. 后端构建中文听书视频 prompt，请求 OpenAI-compatible Chat Completions 接口；素材生成使用流式响应，避免长文本非流式请求被网关 524 截断。
9. AI 返回严格 JSON：`videoTitle`、`description`、`tags`、`narration`。
10. 如果旁白中文字数不在目标区间，后端执行硬校验：短于最小值时改为追加式修复，只请求“可接在原文后的旁白正文”并由应用合并计数；高于最大值时才请求整份 JSON 重写。修复最多尝试 3 次，仍不符合 `targetMinChars-targetMaxChars` 时任务失败，不允许短稿假成功。
11. 后端切分字幕行，返回完整素材结果。
12. 如果配置了飞书 Webhook，生成完成后发送飞书通知。
13. 用户可复制单项或全部素材，也可导出素材包。
14. 用户切换到“生成音频”页，可一键使用素材页生成的旁白，也可直接粘贴文本。
15. 后端使用配置中的微软语音 Speech Key、区域、音色和输出格式生成 mp3；长文本会自动分段。
16. 如果文本分为多个音频片段，后端使用配置中的外部 `ffmpeg.exe` 路径拼接最终 mp3。`ffmpeg.exe` 不随安装包打包。
17. 视频阶段必须先生成或导入与旁白内容相关的高清原图，保存到素材包 `visual_assets/originals`，并写入 SQLite `visual_assets`；禁止只使用截图、缩略图、视频帧或经过模糊处理的派生图作为最终视频背景源。
18. 生成图片时必须记录它对应的字幕/旁白片段，形成 `visual_timeline`：每张图绑定 `start_subtitle_index`、`end_subtitle_index`、`start_time`、`end_time` 和对应文本摘要。视频生成时按该时间轴铺设图片片段，而不是平均切图。

## 前端结构

前端使用 React、TypeScript、Zustand、lucide-react 和自定义 CSS。主要页面如下：

- `流水线`：任务列表优先的工作台。顶部是流水线分析入口和三个阶段按钮：`素材`、`音频`、`视频`。三个阶段按钮位于“流水线分析面板”标题行右侧，不占用素材路径输入行。其中 `素材` 运行文本素材生成；`音频` 作为批量 TTS 流水线入口预留；`视频` 作为视频流水线入口预留，当前不接真实逻辑。主体是流水线任务列表，最近生成结果在任务列表下方展示。
- `生成音频`：把素材旁白或手动文本合成为 mp3，展示输出目录、最终音频、SSML、分段数量和耗时。
- `操作日志`：以 IDEA 控制台风格展示后台日志，默认查看本次或最近一次生成任务日志。
- `配置`：管理素材生成默认参数、AI 模型、API Key、Base URL、飞书 Webhook、微软语音、外部工具路径、基础开关和更新检查入口。
- `关于`：展示应用版本和基础信息。

`materialsWorkbench` 状态保存在 Zustand 全局 store 中，包含请求参数、扫描结果、生成结果、导出目录、错误提示、复制状态、当前结果标签页、当前 `trace_id` 和忙碌状态。切换菜单后不丢失素材页状态。

素材生成默认参数保存在 `settings.materialProfile`，包括 `channelName`、`categoryName`、`categories`、`language`、`targetMinChars`、`targetMaxChars` 和 `extraDirection`。默认目标为 `7000-8300` 个中文字，最佳约 `7600` 字，用于配合 `0%` 原速语音生成约 `30-35` 分钟睡前听书音频，并尽量避免最终音频超过 `35:00`；如果用户调整目标时长，应优先调整这两个字数配置，而不是为了压缩时长提高语速。`categories` 默认包含 `半小时听完一本书`、`睡前听完一本书`、`A Book in 30 Minutes`，配置页允许新增分类；`categoryName` 是当前任务入库分类，等价于后续 YouTube 播放列表名称；`channelName` 为兼容旧生成提示词保留，当前选择分类时会同步更新。素材生成页不再直接编辑这些参数，生成请求会把当前配置合并进请求体。文件级生成状态同时保存在 `materialsWorkbench.fileStatuses` 和 SQLite `material_tasks`，按文件路径记录状态、五档进度、成稿字数和失败信息。

界面字体配置保存在 `settings.uiProfile`，包括 `menuFontFamily`、`menuFontSize`、`contentFontFamily` 和 `contentFontSize`。配置页“基础配置”允许分别设置左侧菜单字体和页面内容字体；默认菜单字号为 `13px`，内容字号为 `12px`。前端通过 CSS 变量 `--menu-font-family`、`--menu-font-size`、`--content-font-family` 和 `--content-font-size` 应用配置，页面表格、配置项和步骤跟踪内容默认跟随内容字体。

流水线跳过策略保存在 `settings.pipelineProfile`，包括 `skipExistingMaterials`、`skipExistingAudio` 和 `skipExistingVideo`。三项默认均为 `true`，配置页显示为“已有则跳过：是”；选择“否，每次重新生成”时，对应阶段不再因为已有素材包、音频或视频产物而跳过。任务列表列名使用 `素材进度`、`音频进度`、`视频进度`，状态列只显示阶段状态，进度列单独显示百分比。

流水线任务列表必须优先保证任务名称可读。列间距控制在 `2-4px`，当前实现为 `2px`；固定列使用窄列宽，任务列使用弹性宽度并设置最小宽度。视频状态不能只看是否存在视频文件：如果视频标记为成功，但视频时长明显短于音频时长，列表显示“异常”，视频进度显示 `-`，避免出现音频和视频都显示“已完成 / 100%”但实际产物明显不一致的误导。

`audioWorkbench` 状态保存在 Zustand 全局 store 中，包含旁白文本、输出目录、文件名、当前 `trace_id`、忙碌状态、错误提示和生成结果摘要。切换菜单后不丢失音频页状态。

右上角显示当前模型名，格式为 `【模型名】`。AI 测试连接前会先保存当前输入，测试成功后再次写入后端 `settings.json`，确保 API Key 被保存。

## 后端命令

Tauri 后端命令集中在 `src-tauri/src/commands.rs`：

- `get_app_state`：返回设置和版本。
- `get_settings` / `set_settings`：读取和保存设置。
- `check_update_mock`：当前为本地更新检查占位，返回当前版本。
- `test_ai_profile`：用当前 AI 配置发送最小测试请求。
- `generate_ai_text`：通用 AI 文本测试。
- `test_feishu_profile` / `send_feishu_message`：测试或发送飞书机器人消息。
- `test_speech_profile`：使用当前微软语音配置合成一小段测试音频，成功后保存配置。
- `preview_speech`：使用试听输入框文字生成短音频并返回文件路径供前端播放。
- `save_speech_region_key`：把当前区域、Speech Key、人声音色、输出格式、语速和音调保存到 SQLite。
- `get_speech_region_key`：按区域从 SQLite 读取已保存的默认语音配置。
- `get_speech_voices`：从 SQLite 读取微软文本转语音音色列表，可按 locale 过滤。
- `test_ffmpeg_path`：执行配置中的 `ffmpeg.exe -version`，验证外部工具路径可用，成功后保存配置。
- `scan_material_files`：扫描输入路径所在目录，只返回 `EPUB`、`PDF`、`TXT` 和 `DOCX` 文件，并把扫描到的文件 upsert 到 SQLite 任务表。
- `get_material_tasks`：按分类读取 SQLite 中已保存的素材任务列表，过滤掉当前已经不存在的源文件。
- `update_material_task_status`：更新单个素材任务的状态、五档进度、成稿字数和消息；如果用户直接粘贴文件路径生成而任务尚未入库，会按当前分类自动创建任务记录。
- `remove_material_task`：从 SQLite 任务表移除单个任务，不删除源文件。
- `reset_material_tasks`：把单个或全部任务重置为“待处理 / 0%”，不清空 SQLite 记录和源文件。
- `generate_book_materials`：生成听书素材主流程。
- `generate_audio`：使用微软语音生成旁白 mp3；长文本分段合成，多段时调用外部 `ffmpeg.exe` 拼接。
- `export_book_materials`：导出素材包。
- `get_operation_logs`：读取 SQLite 操作日志；传入主 `traceId` 时同时返回该 trace 和以 `主trace-` 开头的子阶段日志，保证一键视频的补素材、补音频和视频生成日志能在同一个任务视图中显示。

### 步骤跟踪页

左侧导航中的“生成音频”替换为“步骤跟踪”。步骤跟踪页参考 `video-easy-creator` 的步骤统计与步骤表结构，但数据直接来源于当前素材任务表，不额外引入独立步骤表。页面顶部展示当前任务、总步骤、步骤进度、任务摘要、任务 ID 和整体进度；下方按产物链拆成细步骤行，展示步骤编码、步骤名称、状态、百分比和说明。

当前步骤跟踪页拆分为 16 个步骤：生成 A：解析书籍；生成 B：标题简介标签；生成 C：旁白文本；生成 D：字幕文本；保存素材包；读取旁白文本；拆分音频片段；生成语音片段；合成最终音频；准备视频流水线；生成封面；生成图片；生成字幕；生成无字幕视频；生成硬字幕视频；登记视频产物。`MAT` 步骤编码按 A 到 Z 独立显示，禁止把 C/D 合并成一个步骤。页面先基于 `material_tasks` 的素材、音频、视频三组状态与进度字段映射这些子步骤，避免新增表结构；后续如果需要每个子步骤独立开始时间、结束时间、耗时和日志，可引入 `operation_step` 表承接。

素材阶段请求 AI 超时或失败时，失败会落在素材子步骤上，并在说明中显示后端写入的错误消息，例如 `HTTP 524`；这样用户可以从步骤跟踪页直接判断是 AI 素材 JSON 生成失败，而不是前端卡死。音频任务继续按读取旁白、拆分片段、生成语音片段和合成最终音频显示；视频任务按准备流水线、封面、图片、字幕、无字幕视频、硬字幕视频和登记产物显示。流水线任务列表中的音频与视频列同步显示“状态 + 百分比”。

后端应用状态 `AppData` 持有 `settings`、`settings_path`、`db_path` 和 `OperationLogger`。配置加载对嵌套配置块使用缺省兼容，旧版 `settings.json` 缺少新版字段时仍会保留已有 AI API Key、Base URL、模型、微软语音和工具路径；启动日志只记录配置路径、文件是否存在、Key 是否存在和脱敏长度，不输出密钥内容。

后端在 `generate_book_materials` 中通过 Tauri 事件 `material-task-progress` 推送素材任务进度，事件字段包括 `traceId`、`path`、`status`、`progress`、`step`、`totalSteps` 和 `message`。前端按当前 `traceId` 和文件路径匹配任务行，实时刷新任务列表并同步 SQLite 状态；最终生成结果返回后再写入“已完成 / 100% / 成稿字数”。

AI 请求层统一使用最多 3 次退避重试。HTTP 403、429 和 5xx 视为网关或服务端临时失败，分别等待 20 秒、40 秒后重试。兼容性验证发现当前网关会拒绝 `max_tokens` 和 `max_completion_tokens`，因此不得在本项目请求体中携带这两个字段。长旁白目标通过“AI 初稿 + 多轮小段追加 + 本地源书解读补足”实现：AI 初稿不可用时，后端会直接基于源书代表章节生成本地素材初稿；AI 初稿可用但连续追加被网关拦截或仍不足时，后端会从源书代表章节抽取片段，生成原创解读型补充旁白，并裁剪到 `targetMinChars-targetMaxChars` 范围内。

## 数据存储

设置文件保存在 Tauri app data 目录：

```text
%APPDATA%\com.abookin30minutes.desktop\settings.json
```

SQLite 数据库保存在同一 app data 目录：

```text
%APPDATA%\com.abookin30minutes.desktop\app.db
```

SQLite 保存素材任务列表：

```text
material_tasks(
  path TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  extension TEXT NOT NULL,
  size INTEGER NOT NULL,
  category TEXT NOT NULL DEFAULT '半小时听完一本书',
  status TEXT NOT NULL DEFAULT 'pending',
  progress INTEGER NOT NULL DEFAULT 0,
  narration_chars INTEGER,
  material_output_dir TEXT,
  message TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

任务表参考 `yt-download` 的任务持久化思路，以源文件路径作为主键，保存任务所属分类、处理状态、百分比、成稿字数、生成素材包目录和最近消息。`category` 对应后续 YouTube 播放列表名称；如果没有明确分类，使用默认分类 `半小时听完一本书`。扫描文件夹时只写入 `EPUB`、`PDF`、`TXT`、`DOCX`，其它格式不会显示到任务列表。生成开始写入 `generating / 25%`，生成成功写入 `success / 100% / narration_chars / material_output_dir`，失败写入 `failed / 0% / message`。前端每次读取任务列表时会从 SQLite 恢复状态，同时丢弃源文件已不存在的任务显示。

SQLite 额外保存微软语音默认配置：

```text
speech_region_keys(
  region TEXT PRIMARY KEY,
  speech_key TEXT NOT NULL,
  voice_name TEXT NOT NULL,
  output_format TEXT NOT NULL,
  rate TEXT NOT NULL,
  pitch TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

每个微软语音区域一条记录。配置页点击“保存默认语音配置”后写入该表；保存字段包括区域、Speech Key、人声音色、输出格式、语速和音调。切换区域下拉时通过 `get_speech_region_key` 从 SQLite 读取对应默认配置，并回填到输入框。`settings.json` 仅保留当前正在使用的 UI 配置，多区域默认配置以 SQLite 为准。

SQLite 额外保存微软文本转语音音色列表：

```text
speech_voices(
  voice_name TEXT PRIMARY KEY,
  locale TEXT NOT NULL,
  language TEXT NOT NULL,
  voice_type TEXT NOT NULL,
  gender TEXT NOT NULL,
  styles TEXT NOT NULL,
  roles TEXT NOT NULL,
  source_url TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

该表在应用启动时自动创建并写入内置种子数据。当前种子数据覆盖微软官方文本转语音文档中的中文普通话 `zh-CN` 29 个音色，并先内置英文 `en-US`、`en-GB` 的常用音色，来源 URL 为 `https://learn.microsoft.com/zh-cn/azure/ai-services/speech-service/language-support?tabs=tts`。配置页提供“语音语言”下拉，当前可选中文普通话、英语（美国）和英语（英国）；人声音色下拉通过 `get_speech_voices(locale)` 从 SQLite 读取，不再使用前端硬编码数组。后续日文、韩文、法语、德语可继续追加 seed 数据并开放到语言下拉。

SQLite 额外保存视觉原图资产。该表用于解决“AI 预览图清楚，但流程没有拿到原始高清文件”的问题。任何供正式视频使用的图片都必须先作为原图资产入库，视频阶段只消费 `kind` 为 `ai_original` 或 `imported_original` 的记录；`thumbnail`、`screenshot`、`derived`、`preview` 只允许用于预览或排查，不允许作为最终背景源。

```text
visual_assets(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  asset_id TEXT NOT NULL UNIQUE,
  material_output_dir TEXT NOT NULL,
  book_title TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL,
  source_provider TEXT NOT NULL DEFAULT '',
  prompt TEXT NOT NULL DEFAULT '',
  original_file TEXT NOT NULL,
  width INTEGER NOT NULL DEFAULT 0,
  height INTEGER NOT NULL DEFAULT 0,
  file_size INTEGER NOT NULL DEFAULT 0,
  sha256 TEXT NOT NULL DEFAULT '',
  sort_order INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'ready',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

字段约定：

- `asset_id` 使用稳定 ID，例如 `my-prizes-20260620-content-01`，供时间轴和视频 manifest 引用。
- `material_output_dir` 指向当前书籍素材包目录，避免不同书籍之间误复用图片。
- `kind` 表示资产来源和可用级别：`ai_original` 为 Codex/Imagegen 或后端图片模型生成的原始图，`imported_original` 为用户导入的高清授权图，`derived` 为裁切、压缩、转码或风格化后的派生图，`thumbnail` 和 `screenshot` 不可进入最终视频。
- `source_provider` 记录生成或导入来源，例如 `codex_imagegen_builtin`、`openai_image_api`、`user_import`。
- `prompt` 保存生成提示词或内容节点摘要，便于后续复现和审查。
- `original_file` 必须指向素材包内的原始图片副本，不能只指向 Codex 默认生成目录或系统临时目录。
- `width`、`height`、`file_size`、`sha256` 用于质量检查、去重和确认没有误拿缩略图。

SQLite 额外保存图片与字幕的时间轴映射。该表把“文本、图片、视频、字幕”串成同一条链路：图片不是素材池里的随机背景，而是由明确字幕段落对应文本生成，并在视频中从对应字幕开始时间显示到对应字幕结束时间。

```text
visual_timeline_segments(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timeline_id TEXT NOT NULL UNIQUE,
  material_output_dir TEXT NOT NULL,
  book_title TEXT NOT NULL DEFAULT '',
  asset_id TEXT NOT NULL,
  asset_file TEXT NOT NULL,
  subtitle_file TEXT NOT NULL,
  title TEXT NOT NULL DEFAULT '',
  start_subtitle_index INTEGER NOT NULL DEFAULT 0,
  end_subtitle_index INTEGER NOT NULL DEFAULT 0,
  start_time TEXT NOT NULL DEFAULT '',
  end_time TEXT NOT NULL DEFAULT '',
  start_ms INTEGER NOT NULL DEFAULT 0,
  end_ms INTEGER NOT NULL DEFAULT 0,
  duration_ms INTEGER NOT NULL DEFAULT 0,
  prompt_source TEXT NOT NULL DEFAULT '',
  source_text_preview TEXT NOT NULL DEFAULT '',
  source_text_chars INTEGER NOT NULL DEFAULT 0,
  rationale TEXT NOT NULL DEFAULT '',
  transition TEXT NOT NULL DEFAULT 'cut',
  status TEXT NOT NULL DEFAULT 'ready',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
)
```

时间轴生成规则：

- 输入优先使用真实音频对齐后的 `subtitles/*.zh.srt` 或 `.ass`；如果尚未做真实对齐，才允许使用估算 SRT。
- 每个时间轴片段必须引用一个 `visual_assets.asset_id`，并冗余保存 `asset_file`，便于视频脚本脱离数据库排查。
- `start_subtitle_index` / `end_subtitle_index` 记录字幕行号，`start_time` / `end_time` 记录 SRT/ASS 时间，`start_ms` / `end_ms` 用于视频轨道铺设。
- `prompt_source` 是生成该图时使用的文本节点或提示词摘要；`source_text_preview` 保存对应字幕文本预览，方便确认图片与内容是否匹配。
- 切图不按图片数量平均分配，而按内容锚点生成，例如“买西装”“肺病医院”“法兰克福修稿”“国家文学奖讲话”“奖金修窗”“退出科学院”等节点。
- 视频生成必须优先读取 `visual_timeline_segments`。如果没有时间轴，界面应提示先生成视觉时间轴，而不是静默平均切图。

文本 log 文件作为备用写入 local app data 的 `logs` 目录，但“操作日志”页面读取 SQLite `operate_log`，不读取文本 log 文件。

导出目录默认位于：

```text
%APPDATA%\com.abookin30minutes.desktop\exports
```

每个素材包的视频视觉资产目录约定如下：

```text
visual_assets/
  originals/
    <批次>/
      01_awards_hall_cold_ceremony.png
      02_vienna_menswear_shop.png
      visual_assets_manifest.json
      visual_timeline.json
  derived/
    <批次>/
      scene_01_1920x1080.jpg
      scene_01_preview.jpg
```

`originals` 保存可追溯的高清原图和 manifest；`derived` 保存为剪映、ffmpeg 或预览生成的转码、裁切、降采样副本。正式视频铺图应从 `visual_timeline.json` 或 `visual_timeline_segments` 找到 `ai_original/imported_original` 原图，再按输出分辨率确定是否生成派生副本。不得把 `derived` 反向覆盖 `originals`。

音频输出目录默认位于：

```text
%APPDATA%\com.abookin30minutes.desktop\audio_exports
```

当前微软语音 UI 配置和工具路径写入 `settings.json`：

- `speechProfile.provider`：固定为 `azure_microsoft`。
- `speechProfile.speechKey`：当前区域正在使用的微软语音 Speech Key。日志只记录是否存在和长度，不记录明文。
- `speechProfile.region`：例如 `eastasia`、`eastus`。
- `speechProfile.locale`：微软语音 locale，例如 `zh-CN`、`en-US`、`en-GB`；SSML 的 `xml:lang` 使用该字段。
- `speechProfile.voiceName`：例如 `zh-CN-YunxiNeural`。
- `speechProfile.outputFormat`：默认 `audio-24khz-160kbitrate-mono-mp3`；最终合并文件由 ffmpeg 统一输出为 `48kHz / 192kbps / stereo` MP3，兼顾旁白清晰度和播放器兼容性。
- `speechProfile.rate` / `speechProfile.pitch`：SSML prosody 参数，默认 `0% / +0Hz`，使用微软语音原速朗读；时长控制优先通过 `7000-8300` 字素材目标完成，不通过加速压缩音频。
- `toolProfile.ffmpegPath`：外部 `ffmpeg.exe` 完整路径。安装包不包含 ffmpeg。

微软语音配置页提供两个外部链接：

- Azure Portal 语音资源列表，用于进入 Speech 资源并在 `Keys and Endpoint` 查看 Key 和区域。
- Microsoft Learn 文本转语音 REST API 文档，用于确认请求格式、区域和输出格式。

区域选择使用下拉框，显示中文区域名、英文区域名和微软 region code，例如 `东亚 East Asia (eastasia)`。保存到配置中的仍是 region code。

语音语言使用独立下拉框，当前包含 `zh-CN`、`en-US` 和 `en-GB`。切换语言后，前端会重新从 SQLite 读取该 locale 下的人声音色；如果当前 voice code 不属于新语言，会自动选择该语言下的第一条音色。人声音色下拉显示短音色名、性别、首个风格标签和微软 voice code，例如 `Yunxi 男声，assistant (zh-CN-YunxiNeural)`。保存到配置中的仍是 voice code。

微软语音配置页提供试听区。默认试听文字为“夜深了，我们用半小时，慢慢听完一本书。愿故事里的光，也照进你今晚的梦里。”，用户可以直接修改。点击“播放试听”后，后端使用当前区域、当前 Key、当前音色、输出格式、语速和音调生成短音频，返回本地 mp3 路径和 `data:audio/mpeg;base64,...`，前端优先播放 data URL，避免 WebView 本地文件协议无法识别 mp3 源。

## 源书解析设计

当前支持：

- `EPUB`：解包 ZIP，读取 `META-INF/container.xml`，定位 OPF，解析 manifest、spine、NCX 目录，读取 HTML/XHTML 章节并清洗正文。
- `TXT`：直接读取文本，作为单章节书籍处理。

暂未接入：

- `DOCX`：生成阶段返回“已识别但正文解析还未接入”。
- `PDF`：生成阶段返回“已识别但正文解析还未接入”。

EPUB 正文清洗规则：

- 删除 `<script>` 和 `<style>` 块。
- 将段落、标题、列表、正文容器等块级标签替换为换行。
- 删除剩余 HTML 标签。
- 解码基础 XML/HTML 实体。
- 压缩空白和连续空行。

2026-06-18 修复点：Rust `regex` 不支持反向引用，因此不能使用 `</\1>` 形式匹配 `script/style`。当前拆成两个受支持的正则，避免解析正文时 panic。`generate_book_materials` 也给 `source.read` 增加了 panic 和超时兜底，后续内部异常会写入 `source.read.panic` 并返回界面；解析超过 30 秒会写入 `source.read.timeout` 并抛出明确错误，避免用户一直等待。

外部请求超时策略：

- AI 请求总超时 600 秒，连接超时 30 秒；超时后返回“AI 请求失败或超过 600 秒未返回”。
- 飞书请求总超时 20 秒，连接超时 10 秒；超时后返回“飞书请求失败或超过 20 秒未返回”。
- 微软语音请求总超时 120 秒，连接超时 20 秒；每个音频分段独立请求，失败会写入本次任务日志并返回界面。

2MB 级 EPUB 的解析应为秒级。如果日志长时间停在 `source.read`，优先检查 EPUB 解析异常、压缩包结构和 HTML 清洗逻辑；AI 生成慢通常发生在 `ai.request` 之后。

## Prompt 与生成策略

默认目标是 7000 到 8300 个中文字，最佳约 7600 个中文字，配合默认 `0%` 原速，适配 30 到 35 分钟睡前听书音频的文本素材准备。

Prompt 设计原则：

- 不是逐字朗读，也不是替代原书，而是原创转述、摘要、评论和解读。
- 不照抄原文长句，不输出大段原书内容。
- 不平均覆盖所有章节，选择 5 到 7 个更适合视频叙事的节点。
- 结构接近“睡前听完一本书”：开场陪伴感、作者/地点引入、生活趣味、人物创造力、现实冲突、命运转折、主题升华、晚安式结尾。
- 旁白口语化、短句多，适合 AI 朗读和字幕切分；素材阶段就要求 AI 频繁使用逗号、顿号、分号和句号拆开短半句，每个半句尽量 6 到 14 个汉字，最长不超过 18 个汉字。
- 只接受严格 JSON，不接受 Markdown。

如果 AI 返回的 `narration` 中文字数不在目标范围，后端会构建修复 prompt，要求保留同一素材结构并重写 JSON。

## 字幕与导出

字幕切分必须从素材阶段源头完成，而不是视频阶段临时补救。`subtitles.txt` 一行只承载一个短意群，后端按 `。！？；，、`、英文同类标点和换行切分；如果 AI 仍返回过长半句，后端会按 14 个字符强制切块。目标是“每个半句就是一句字幕”，避免剪映字幕出现一行塞多个半句、英文换行溢出或字幕遮挡画面。

素材生成完成并生成最终旁白音频后，正式字幕时间轴必须由 aeneas 基于“实际音频 + `subtitles.txt`”强制对齐生成，不再使用按总时长和文本权重估算的时间轴作为发布版本。aeneas 的中文音频参数使用 `task_language=cmn`，英文音频使用 `task_language=eng`；项目输出文件名仍使用 `chn.srt` 表示中文字幕，例如 `*.aeneas.chn.srt`。manifest 必须写入 `audioLanguage=cmn`、输入音频、输入文本、输出 SRT/ASS 和首尾字幕时间。

英文字幕不重新估算时间轴，而是复用 aeneas 生成的中文 cue 时间，按字幕编号从 `translation_cache.json` 读取英文翻译，输出英文 SRT、双语 SRT 和双语 ASS。aeneas 失败时必须显式报错，不能静默回退为估算时间轴；估算脚本只允许用于早期草稿或环境不可用时的临时预览。

### 通用 EPUB 到硬字幕流水线

为了避免每本书都靠临时命令串接，当前验证脚本 `tmp/book_video_pipeline.py` 被定义为通用流水线入口。书籍可以变化，但流程固定为“源书 EPUB -> 素材/音频 -> 翻译缓存 -> aeneas 字幕 -> 封面和视频图片 -> 无字幕视频 -> 硬字幕视频”。命令示例：

```powershell
python tmp\book_video_pipeline.py --epub <源书.epub> --clean --skip-notify --audio-language cmn --source-image-dir <正式图片目录>
```

如果素材和音频已经生成，必须优先使用恢复模式，避免重复消耗 AI 和 TTS：

```powershell
python tmp\book_video_pipeline.py --epub <源书.epub> --material-dir <已有素材目录> --skip-notify --audio-language cmn --source-image-dir <正式图片目录>
```

流水线参考 `D:\04_GitHub\video-easy-creator` 的 `operation_history`、`operation_step`、`operation_event` 思路，在 `C:\Users\Administrator\AppData\Roaming\com.abookin30minutes.desktop\app.db` 中记录整次任务和每个步骤状态。启动时会自动把历史 `RUNNING` 收口为 `FAILED`，避免上下文中断后留下假运行状态。当前步骤约定：

| 步骤 | 名称 | 主要产物 |
| --- | --- | --- |
| `STEP01` | 恢复/生成素材和音频 | `materials.json`、`narration.txt`、`subtitles.txt`、`audio/**/*.mp3`、`audio_manifest.json` |
| `STEP02` | 生成翻译缓存 | `translation_cache.json`、草稿 `zh/en/zh-en` 字幕 |
| `STEP03` | 生成 aeneas 字幕 | `*.aeneas.chn.srt`、`*.aeneas.en.srt`、`*.aeneas.zh-en.srt`、`*.aeneas.zh-en.ass` |
| `STEP04` | 生成封面和视觉时间轴 | `visual_assets/covers/**/cover_manifest.json`、`visual_assets/originals/**/visual_assets_manifest.json`、`visual_timeline.json` |
| `STEP05` | 渲染无字幕视频 | `video/cover_timeline_no_subtitle_*/<name>.mp4` 和 `render_manifest.json` |
| `STEP06` | 压制硬字幕视频 | `<name>.aeneas.hardsub.mp4` 和 `<name>.aeneas.hardsub_manifest.json` |

Python 子进程必须设置 `PYTHONIOENCODING=UTF-8`，命令输出和 JSON manifest 必须保持 UTF-8，防止 Windows 控制台在长中文路径下抛出 `UnicodeEncodeError`。硬字幕压制阶段使用 `tmp/burn_ass_hardsub.py`，带封面片头时把 ASS Dialogue 整体延后；延迟后的中间 ASS 文件固定使用短 ASCII 名 `hardsub_delay5000ms.ass`，避免 ffmpeg/libass 在超长中文路径或文件名上 `fopen failed`。

正式视频阶段必须提供 `--source-image-dir`，指向已经生成或导入的 16:9 正式内容图目录。`tmp/generate_book_visuals.py` 的 Pillow 占位模板只允许草稿验证，不能静默进入发布成片；如果确需草稿占位，必须显式传 `--allow-placeholder-visuals`，并在日报和 manifest 中标明该成片不可作为最终版本。

导出素材包包含：

- `title.txt`
- `description.txt`
- `tags.txt`
- `narration.txt`
- `subtitles.txt`
- `draft.srt`
- `prompt.txt`
- `overview.json`
- `materials.json`
- `README.md`

素材生成成功后，后端会自动写出一份素材包到默认 `exports` 目录，并将该目录写入当前任务的 `material_output_dir`。用户手动点击“导出素材包”仍可按当前输出目录再次导出一份副本；任务右键【打开素材文件夹】打开的是任务表记录的自动生成素材包目录。对于升级前已经成功但没有 `material_output_dir` 的旧任务，后端会用源文件名或书名在默认 `exports` 下匹配包含 `materials.json` 或 `narration.txt` 的最近目录，找到后回填任务表并打开；如果没有匹配目录，则要求重新生成，避免误打开其它书的素材包。

前端 Tauri 命令调用统一通过 `formatCommandError` 格式化错误：优先展示 `Error.message`，其次展示后端 `{ message }` 字段，避免页面出现 `[object Object]`。

`draft.srt` 可以作为素材阶段预览草稿存在，但最终视频、剪映草稿和硬字幕成片都应优先使用 aeneas 对齐后的 `*.aeneas.*.srt` / `*.aeneas.*.ass`。

## 音频生成设计

“生成音频”页是素材生成后的第二阶段，也支持独立使用。用户可以点击“使用素材旁白”把素材页当前 `materials.narration` 带入，也可以直接粘贴文本。流水线页的【音频】按钮则面向任务列表批处理：优先读取每个素材任务的 `material_output_dir/narration.txt`，按任务逐个生成音频并回填 SQLite 状态。

音频生成不固定按“每 100 句”切分，而是采用“句子边界优先 + 字符数约束 + 句子数约束 + 预计时长约束”的分块策略：

- 先按 `。！？!?`、换行等自然边界切句，保留句末标点。
- 每个音频分块最多约 100 句，最多约 2200 个字符，预计朗读时长不超过 8 分钟。
- 单句超过字符上限时再按字符硬切，避免单次微软语音请求过大。
- 8 分钟是安全余量，用于规避微软实时 TTS 单次请求 10 分钟音频上限，也避免 SSML 过大或长请求超时。
- 分块计划写入 `audio_manifest.json`，每个分块记录句子范围、字符数、预计时长、状态、文件路径、错误信息和耗时，便于失败续跑和排查。

输出目录结构：

```text
audio/
  parts/
    part_001.mp3
    part_002.mp3
  ssml/
    part_001.ssml
    part_002.ssml
  concat.txt
  audio_manifest.json
  narration.ssml
  <书名>.mp3
```

其中 `parts` 保存分段音频，`ssml` 保存每段实际请求体，`narration.ssml` 保存完整归档版 SSML，`concat.txt` 是 ffmpeg concat demuxer 清单，`audio_manifest.json` 是任务续跑和结果回填依据。

后端生成流程：

1. 校验微软语音配置：Speech Key、区域、音色名称、输出格式必须存在。
2. 校验输入文本不能为空。流水线入口会先校验素材任务已生成素材包且 `narration.txt` 存在。
3. 按分块策略生成音频计划，写入 `audio_manifest.json` 初始状态。
4. 多段音频时先校验 `toolProfile.ffmpegPath`，确保外部 `ffmpeg.exe` 可执行。
5. 为每段文本构建 SSML，使用 Azure/Microsoft Speech REST TTS 接口：
   - URL：`https://{region}.tts.speech.microsoft.com/cognitiveservices/v1`
   - Header：`Ocp-Apim-Subscription-Key`
   - Header：`X-Microsoft-OutputFormat`
   - Body：`application/ssml+xml; charset=utf-8`
6. 每个分段生成成功后立即更新 manifest；如果目标 `part_xxx.mp3` 已存在且大小大于 0，续跑时可跳过该分段。
7. 全部分段成功后生成 `concat.txt`，单段时复制为最终 mp3，多段时调用 `ffmpeg -y -f concat -safe 0 -i concat.txt -c copy <final>.mp3`。
8. 生成完成后用 ffprobe 或 ffmpeg 输出解析最终音频时长，写回任务表。
9. 返回输出目录、最终音频、SSML、manifest、分段列表、文本字数、分段数量、音频时长和耗时。

SQLite `material_tasks` 在素材状态字段之外追加音频阶段字段：

```sql
audio_status TEXT DEFAULT 'pending',
audio_progress INTEGER DEFAULT 0,
audio_output_dir TEXT,
audio_file TEXT,
audio_duration_ms INTEGER,
audio_chunks INTEGER,
audio_message TEXT
```

`audio_status` 使用 `pending/generating/success/failed` 四态，前端显示为 `待生成/生成中/已完成/失败`；`audio_progress` 使用 `0/25/50/75/100` 五档，其中 `25%` 表示已读取旁白和完成计划，`50%` 表示正在生成分段，`75%` 表示分段完成正在合并，`100%` 表示最终 mp3 已生成。

后端命令：

- `generate_audio`：独立音频页入口，输入文本和输出目录，返回完整生成结果。
- `generate_material_task_audio`：流水线任务入口，输入任务源文件路径，自动读取任务素材包下的 `narration.txt`，生成音频并更新任务音频字段。
- `get_material_tasks`：返回任务列表时一并返回音频状态、音频路径、音频时长、分段数和消息。

日志动作包括：

- `audio.generate.start`
- `audio.generate.plan`
- `audio.manifest.write`
- `audio.speech.request`
- `audio.speech.response`
- `audio.speech.request.failed`
- `audio.ffmpeg.concat`
- `audio.ffmpeg.concat.done`
- `audio.ffmpeg.concat.failed`
- `audio.duration.probe`
- `audio.task.update`
- `audio.generate.done`

音频任务的 `trace_id` 默认使用 `audio-YYYYMMDD-HHMMSS-random`；如果从素材页带入，则使用素材任务 ID 派生，便于在日志页串联查看文本和音频阶段。

## 视频视觉资产与时间轴设计

视频阶段的核心不是随机轮播背景图，而是把旁白文本、字幕、图片和视频轨道绑定成可追溯的时间轴。设计目标有三个：

- 确保 AI 生成的高清原图被应用真正拿到，并保存到当前书籍素材包。
- 确保每张图知道自己由哪一段字幕/旁白文本生成。
- 确保剪映草稿、ffmpeg 预览和完整视频使用同一份图片时间轴。

### 图片生成与落盘

图片可以来自 Codex `imagegen`、后端图片 API 或用户导入的授权图片。无论来源如何，进入正式视频前都必须执行同一套资产登记流程：

1. 生成或导入图片后，先定位原始高清文件。
2. 将原图复制到当前素材包 `visual_assets/originals/<批次>/`，保留原始生成目录副本，不直接依赖临时目录。
3. 计算宽高、文件大小和 SHA256。
4. 写入 `visual_assets_manifest.json`。
5. 写入 SQLite `visual_assets`，`kind` 必须是 `ai_original` 或 `imported_original` 才能被最终视频使用。

图片生成提示词必须来自内容节点，而不是泛化的“漂亮背景”。例如《我的文学奖》的正式节点包括：

- 文学奖的冷光与荒诞。
- 维也纳男装店与领奖前买西装。
- 肺病医院、病房、纸牌与死亡。
- 法兰克福旅馆修稿《严寒》。
- 华沙冬天、校样和金钱暗线。
- 奥地利国家文学奖讲话冷场。
- 维尔德甘斯奖、奖金、修窗和现实需要。
- 退出科学院与离开制度房间。

图片质量检查要求：

- 原图分辨率应接近或高于视频目标比例，当前长视频默认 16:9。
- 文件大小和 SHA256 必须记录，避免误用几十 KB 的截图帧或缩略图。
- 不使用模糊、雾化、截图裁剪、视频帧截取作为正式背景源。
- 如果需要裁切为 1920x1080，应生成 `derived` 副本，不能覆盖原图。

### 图片-字幕映射

图片生成完成后必须建立 `visual_timeline`。每个时间轴片段包含：

- `asset_id` / `asset_file`：对应 `visual_assets` 中的原图资产。
- `start_subtitle_index` / `end_subtitle_index`：该图覆盖的字幕行范围。
- `start_time` / `end_time`：从 SRT/ASS 读取的实际字幕时间。
- `start_ms` / `end_ms` / `duration_ms`：视频轨道铺设使用的毫秒时间。
- `prompt_source`：该图生成时对应的文本节点摘要。
- `source_text_preview`：该时间段内字幕文本预览。
- `rationale`：为什么这张图适合这一段。

时间轴示例：

```text
00:00:00,000 -> 00:03:22,660  开场：文学奖的冷光与荒诞
00:03:22,660 -> 00:06:03,984  格里尔帕策奖：领奖前买西装
00:06:03,984 -> 00:08:51,244  肺病医院：病房、纸牌与死亡
00:08:51,244 -> 00:09:40,289  《严寒》：法兰克福旅馆修稿
00:09:40,289 -> 00:15:45,791  华沙冬天与金钱暗线
00:15:45,791 -> 00:20:24,216  奥地利国家文学奖：台上冷场
00:20:24,216 -> 00:28:12,077  维尔德甘斯奖：奖金、修窗与现实
00:28:12,077 -> 00:35:15,550  退出科学院：离开制度房间
```

该映射同时写入：

- `visual_assets/originals/<批次>/visual_timeline.json`，便于人工检查、脚本调试和跨机器迁移。
- SQLite `visual_timeline_segments`，便于应用页面、后端命令和视频生成流程统一查询。

### 视频消费规则

视频生成流程必须读取 `visual_timeline_segments` 或 `visual_timeline.json`，按 `start_ms` / `duration_ms` 创建图片片段：

- 剪映草稿：每个时间轴片段生成一个背景图片素材片段，起止时间与字幕时间轴一致。
- ffmpeg 预览/直出：按时间轴生成 concat/filter_complex，不再平均循环背景图。
- 字幕轨：继续使用同一份 SRT/ASS，保证图片切换点与字幕内容一致。
- 背景音乐：只跟随完整视频时长，不影响图片/字幕时间轴。

如果图片时间轴缺失或图片文件不存在，视频阶段应失败并给出明确提示，例如“请先生成视觉资产时间轴”，而不是静默回退到截图、默认占位图或平均切图。

### 《我的文学奖》封面与视频图片设计记录

本批次视觉方向是“冷静、出版感、舞台疏离感”。画面不追求热闹的颁奖氛围，而是用空大厅、夜色、背影、冷光和制度化空间，呼应托马斯·伯恩哈德在《我的文学奖》中对荣耀、奖金、疾病、公共仪式和文学制度的讽刺。人物尽量采用背影或侧影，不生成真实作家肖像；所有中文标题、作者、栏目名都由本地 Pillow 模板绘制，AI 图像只负责无字底图，避免 AI 直接生成中文导致错字。

封面设计思想：

- 结构：左上角为频道系列名，右上角为集数，右半区为书名和作者，底部为两行内容钩子与频道脚注。
- 视觉：深色颁奖大厅、红色帷幕、夜窗、讲台和孤独背影，形成“仪式很隆重，但人很疏离”的气质。
- 字体：中文主标题、作者和系列名使用微软雅黑粗体/常规，英文脚注使用 Noto Sans；系列名标签框按文字 bbox 动态计算宽高，水平和垂直居中。
- 色彩：炭黑、暗红、冷蓝灰和旧金色，不使用高饱和营销感配色。
- 文本策略：AI 生成底图不含文字；`tmp/create_series_cover.py` 在本地绘制所有文字层，输出 `1920x1080` PNG 和 `1280x720` JPG。

封面底图提示词：

```text
cinematic literary award hall at night in Vienna, empty ceremonial auditorium, red velvet curtains, cold spotlight on a small wooden podium, rows of dark chairs, a solitary older male figure seen from behind in the lower left, European classical interior, quiet ironic atmosphere, elegant but unsettling, no readable text, no logos, no banners, no subtitles, 16:9 composition, high detail, realistic painterly cinematic lighting
```

封面文字层模板：

```text
bookTitle: 我的文学奖
author: 托马斯·伯恩哈德
seriesName: 半小时听完一本书
episode: VOL. 01
deck:
  - 从颁奖台、病房、旅馆修稿，到退出科学院。
  - 三十五分钟，听一个作家怎样把荣耀写成讽刺。
footerLeft: A BOOK IN 30 MINUTES
footerRight: 睡前听书系列
```

8 张内容图设计思想与提示词：

| 序号 | 文件 | 设计思想 | 提示词 |
| --- | --- | --- | --- |
| 1 | `01_awards_hall_cold_ceremony.png` | 用空旷颁奖大厅、讲台、奖状和背影表现“文学奖的冷光与荒诞”，让荣耀看起来像一套冷冰冰的制度。 | `empty European literary award ceremony hall at night, cold spotlight on podium, rows of chairs, framed certificates on side tables, solitary male figure seen from behind, red velvet curtains, marble floor, Vienna atmosphere, elegant but alienating, no readable text, no logos, cinematic 16:9, high detail` |
| 2 | `02_vienna_menswear_shop.png` | 领奖前买西装是现实生活细节，用维也纳男装店把文学荣誉拉回金钱、体面和尴尬。 | `old Vienna menswear shop interior, dark wooden shelves, black formal suits, measuring tape, mirror reflections, a nervous solitary customer seen from behind, 1960s European mood, warm shop light mixed with cold window light, literary cinematic realism, no readable text, no logos, 16:9` |
| 3 | `03_tuberculosis_ward_cards.png` | 肺病医院段落需要把疾病、无聊和死亡感放在同一张图里，纸牌是生活还在继续的讽刺性细节。 | `mid century tuberculosis hospital ward, narrow beds, pale sheets, small table with playing cards, winter light through tall windows, quiet patients implied but not shown clearly, sterile and melancholic atmosphere, muted colors, realistic cinematic still, no readable text, 16:9` |
| 4 | `04_frankfurt_hotel_revisions.png` | 旅馆修稿强调写作的孤独劳动，画面重点是桌面、手稿、台灯和临时栖居感。 | `Frankfurt hotel room at night, small desk with manuscript pages and pencil corrections, warm desk lamp, rain on window, suitcase half open, lonely writer implied by coat on chair, European literary realism, intimate but austere, no readable text, no logos, 16:9` |
| 5 | `05_warsaw_winter_dorm_proofs.png` | 华沙冬天与校样段落用冷色学生宿舍表现异乡、寒冷、校样和金钱压力。 | `Warsaw winter student dormitory room, frosted window, stacks of proof pages on a small desk, dim radiator, simple bed, cold blue gray light, Eastern European austerity, a small envelope of money on the table, cinematic quiet realism, no readable text, 16:9` |
| 6 | `06_state_award_speech_tension.png` | 国家文学奖讲话冷场需要把台上正式感与台下压抑反应并置，体现公共仪式中的冒犯和尴尬。 | `Austrian state literary award ceremony, formal hall, podium under hard spotlight, officials seated stiffly in the shadows, tense silence, speaker seen from behind or side silhouette, red curtains, marble walls, cold ceremonial mood, no readable text, no logos, cinematic 16:9` |
| 7 | `07_award_money_window_repair.png` | 维尔德甘斯奖段落把奖金和修窗并置，强调文学奖金最终被现实生活吞掉。 | `night apartment interior in Vienna, cracked window being repaired, envelope of award money on wooden table, tools, receipts, cold street light outside, quiet domestic realism, literary irony, no readable text, no logos, cinematic 16:9` |
| 8 | `08_academy_withdrawal_dining_room.png` | 退出科学院不是激烈反抗，而是从制度餐厅安静离开；画面用空餐桌和背影表达退出。 | `empty academy dining room after a formal dinner, long table with white cloth, abandoned glasses and plates, tall windows at night, a solitary figure leaving through a doorway seen from behind, institutional elegance turning cold, no readable text, no logos, cinematic 16:9` |

当前无字幕成片规则：

- 片头先显示系列封面 `5` 秒。
- 旁白音频延后 `5000ms` 开始，保证封面片头不截断原始旁白。
- 片头后按 `visual_assets/originals/20260621_0820_content_images/visual_timeline.json` 铺设 8 张内容图。
- 输出视频不烧录字幕，`render_manifest.json` 中 `hardSubtitles=false`。
- 本轮输出：`video/cover_timeline_no_subtitle_20260621_1539/我的文学奖_cover_timeline_no_subtitle_20260621_1539.mp4`。

当前硬字幕成片规则：

- 硬字幕以无字幕完整视频为源，禁止把已经烧录过字幕的 `.hardsub.mp4` 再次作为源视频。
- 带封面片头的视频烧录 ASS 前必须把所有 Dialogue 整体延后 `5000ms`，第一条字幕从 `0:00:05.00` 开始。
- 双语 ASS 样式参考 `yt-download` 的发布效果：中文 `Microsoft YaHei UI`、字号 `128`、浅黄色 `&H00C8F6FF`、粗体、黑描边 `5`、阴影 `2`、底边距 `238`；英文同字体、字号 `82`、橙色 `&H001AA5F2`、粗体、黑描边 `5`、阴影 `2`、底边距 `140`。
- ffmpeg 压制沿用 `yt-download` 的硬字幕策略：`ass=` 滤镜、输出 `.hardsub*.mp4`、临时 `.mp4.part`、显式 `-f mp4`、视频 `libx264 -preset veryfast -crf 18`、音频 copy。
- 本轮推荐硬字幕输出：`video/cover_timeline_no_subtitle_20260621_1539/我的文学奖_cover_timeline_no_subtitle_20260621_1539.aeneas.hardsub.mp4`。
- 推荐字幕来源：`subtitles/aeneas_20260621_1951/*.aeneas.zh-en.ass`，cue 数 `1288`，首条字幕 `0.0 -> 1.0 晚上好`，末条字幕 `2113.6 -> 2115.52 晚安`。
- 验证帧位于 `video/cover_timeline_no_subtitle_20260621_1539/hardsub_aeneas_frames`，其中 `cover_2s.jpg` 确认片头无字幕，`subtitle_7s.jpg`、`subtitle_10m.jpg`、`subtitle_34m.jpg` 确认 aeneas 时间轴硬字幕正常显示。

### 《布尔乔亚》通用流水线验证记录

本批次用于验证“书籍会变化”的通用流水线，源书为 `E:\迅雷下载\0308新书四本\2025-01《布尔乔亚》【豆瓣评分9.0】\2025-01《布尔乔亚》【豆瓣评分9.0】.epub`。执行入口为 `tmp/book_video_pipeline.py`，恢复模式复用已生成素材目录，SQLite `operation_history.id=4`、`operation_key=book-video-pipeline`，`STEP01` 到 `STEP06` 均为 `SUCCESS`。

关键产物：

- 素材目录：`C:\Users\Administrator\AppData\Roaming\com.abookin30minutes.desktop\exports\20260621_212132_布尔乔亚：在历史与文学之间_【意】弗朗哥·莫莱蒂_【意】弗朗哥·莫莱蒂`
- 音频：`audio\20260621_212132_pipeline\20260621_212132_布尔乔亚：在历史与文学之间_【意】弗朗哥·莫莱蒂_【意】弗朗哥·莫莱蒂.mp3`，时长 `00:26:20.98`，大小 `37944044` 字节，旁白约 `6277` 个中文字。
- aeneas 字幕目录：`subtitles\aeneas_20260621_222245`，`cueCount=1019`，包含 `*.aeneas.chn.srt`、`*.aeneas.zh.srt`、`*.aeneas.en.srt`、`*.aeneas.zh-en.srt`、`*.aeneas.zh-en.ass`。
- 封面：`visual_assets\covers\20260621_230049_series_cover\布尔乔亚：在历史与文学之间_【意】弗朗哥·莫莱蒂_series_cover_1920x1080.png`。
- 视频图片时间轴：`visual_assets\originals\20260621_230049_formal_content_images\visual_timeline.json`，共 8 段正式 AI 内容图。
- 无字幕视频：`video\cover_timeline_formal_no_subtitle_20260621_2301\20260621_212132_布尔乔亚_formal_cover_timeline_no_subtitle.mp4`。
- 硬字幕视频：`video\cover_timeline_formal_no_subtitle_20260621_2301\20260621_212132_布尔乔亚_formal_cover_timeline.aeneas.hardsub.mp4`。
- 已作废版本：`visual_assets\originals\20260621_222255_generic_content_images` 和 `video\cover_timeline_no_subtitle_20260621_222257` 使用了 Pillow 通用占位背景，只能作为流水线结构验证，不再作为最终成片。

封面设计思想：

- 风格：`generic_literary_stage_cover_v1`，用于非单本定制图片不足时的通用文学听书封面。
- 构图：深色舞台/阅读厅，抽象竖柱、圆形和书本/讲台意象，右侧放书名和作者，左上角放“半小时听完一本书”，底部放两行栏目文案。
- 文本清洗：`normalize_cover_title` 只把主书名《布尔乔亚》作为大标题，副标题和作者进入小字区域；`wrap_title` 按安全宽度自动换行和缩小，避免 EPUB 元数据过长造成溢出。
- 文字策略：所有中文、英文和系列名都由 Pillow 本地绘制，AI/模板底图不直接生成文字；系列标签框按字体 bbox 动态计算宽高，水平和垂直居中。

封面提示词：

```text
Generic literary listening-video cover for 布尔乔亚：在历史与文学之间_【意】弗朗哥·莫莱蒂 by 【意】弗朗哥·莫莱蒂: a dim ceremonial reading hall, stage-like composition, no AI-rendered text, local typography overlay, charcoal, antique gold, restrained editorial mood.
```

封面文字层：

```text
displayTitle: 布尔乔亚
displaySubtitle: 在历史与文学之间_弗朗哥·莫莱蒂
author: 【意】弗朗哥·莫莱蒂
seriesName: 半小时听完一本书
deck:
  - 从一本书进入一个时代，
  - 用三十分钟听见它的命运、欲望与回声。
footerLeft: A BOOK IN 30 MINUTES
```

8 张内容图使用 `formal_ai_literary_content_background_v1`，由 OpenAI 图像生成工具生成后复制到素材目录，尺寸统一为 `1920x1080`。设计思想是让图片直接服务于《布尔乔亚》的章节语义：实用、体面、工业道德、市场贫穷、易卜生式客厅裂缝、制度餐厅、现代效率焦虑和结尾档案室反思。所有图都要求无可读文字、无 logo、无字幕、无真实人物肖像，并预留较暗的下三分之一给硬字幕。

```text
Use case: literary listening-video background. Book: 布尔乔亚：在历史与文学之间_【意】弗朗哥·莫莱蒂. Author: 【意】弗朗哥·莫莱蒂. Visual node NN: <章节场景提示词>. Source text cue: <对应 aeneas 字幕段落摘要>
```

内容图节点：

| 序号 | 文件 | 时间范围 | 设计思想 |
| --- | --- | --- | --- |
| 1 | `01_island_study_tools_ledger.png` | `00:00:00,000 -> 00:03:12,480` | 荒岛工作台、工具、账本、种子罐和油灯，表现鲁滨孙式实用、劳动、库存和早期布尔乔亚秩序。 |
| 2 | `02_bourgeois_drawing_room_order.png` | `00:03:12,480 -> 00:06:25,960` | 十九世纪中产客厅、整齐家具、书本、窗帘和空椅子，表现舒适、观察、品味和体面秩序。 |
| 3 | `03_victorian_factory_office_morality.png` | `00:06:25,960 -> 00:09:48,120` | 维多利亚工业办公室、账本、经理椅和远处烟囱，表现工业权力、道德严肃和自我证明。 |
| 4 | `04_poor_room_market_bargain.png` | `00:09:48,120 -> 00:13:07,960` | 破旧出租屋、硬币、收据、锁箱和半开的市场走廊，表现贫穷外观、讨价还价和市场价值。 |
| 5 | `05_ibsen_living_room_crack.png` | `00:13:07,960 -> 00:16:21,520` | 斯堪的纳维亚客厅、关上的门、熄灭壁炉和地板裂缝，表现易卜生式体面谎言和家庭裂痕。 |
| 6 | `06_institutional_dining_room_self_interest.png` | `00:16:21,520 -> 00:19:33,120` | 空荡制度餐厅、长桌、酒杯、信封和黑暗门洞，表现责任话术、公益包装和隐蔽私利。 |
| 7 | `07_modern_office_productivity_anxiety.png` | `00:19:33,120 -> 00:23:03,920` | 深夜现代办公室、模糊图表、空桌和城市倒影，连接效率、自我提升和当代资本主义焦虑。 |
| 8 | `08_archive_dawn_reflection.png` | `00:23:03,920 -> 00:26:20,920` | 清晨档案室、旧书、沙漏、空白纸和窗外晨光，收束为对“有用、效率、舒适、体面”的反思。 |

最终成片验证：

- 硬字幕 manifest：`video\cover_timeline_formal_no_subtitle_20260621_2301\20260621_212132_布尔乔亚_formal_cover_timeline.aeneas.hardsub_manifest.json`。
- 参数：`dialogueCount=2038`，`delayMs=5000`，延迟 ASS 为短 ASCII 文件 `hardsub_delay5000ms.ass`，视频 H.264 `1920x1080`、`30fps`，音频 AAC `48000Hz` 双声道，总时长 `1586.0` 秒，SHA256 `f5662f0fdca31107348cc8d840f6db067408c4f9d2eaf2d948b5a2678db08542`。
- 验证帧：`hardsub_formal_verify_frames\cover_2s.jpg` 片头无字幕且封面不溢出；`subtitle_7s.jpg` 为荒岛工作台正式背景，`subtitle_10m.jpg` 为贫穷房间/市场正式背景，`subtitle_25m.jpg` 为档案室反思正式背景，三处均显示双语硬字幕且中文无乱码。

## 日志设计

日志体系由 `OperationLogger` 统一写入：

- SQLite 表：`operate_log`
- 字段：`id`、`created_at`、`level`、`module`、`action`、`message`、`detail`、`trace_id`
- 等级：`DEBUG`、`INFO`、`WARN`、`ERROR`
- 索引：创建时间、模块动作、`trace_id`

每次生成素材时，前端创建类似 `materials-YYYYMMDD-HHMMSS-random` 的任务 ID，作为 `trace_id` 传入后端。生成链路日志包括：

- `generate.start`
- `settings.snapshot`
- `source.file`
- `source.read`
- `source.read.done`
- `source.read.failed`
- `source.read.panic`
- `source.read.timeout`
- `prompt.build`
- `ai.request`
- `ai.response`
- `ai.parse`
- `ai.repair.*`
- `subtitle.split`
- `generate.done`
- `materials_notify.*`
- `export.*`

日志只记录 API Key 是否存在和长度，不记录明文。

日志页面设计：

- 默认读取本次生成任务的 `trace_id` 日志；没有当前任务时自动定位最近一次素材生成任务。
- 支持自动刷新、单行/多行选择、复制选择、清空显示。
- 清空只影响前端显示，不清空 SQLite 或文本 log 文件。
- 参考 IDEA 控制栏，提供 5 个横向按钮：上一条、下一条、软换行、滚动到底部、清空显示；不提供打印按钮。
- 支持搜索、大小写、全词、正则、上一处/下一处、只显示匹配项和搜索历史。

## 飞书通知设计

飞书配置位于“配置”页，包括 Webhook、消息标题和测试消息。

生成完成后，如果 Webhook 为空，则写入跳过日志；如果已配置，则发送文本通知。通知内容包括：

- 书名
- 视频标题
- 旁白中文字数
- 字幕行数
- 使用模型
- 总耗时
- 源文件路径
- 生成状态描述

飞书请求使用机器人 Webhook 的 `msg_type=text` 格式。返回非零 code 时视为失败并写入错误日志。

飞书通知中的耗时使用人类可读格式：大于等于 1 小时显示 `X时Y分Z.Z秒`，大于等于 1 分钟显示 `Y分Z.Z秒`，不足 1 分钟显示 `Z.Z秒`。例如 `208.0 秒` 会显示为 `3分28.0秒`。

## 打包与版本

每次代码或打包相关变更必须：

1. 补丁版本号递增 `0.0.1`。
2. 同步修改 `package.json`、`src/config/app.ts`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`。
3. 通过 `cargo update -p a_book_in_30_minutes` 同步 `Cargo.lock`。
4. 运行前端构建、Rust 检查和 Windows 打包。
5. 在日报中记录验证方式和安装包路径。

当前 Windows 安装包输出目录：

```text
a-book-in-30-minutes/src-tauri/target/x86_64-pc-windows-gnu/release/bundle/nsis
```

## 设计历史

2026-06-18 已完成并补录：

- 从 Tauri Framework 派生 `a-book-in-30-minutes`，定位为 YouTube 听书素材工作台。
- 确定频道名 `半小时听完一本书` 和英文名 `A Book in 30 Minutes`。
- 新增素材生成页：输入 EPUB/TXT，生成视频标题、简介、标签、旁白和字幕。
- 新增生成音频页：支持把旁白合成为 mp3，长文本分段，使用外部 ffmpeg 拼接。
- 新增文件/文件夹选择；扫描目录时不再过滤扩展名，解析阶段再判断支持情况。
- 新增全局 store 保存素材页状态，切换菜单不丢状态。
- 修正最大化后页面宽度，内容随窗口扩展。
- 新增 AI 配置测试成功后保存 Key；右上角显示模型名。
- 新增飞书配置和素材生成完成通知。
- 新增微软语音配置：Speech Key、区域、音色、输出格式、语速和音调。
- 微软语音配置增加 Azure Portal 和官方文档链接；区域改为中文+英文下拉；人声音色改为听书常用中文音色下拉。
- 新增工具路径配置：保存外部 `ffmpeg.exe` 路径，不随安装包打包。
- 新增 SQLite 操作日志和“操作日志”菜单。
- 日志页改为查看本次生成任务日志，而不是文本 log 文件。
- 日志页补齐 IDEA 风格控制栏、选择复制、清空显示、搜索和搜索历史。
- 日志链路增加 DEBUG/INFO/WARN/ERROR 详细记录。
- 修复 EPUB HTML 清洗中不兼容 Rust regex 的反向引用，避免解析正文阶段异常中断。
- `source.read` 增加 panic 和 30 秒超时兜底，内部异常或解析超时会进入任务日志和界面错误提示。
- AI 和飞书 HTTP 客户端增加明确超时，避免外部服务无响应时一直等待。

2026-06-20 已完成并补录：

- 素材目标字数调整为 `7000-8300`，微软语音默认语速调整为 `0%`，优先通过文本长度控制 30 到 35 分钟音频，而不是加速旁白。
- 素材阶段强化短字幕策略：每个半句尽量独立成一条字幕，最长不超过 18 个汉字，避免剪映字幕一行塞多个半句。
- 音频生成结果统一记录音频时长、分段数量、耗时和输出路径，并通过飞书通知。
- 生成字幕链路已验证可从最终 mp3 和 `subtitles.txt` 生成中文字幕、剪映 SRT、双语 SRT 和双语 ASS。
- 视频阶段设计从占位图/截图预览升级为高清原图资产流程：AI 原图必须复制到 `visual_assets/originals`，写入 `visual_assets_manifest.json` 和 SQLite `visual_assets`。
- 新增图片与字幕时间轴设计：每张图片必须绑定字幕起止编号和起止时间，写入 `visual_timeline.json` 和 SQLite `visual_timeline_segments`。
- 《我的文学奖》样例已生成 8 张内容相关 `ai_original` 图片，并完成 8 段图片-字幕时间轴映射。

## 已知限制与后续方向

- 2026-06-22 起，一键视频生成采用后台任务模式：前端点击“视频”后只负责把任务列表中对应记录刷新为“视频生成中”，并立即释放页面 busy 状态；长耗时的 Python 视频流水线由 Tauri 后台线程继续执行，实际进度和错误通过“操作日志”菜单查看。后台任务完成后回填 `material_tasks.status/progress/material_output_dir/message`，避免 WebView 长时间等待导致窗口未响应。
- 2026-06-22 起，流水线任务列表独立展示视频状态、视频时长和视频文件大小。任务表新增 `video_status`、`video_progress`、`video_file`、`video_duration_ms`、`video_file_size`、`video_message`，视频后台任务启动时写入生成中，完成后优先记录硬字幕 MP4 的路径、时长和大小。
- 2026-06-22 起，流水线任务列表不再使用横向滚动条。首列固定为复选框，不显示“源文件”等额外表头，也不渲染源文件大小等未定义列；任务名称列是唯一弹性列，窗口最大化时只调整任务名称宽度；任务名称显示最多 20 个字符，超过后显示前 18 个字符加 `...`，悬停显示完整文件名；任务列内容居左，其它列内容居中；格式、状态、进度、字数、音频、视频、时长和大小列使用固定宽度。
- 2026-06-23 起，素材、音频、字幕、封面、视觉图、无字幕母版和最终视频等产出物直接归档到源书所在目录下的 `output` 文件夹根目录，不再为单个任务新建时间戳子文件夹，也不再固定创建 `audio`、`video`、`subtitles` 子目录。任务表中的 `material_output_dir` 和新回填的 `audio_output_dir` 指向这个 `output` 根目录；历史子目录只作为兼容读取来源，懒迁移时也复制到 `output` 根目录。
- 2026-06-22 起，流水线页【视频】按钮是完整一键视频入口：优先使用勾选任务，其次当前任务或路径输入；如果缺少素材会先补生成素材，如果缺少音频会再补生成音频，最后启动视频后台任务。按钮可用状态按勾选、当前任务和输入路径综合判断，不再只依赖顶部素材路径输入框。
- 2026-06-22 起，操作日志菜单默认只显示本次 app 启动后的日志；点击生成按钮后，如果前端有当前 `trace_id`，日志页只显示该按钮触发任务的日志，不再回退显示历史“最近一次生成任务”。
- DOCX 和 PDF 目前只识别格式，正文解析暂未接入。
- 素材包外部脚本已验证字幕和视觉时间轴，但应用内【视频】阶段尚未产品化，需要把临时脚本迁移为 Tauri 后端命令和前端入口。
- AI 图片生成、原图复制、SQLite 入库、`visual_timeline` 生成目前仍依赖人工/Codex 脚本串联，后续要固化为可重复流水线。
- 剪映草稿生成已验证基础结构，但正式导出、背景慢推拉、BGM 选择、封面图和发布辅助尚未完整接入应用。
- 更新检查目前是 mock，占位显示当前版本；后续如果接入真实更新器，需要同步更新元数据生成和发布流程。
- EPUB 目录解析目前优先 NCX，部分 EPUB3 nav 文档可能需要后续增强。


## 2026-06-23 视频流水线与产物目录更新

- 流水线任务列表拆分为素材进度、音频进度、视频进度；素材阶段达到 100% 时任务状态写入 `success`，界面显示已完成和 100%。
- 顶部 `素材`、`音频`、`视频` 三个动作按钮统一为透明默认态；当前执行阶段才使用绿色高亮，素材按钮使用书籍图标。
- EPUB 所在目录下统一使用 `output` 作为产物根目录。文本、标签、字幕、`materials.json`、旁白音频、aeneas SRT/ASS、封面、视觉图、无字幕母版和最终视频都直接保存在 `output` 根目录。
- Python 视频流水线即使单独运行且未显式传入 `--output-dir`，也必须默认使用 `epub.parent/output`，不得回退创建 `<书名>_video_时间戳` 子目录。
- 点击 `视频` 是一键入口：没有素材先生成素材，没有音频先生成音频，最后启动后台视频流水线；前端只刷新任务列表状态，实际过程到操作日志查看。
- 视频流水线不再写入固定 5 秒占位视频。脚本会读取素材包内最新音频，根据音频时长生成 MP4，并输出 `pipeline_manifest.json`。
- 后端在视频任务成功落库前使用 `ffprobe` 读取音频和视频时长；音频超过 60 秒时，如果视频时长缺失或与音频差异超过 5 秒，任务标记为失败，不允许显示已完成。
- 背景循环音乐从 exe 同级 `bg` 文件夹查找第一个 `.mp3` 文件，作为 `--background-music` 传入视频流水线；打包或绿色版部署时需把背景音乐复制到 `exe所在目录/bg/`。旧 `music` 目录只作为兼容回退，不能作为新版本主路径。
- 当前版本升级为 `0.1.69`。
## 2026-06-23 两段式视频生成流程

- 视频生成必须保留两段式流程：第一段先生成 `<书名>_无字幕母版.mp4`，内容包含 5 秒封面、阅读背景图、旁白音频和循环背景音乐；第二段再以该无字幕母版为输入，烧录 ASS 硬字幕生成最终 `<书名>_中英双语字幕_精修版.mp4`，命名参考 `WWDC26_Keynotes_中英双语字幕_精修版.mp4`。
- 字幕时间轴以旁白音频为主体，但整体向后偏移 `coverSeconds`，默认 5 秒，避免封面阶段提前出现正文字幕。
- `visual_timeline.json` 必须同时记录封面片段和背景阅读片段，封面片段从 0 到 `coverSeconds`，背景片段从 `coverSeconds` 到最终视频结束。
- `pipeline_manifest.json` 至少包含 `cover`、`background`、`visualTimeline`、`noSubtitleVideo`、`hardSubtitleVideo`、`hardSubtitleManifest`、`hardSubtitleSrt`、`narrationAudioForVideo`、`noSubtitleVideoDurationMs`、`videoDurationMs` 和 `coverSeconds`，供后端更新任务列表和日志追踪。
- 后端完成校验优先比较 `noSubtitleVideo` 与 `hardSubtitleVideo` 的时长是否一致；当生成了硬字幕最终版时，不再用任务旧音频时长作为唯一基准，因为流程可能会把短音频拉伸到 30 分钟并额外增加封面片段。

## 2026-06-23 视频封面模板

- 视频封面就是最终视频开头 5 秒的 16:9 封面画面，必须按用户确认的模板生成，不作为单独的静态海报方案分叉。
- 固定模板元素包括：左上“半小时听完一本书”栏目牌、右上 `VOL. 01` 标签、右侧竖向金线、右侧主书名、作者行、底部两行简介、底部横线、左下 `A BOOK IN 30 MINUTES` 和右下“睡前听书系列”。
- 不同书籍只替换动态字段：书名从视频标题中的 `《...》` 或 EPUB 文件名抽取；作者优先读素材或 EPUB 元数据；分类文案优先从素材标签生成；简介优先使用视频简介第一段。
- 书名必须自适应中文长度和换行，避免标点出现在第二行开头；以中文标点换行时封面主标题隐藏该分隔标点，让画面更接近模板示例。
- 封面渲染由视频流水线 `render_cover_image` 统一负责，使用 Pillow 绘制圆角栏目牌、文字和装饰线，避免 ffmpeg 文字裁切或溢出；后续 app 内一键视频、脚本验证和打包版本必须共用这一套模板，避免出现绿色占位封面或纯色测试封面。
- 左上栏目牌必须是圆角矩形；右上标签统一使用三位数字格式 `VOL.XXX`，例如 `VOL.001`，标签宽度要能完整容纳文字。
- 左下描述只保留两行短文案，不能直接填入冗长视频简介，也不能出现 `...` 省略号；优先生成“从……到……”式短句和“三十五分钟，听完《书名》。”这类模板句。

## 2026-06-23 双语硬字幕样式

- 最终硬字幕必须支持真实中英双语字幕，而不是只有中文字幕套双语样式。素材包内存在 `subtitles_en.json` 时，视频流水线必须优先把 `subtitles.txt` 与 `subtitles_en.json` 一一配对生成双语字幕事件。
- `subtitles_en.json` 与 `subtitles.txt` 的条数必须一致；生成 ASS 后应能统计到相同数量的 `Chinese` 与 `English` Dialogue。
- 发布级硬字幕禁止使用按文本长度或总时长估算出来的字幕时间轴。应用内视频脚本必须优先复用素材包 `subtitles/aeneas_*/*.aeneas.zh-en.ass`；没有可复用 ASS 时，必须调用 `aeneas.tools` 使用最终旁白音频和 `subtitles.txt` 生成中文 cue，再按相同 cue 写入英文字幕。若 aeneas 环境或英文字幕缓存缺失，视频生成应明确失败并写日志，不能静默退回估算时间轴。
- ASS 样式参考 `D:\04_GitHub\yt-download` 的 Apple 双语字幕：中文使用米白大字、加粗、黑描边和阴影，英文使用橙色较小字号、加粗、黑描边和阴影，英文位于中文下方。
- 当前 1920x1080 视频使用 `Chinese` 和 `English` 两个独立 Style；中文底边距高于英文，避免两行重叠。只有中文行时仍使用 `Chinese` 样式；有英文行时必须输出成对 Dialogue。
- 生成最终视频后必须抽帧验证：封面帧、无字幕母版背景帧、硬字幕双语帧；同时使用 `ffprobe` 校验无字幕母版和硬字幕最终版时长一致。
