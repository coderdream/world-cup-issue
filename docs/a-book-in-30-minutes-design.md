# A Book in 30 Minutes 详细设计文档

## 文档维护规则

- 本文档是 `a-book-in-30-minutes` 的长期设计基线。每次修改功能、流程、界面、数据结构、提示词、日志、通知或打包配置时，都必须同步更新本文档。
- 日报仍写入 `docs/daily/YYYY-MM-DD.md`，本文档负责沉淀稳定设计，避免只靠聊天记录传递背景。
- 所有新增中文文案、日志、配置和文档必须保持 UTF-8 正常显示，不得出现乱码。

## 产品定位

`A Book in 30 Minutes` 是一个 Tauri 桌面工具，用于把小说或书籍源文件转换成 YouTube 听书视频素材。核心工作台命名为“流水线”，按素材、音频、视频三个阶段逐步处理。当前阶段已覆盖“文本素材生成”和“旁白音频生成”，视频阶段的设计基线已明确为“字幕时间轴 + 统一视觉资产 + 剪映草稿/ffmpeg/白板动画渲染”。视觉资产可以走电影感 AI 原图路线，也可以走程序化白板解释插画路线，但都必须和字幕时间轴绑定，后续实现必须沿该链路落地。

- 中文频道名：`半小时听完一本书`
- 英文产品名：`A Book in 30 Minutes`
- 应用目录：`a-book-in-30-minutes`
- Tauri 标识：`com.abookin30minutes.desktop`
- Rust crate：`a_book_in_30_minutes`
- 当前版本：`0.1.151`

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
17. 视频阶段必须先生成或导入与旁白内容相关的正式视觉资产，电影感路线保存高清原图到素材包 `visual_assets/originals` 并写入 SQLite `visual_assets`；程序化白板插画路线保存 4K PNG、`scene_plan.json` 和 `image_timeline.json`。禁止只使用截图、缩略图、视频帧或经过模糊处理的派生图作为最终视频背景源。
18. 生成图片时必须记录它对应的字幕/旁白片段，形成 `visual_timeline` 或 `image_timeline.json`：每张图绑定 `start_subtitle_index`、`end_subtitle_index`、`start_time`、`end_time` 和对应文本摘要。视频生成时按该时间轴铺设图片片段或白板动画片段，而不是平均切图。

## 前端结构

前端使用 React、TypeScript、Zustand、lucide-react 和自定义 CSS。主要页面如下：

- `流水线`：任务列表优先的工作台。顶部是流水线分析入口和 6 个阶段按钮：`文本`、`图片`、`音频`、`字幕`、`视频`、`发布`。按钮位于“流水线分析面板”标题行右侧，不占用素材路径输入行。其中 `文本` 运行素材生成，`图片` 启动仅图片素材后台流水线，`音频` 执行批量 TTS，`字幕` 启动仅音频字幕后台流水线，`视频` 启动完整视频后台流水线，`发布` 生成 YouTube 发布资料。主体是流水线任务列表，最近生成结果在任务列表下方展示。
- `生成音频`：把素材旁白或手动文本合成为 mp3，展示输出目录、最终音频、SSML、分段数量和耗时。
- `操作日志`：以 IDEA 控制台风格展示后台日志，默认查看本次或最近一次生成任务日志。
- `配置`：管理素材生成默认参数、AI 模型、API Key、Base URL、每个 AI 独立代理、飞书 Webhook、微软语音、外部工具路径、基础开关和更新检查入口。
- `关于`：展示应用版本和基础信息。

`materialsWorkbench` 状态保存在 Zustand 全局 store 中，包含请求参数、扫描结果、生成结果、导出目录、错误提示、复制状态、当前结果标签页、当前 `trace_id` 和忙碌状态。切换菜单后不丢失素材页状态。

流水线主顺序为 `文本 -> 音频 -> 字幕 -> 图片 -> 视频 -> 发布`。`文本` 生成 `narration.txt` 和 `subtitles.txt`；`音频` 根据 `subtitles.txt` 生成最终 mp3；`字幕` 根据 mp3 和字幕文本对齐生成中文字幕 SRT、双语 SRT 和 ASS；`图片` 必须基于已对齐的中文字幕 SRT 时间戳和文本分段生成，并在图片清单/时间轴里记录每张图片的开始时间、结束时间、覆盖字幕文本和图片路径；`视频` 只消费音频、字幕和图片时间轴合成成片；`发布` 最后生成 YouTube 发布资料。

字幕阶段选择旁白源音频时必须优先读取素材目录 `audio_manifest.json.finalAudioFile`，例如 `书名.epub.mp3`，并排除 `part_*.mp3`、`narration_for_video.mp3`、`*_video_mix.mp3`、硬字幕派生音频和其它视频混音产物。`*_video_mix.mp3` 是视频/字幕准备阶段追加片头后的派生音频，只能作为输出或后续消费文件，不能再次作为源旁白输入；若输入输出路径意外相同，音频拼接必须先写入临时文件再替换，禁止触发 ffmpeg 原地覆盖失败。

流水线的 `音频`、`字幕`、`图片`、`视频` 属于后台任务阶段。点击其中任一按钮后，前端保留当前 `trace_id` 和高亮阶段，6 个阶段按钮全部禁用，直到用户点击【终止任务】解除锁定；这样可以避免同一个任务在后台运行时误点其它阶段。终止区左侧显示当前错误、复制或执行状态日志，右侧显示【终止任务】按钮，两者顶部对齐，日志允许多行换行。任务列表轮询 SQLite 时，正在生成的图片、字幕、音频或视频阶段不能因为产物文件暂未出现而被恢复为待处理。

步骤跟踪中的发布阶段不能复用视频阶段的 `generating` 状态。发布只有在真实发布资料步骤执行时才显示进行中；视频已生成但发布资料未生成时显示等待生成发布资料，避免用户看到视频和发布两个阶段同时运行。

图片阶段的正式内容图数量必须按字幕规模动态生成，范围固定为 32~64 张。目标数量按 `字幕行数 / 28` 向上取整后夹在该范围内；例如 1000 多行字幕通常生成约 36~40 张图片。禁止沿用早期 8 段验证分镜作为正式听书视频图片数量，因为 30~35 分钟视频中 8 张图会导致单张停留数分钟，视觉变化不足。若迁移到的本地旧图片素材少于 32 张，且允许程序化视觉生成，必须重新生成满足数量范围的图片。配置页 `pipelineProfile.imageBackend` 控制图片生成方案，默认正式后端为 `BOOK_IMAGE_BACKEND=xiaohei-production`：本机按字幕区间生成 JSON spec，通过 `ssh macmini4` 调用 MacMini4 `/Volumes/System/AI/apps/xiaohei-local-generator/xiaohei_local_generate.py` 输出 3200x1800 生产图，再 `scp` 拉回并缩放为 1920x1080 视频图。旧 `xiaohei-sequence` 快速本机方案保留为可回退选项；`xiaohei-ai-y9000p` 调用本机 187 / Y9000P 的 ComfyUI 节点；`qwen-image-2512` 和 `whiteboard-skill` 仅作为显式实验/兼容选项。禁止用低保真方框图、线框图或占位图冒充正式图片；但有明确视觉规范、时间轴和 manifest 的小黑序列图是正式后端。

图片阶段不得早于字幕阶段执行。若缺少最终 mp3、中文字幕 SRT 或字幕对齐清单，图片阶段必须失败并提示先生成音频和字幕。图片分段以中文字幕 SRT 为准，而不是用 `subtitles.txt` 和估算时长临时切分。`visual_assets_manifest.json` 和 `visual_timeline.json` 都必须能追溯每张图覆盖的 `startMs`、`endMs`、字幕文本范围和源图片文件，视频阶段以该时间轴控制图片显示开始和结束。

图片阶段后端读取素材目录时必须使用 SQLite `material_tasks.material_output_dir`，不能使用固定字符串或错误 SQL；否则已生成的 `hard_subtitle.aeneas.cmn.srt` / `hard_subtitle.aeneas.chn.srt` 会被误判缺失。前置校验失败时，后端必须同时把 `image_status` 收口为 `failed`，并将仍处于 `generating` 的视频状态收口为失败，避免步骤跟踪同时显示图片和视频进行中。

图片生成仍保留可选远程 Qwen Image 后端。只有显式设置 `BOOK_IMAGE_BACKEND=qwen-image-2512` 时，脚本才读取 `QWEN_IMAGE_BASE_URL=http://100.96.199.26:8188`、`QWEN_IMAGE_WIDTH`、`QWEN_IMAGE_HEIGHT`、`QWEN_IMAGE_STEPS`、`QWEN_IMAGE_REQUEST_TIMEOUT_SECONDS`、`QWEN_IMAGE_MAX_WAIT_SECONDS` 和 `QWEN_IMAGE_POLL_SECONDS`。`100.96.199.26` 是 MacMini4 的 Tailscale IP，当前在家里或不在同一局域网时必须使用该地址，不要改用公司/局域网 NAS 地址。脚本通过 ComfyUI `POST /prompt`、`GET /history/{prompt_id}` 和 `/view` 生成并下载图片，输出到 `qwen_image_2512` 目录，同时写入 `qwen_image_2512_manifest.json` 和 `visual_assets_manifest.json`。Qwen 生成单张图可能耗时数分钟，HTTP socket timeout 必须长于生成时间；脚本必须在 stderr 输出每张图的 queued、waiting 和 generated 进度，避免操作日志看起来像卡住。若 Qwen 服务不可用、超时或输出低质量图片，脚本必须在 stderr 写明原因并回退到现有 whiteboard image skill，保证流水线有明确日志且不中断。

图片生成新增本机 187 / Y9000P ComfyUI 后端。只有显式设置 `BOOK_IMAGE_BACKEND=xiaohei-ai-y9000p` 时，脚本才读取 `Y9000P_COMFYUI_BASE_URL=http://127.0.0.1:8188`、`Y9000P_COMFYUI_WORKFLOW`、`Y9000P_COMFYUI_CHECKPOINT`、`Y9000P_COMFYUI_INPUT_DIR`、`Y9000P_COMFYUI_WIDTH`、`Y9000P_COMFYUI_HEIGHT`、`Y9000P_COMFYUI_STEPS`、`Y9000P_COMFYUI_CFG`、`Y9000P_COMFYUI_DENOISE`、`Y9000P_COMFYUI_RESTORE_GUIDE_LINE_ART`、`Y9000P_COMFYUI_GUIDE_CLEANUP_RADIUS`、`Y9000P_COMFYUI_BACKGROUND_WHITE_THRESHOLD`、`Y9000P_COMFYUI_REQUEST_TIMEOUT_SECONDS`、`Y9000P_COMFYUI_MAX_WAIT_SECONDS` 和 `Y9000P_COMFYUI_POLL_SECONDS`。该后端默认使用官方风格受控 img2img：先用本机程序化小黑分镜生成 32~64 张有中文标注的 `xiaohei_ai_y9000p_guides/guide_XX.png`，画法对齐官方样例的白底、黑色小人、稀疏概念隐喻、红蓝橙箭头和短中文标注；中文标注优先用 Windows 楷体 `simkai.ttf`，也可通过 `XIAOHEI_KAITI_FONT` 指定字体。guide 生成必须避免生硬方框和大量直线，标签默认用楷体文字加手绘波浪下划线，箭头和连接线使用弯曲手绘线，整体保持随手画、流畅线条的官方小黑风格；同时生成无中文标注的 `guide_XX_no_text.png` 并缩放复制到 `D:\AI\apps\ComfyUI\input\xiaohei_y9000p_guides`，通过 `LoadImage -> VAEEncode -> KSampler -> VAEDecode -> SaveImage` 调用本机 ComfyUI 精修。默认 checkpoint 为 `DreamShaper8_LCM.safetensors`，默认 `sampler=lcm`、`scheduler=sgm_uniform`、`width=1536`、`height=864`、`steps=32`、`cfg=1.9`、`denoise=0.38`，在 RTX 3070 Laptop GPU 上单张约 30 秒，作为质量优先档；显式设置 `Y9000P_COMFYUI_WORKFLOW=txt2img` 时才回到旧的自由生图 smoke 路线。由于扩散模型会破坏中文标注，模型输入刻意不包含中文，最终视频图默认开启 `Y9000P_COMFYUI_RESTORE_GUIDE_LINE_ART=1`，并只覆盖有中文 guide 与无中文 guide 的差异文字层，不再覆盖整张线稿，从而保留 AI 对无文字线稿的增强；默认 `Y9000P_COMFYUI_GUIDE_CLEANUP_RADIUS=5` 用于覆盖前轻微擦除文字区域，默认 `Y9000P_COMFYUI_BACKGROUND_WHITE_THRESHOLD=185` 用于把近白背景压回官方白底。输出写入 `xiaohei_ai_y9000p` 目录，同时写入 `xiaohei_ai_y9000p_manifest.json` 和 `visual_assets_manifest.json`，manifest 记录 guide、modelGuide、ComfyUI raw 输出、最终视频图、prompt、工作流参数和每张图的分镜元数据。若本机 ComfyUI 不可用或生成失败，流水线会在 stderr 写明原因并回退到 `xiaohei-sequence`，避免图片阶段中断。

素材生成默认参数保存在 `settings.materialProfile`，包括 `channelName`、`categoryName`、`categories`、`language`、`targetMinChars`、`targetMaxChars` 和 `extraDirection`。默认目标为 `7000-8300` 个中文字，最佳约 `7600` 字，用于配合 `0%` 原速语音生成约 `30-35` 分钟睡前听书音频，并尽量避免最终音频超过 `35:00`；如果用户调整目标时长，应优先调整这两个字数配置，而不是为了压缩时长提高语速。`categories` 默认包含 `半小时听完一本书`、`睡前听完一本书`、`A Book in 30 Minutes`，配置页允许新增分类；`categoryName` 是当前任务入库分类，等价于后续 YouTube 播放列表名称；`channelName` 为兼容旧生成提示词保留，当前选择分类时会同步更新。素材生成页不再直接编辑这些参数，生成请求会把当前配置合并进请求体。文件级生成状态同时保存在 `materialsWorkbench.fileStatuses` 和 SQLite `material_tasks`，按文件路径记录状态、五档进度、成稿字数和失败信息。

界面字体配置保存在 `settings.uiProfile`，包括 `menuFontFamily`、`menuFontSize`、`contentFontFamily` 和 `contentFontSize`。配置页“基础配置”允许分别设置左侧菜单字体和页面内容字体；默认菜单字号为 `13px`，内容字号为 `12px`。前端通过 CSS 变量 `--menu-font-family`、`--menu-font-size`、`--content-font-family` 和 `--content-font-size` 应用配置，页面表格、配置项和步骤跟踪内容默认跟随内容字体。

流水线跳过策略保存在 `settings.pipelineProfile`，包括 `skipExistingText`、`skipExistingImages`、`skipExistingAudio`、`skipExistingSubtitles`、`skipExistingVideo` 和 `skipExistingPublish`。六项默认均为 `true`，配置页显示为“已有则跳过：是”；选择“否，每次重新生成”时，对应阶段不再因为已有产物而跳过。`skipExistingMaterials` 作为旧版兼容字段保留，并与 `skipExistingText` 同步。任务列表列名使用 `素材进度`、`音频进度`、`视频进度`，状态列只显示阶段状态，进度列单独显示百分比。

流水线任务列表必须优先保证任务名称可读。列间距控制在 `2-4px`，当前实现为 `2px`；固定列使用窄列宽，任务列使用弹性宽度并设置最小宽度。视频状态不能只看是否存在视频文件：如果视频标记为成功，但视频时长明显短于音频时长，列表显示“异常”，视频进度显示 `-`，避免出现音频和视频都显示“已完成 / 100%”但实际产物明显不一致的误导。

`audioWorkbench` 状态保存在 Zustand 全局 store 中，包含旁白文本、输出目录、文件名、当前 `trace_id`、忙碌状态、错误提示和生成结果摘要。切换菜单后不丢失音频页状态。

右上角显示当前选中 AI 的模型名，格式为 `【模型名】`。AI 测试连接前会先保存当前输入，测试成功后再次写入 SQLite 中的 `app_settings.settings`，确保 API Key 和代理配置被保存。

AI 模型配置保存在 `settings.activeAiProvider`、`settings.aiProfile` 和 `settings.geminiProfile`。配置页顶部“流水线使用 AI”下拉框明确控制素材流水线、测试连接和 AI 文本测试实际使用 GPT 还是 Gemini；GPT/Gemini 分段控件用于编辑对应 provider 的详细配置。GPT 默认 provider 为 `openai_compatible`，默认不启用代理；Gemini 默认 provider 为 `gemini`，Base URL 为 `https://generativelanguage.googleapis.com/v1beta`，模型为 `gemini-flash-latest`，默认启用代理 `http://127.0.0.1:1080`。每个 AI profile 都独立保存 `name`、`baseURL`、`model`、`apiKey`、`proxyEnabled` 和 `proxyUrl`，后端只按当前选中 AI 的 profile 决定是否走代理，避免 Gemini 的 VPN 要求影响 GPT。

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
- `get_material_task_steps`：按当前 `traceId` 或源文件路径读取 SQLite 中持久化的步骤记录，返回每步状态、进度、开始时间、完成时间和耗时。
- `get_operation_logs`：读取 SQLite 操作日志；传入主 `traceId` 时同时返回该 trace 和以 `主trace-` 开头的子阶段日志，保证一键视频的补素材、补音频和视频生成日志能在同一个任务视图中显示。
- `generate_book_video_pipeline`：启动图片、字幕或视频后台流水线。图片阶段日志必须写清楚阶段、EPUB、输出目录、脚本、Python、素材目录、图片目录、图片数量、封面和耗时；结果 JSON 不作为普通日志整段刷屏，而是提取为中文摘要，失败时仍保留 stdout/stderr 预览用于排障。

### 步骤跟踪页

流水线页顶部的 6 个操作按钮必须在任务列表中有对应列：文本、图片、音频、字幕、视频、发布。任务列表允许横向滚动，列内显示每个阶段的状态、百分比和关键产物信息；当用户点击【视频】这类后台任务后，列表中的视频列必须能直接看到当前阶段正在生成和后台消息对应的百分比，不能只提示用户去操作日志查看。顶部也可以保留阶段概览卡，但任务列表列展示是硬性要求。

左侧导航中的“生成音频”替换为“步骤跟踪”。步骤跟踪页参考 `video-easy-creator` 的步骤统计与步骤表结构；结构化步骤数据持久化在 SQLite 的 `material_task_steps` 表中，页面通过 `get_material_task_steps` 按当前 `traceId` 或源文件路径读取。页面顶部展示当前任务、总步骤、步骤进度、任务摘要、任务 ID 和整体进度；下方按产物链拆成细步骤行，展示步骤编码、步骤名称、状态、百分比、耗时和说明。若当前任务没有步骤表记录，前端才回退到 `material_tasks` 的阶段状态推导，避免旧任务完全空白。

当前步骤跟踪页拆分为 17 个步骤：A-01 解析书籍、A-02 标题简介标签、A-03 旁白文稿、A-04 保存素材包、B-01 读取旁白、B-02 拆分片段、B-03 生成语音、B-04 合成音频、C-01 生成中文字幕、C-02 生成双语字幕、D-01 生成封面、D-02 生成分镜图、E-01 准备流水线、E-02 生成无字幕母版、E-03 生成硬字幕版、E-04 登记视频产物、F-01 生成发布资料。步骤编码必须稳定显示，禁止把相邻产物阶段合并成一个步骤。

材料生成阶段已接入持久化步骤记录：`generate_book_materials` 在读取 EPUB 时写入 A-01 进行中，`source.read.done` 时把 A-01 标记为成功；构建 prompt 和 `ai.request` 后写入 A-02 进行中，AI JSON 解析成功后把 A-02 标记为成功；旁白长度检查、修复和字幕切分归入 A-03；素材包写入归入 A-04。每条步骤记录包含 `started_at`、`finished_at` 和 `elapsed_ms`，步骤页的【耗时】列优先显示已落库耗时，运行中的步骤按 `started_at` 到当前时间实时刷新。AI 已经进入 `ai.request` 时，A-01 必须已经完成，不能继续显示“待处理”。

步骤耗时统一显示为 `MM分SS.SSS秒`。新写入的 `material_task_steps.started_at` 和 `finished_at` 使用毫秒精度，后端计算 `elapsed_ms` 时兼容旧的秒级时间戳；前端如果遇到旧记录 `elapsed_ms=0`，会优先用开始/结束时间重新计算，仍无法判断时才显示 `00分00.000秒`。步骤页选择跟踪任务时，必须把 `image_status`、`subtitle_status` 的 `generating` 状态纳入优先级，确保点击【图片】后 D-01 立即显示“进行中”。

素材阶段请求 AI 超时或失败时，失败会落在素材子步骤上，并在说明中显示后端写入的错误消息，例如 `HTTP 524`；这样用户可以从步骤跟踪页直接判断是 AI 素材 JSON 生成失败，而不是前端卡死。音频任务继续按读取旁白、拆分片段、生成语音片段和合成最终音频显示；视频任务按准备流水线、封面、图片、字幕、无字幕视频、硬字幕视频和登记产物显示。流水线任务列表中的音频与视频列同步显示“状态 + 百分比”。

后端应用状态 `AppData` 持有 `settings`、`settings_path`、`db_path` 和 `OperationLogger`。配置加载对嵌套配置块使用缺省兼容，旧版 `settings.json` 缺少新版字段时仍会保留已有 AI API Key、Base URL、模型、微软语音和工具路径；启动日志只记录配置路径、文件是否存在、Key 是否存在和脱敏长度，不输出密钥内容。

后端在 `generate_book_materials` 中通过 Tauri 事件 `material-task-progress` 推送素材任务进度，事件字段包括 `traceId`、`path`、`status`、`progress`、`step`、`totalSteps` 和 `message`。前端按当前 `traceId` 和文件路径匹配任务行，实时刷新任务列表并同步 SQLite 状态；最终生成结果返回后再写入“已完成 / 100% / 成稿字数”。

AI 请求层统一使用最多 3 次退避重试。HTTP 403、429 和 5xx 视为网关或服务端临时失败，分别等待 20 秒、40 秒后重试。GPT/OpenAI-compatible 分支请求 `{baseURL}/chat/completions`，使用 Bearer Auth 并支持 SSE/JSON 解析；Gemini 分支请求 `{baseURL}/models/{model}:generateContent`，使用 `X-goog-api-key` header，正文采用 Google `contents[].parts[].text` 结构，并解析 `candidates[].content.parts[].text`。兼容性验证发现当前 GPT 网关会拒绝 `max_tokens` 和 `max_completion_tokens`，因此不得在本项目 GPT 请求体中携带这两个字段。长旁白目标通过“AI 初稿 + 多轮小段追加 + 本地源书解读补足”实现：AI 初稿不可用时，后端会直接基于源书代表章节生成本地素材初稿；AI 初稿可用但连续追加被网关拦截或仍不足时，后端会从源书代表章节抽取片段，生成原创解读型补充旁白，并裁剪到 `targetMinChars-targetMaxChars` 范围内。

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

SQLite 额外保存任务步骤跟踪记录：

```text
material_task_steps(
  trace_id TEXT NOT NULL,
  path TEXT NOT NULL,
  step_code TEXT NOT NULL,
  step_name TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  progress INTEGER NOT NULL DEFAULT 0,
  detail TEXT NOT NULL DEFAULT '',
  started_at TEXT,
  finished_at TEXT,
  elapsed_ms INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (trace_id, step_code)
)
```

该表用于步骤跟踪页，不替代 `operate_log`。`operate_log` 负责完整调试日志；`material_task_steps` 负责每个稳定步骤的结构化状态、百分比和耗时。前端按当前 `traceId` 优先查询；没有 trace 时按源文件路径取最近一次任务的步骤记录。运行中的步骤在后端写 `started_at`，完成或失败时写 `finished_at` 与 `elapsed_ms`，应用重启后仍能恢复已完成步骤耗时。

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
- 音频生成阶段也必须保持扁平化：完整旁白 mp3、分段 `part_*.mp3`、分段 `part_*.ssml`、`narration.ssml` 和 `audio_manifest.json` 均直接写入 `output` 根目录，不再创建时间戳音频目录、`parts` 子目录或 `ssml` 子目录。
- 视频流水线选择旁白音频时必须优先使用 `output` 根目录下的 `*.mp3`/`*.wav`，只有根目录没有音频时才兼容读取历史 `audio/**/*.mp3`/`audio/**/*.wav`，避免新流程误用旧子目录产物。
- Python 视频流水线即使单独运行且未显式传入 `--output-dir`，也必须默认使用 `epub.parent/output`，不得回退创建 `<书名>_video_时间戳` 子目录。
- 点击 `视频` 是一键入口：没有素材先生成素材，没有音频先生成音频，最后启动后台视频流水线；前端只刷新任务列表状态，实际过程到操作日志查看。
- 视频流水线不再写入固定 5 秒占位视频。脚本会读取素材包内最新音频，根据音频时长生成 MP4，并输出 `pipeline_manifest.json`。
- 后端在视频任务成功落库前使用 `ffprobe` 读取音频和视频时长；音频超过 60 秒时，如果视频时长缺失或与音频差异超过 5 秒，任务标记为失败，不允许显示已完成。
- 背景循环音乐从 exe 同级 `bg` 文件夹查找第一个 `.mp3` 文件，作为 `--background-music` 传入视频流水线；打包或绿色版部署时需把背景音乐复制到 `exe所在目录/bg/`。旧 `music` 目录只作为兼容回退，不能作为新版本主路径。
- 当前版本升级为 `0.1.69`。
## 2026-06-23 两段式视频生成流程

- 视频生成必须保留两段式流程：第一段先生成 `<书名>_无字幕母版.mp4`，内容包含 3 秒静态封面、阅读背景图、已前置 `header.mp3` 的旁白音频和循环背景音乐；第二段再以该无字幕母版为输入，烧录 ASS 硬字幕生成最终 `<书名>_中英双语字幕_精修版.mp4`，命名参考 `WWDC26_Keynotes_中英双语字幕_精修版.mp4`。
- 字幕时间轴以已前置 `header.mp3` 的最终旁白音频为准，不再额外整体后移；aeneas 会自然把首条正文字幕对齐到 3 秒静音之后。
- `visual_timeline.json` 必须同时记录封面片段和背景阅读片段，封面片段从 0 到 `coverSeconds`，背景片段从 `coverSeconds` 到最终视频结束。
- `pipeline_manifest.json` 至少包含 `cover`、`background`、`visualTimeline`、`noSubtitleVideo`、`hardSubtitleVideo`、`hardSubtitleManifest`、`hardSubtitleSrt`、`narrationAudioForVideo`、`noSubtitleVideoDurationMs`、`videoDurationMs` 和 `coverSeconds`，供后端更新任务列表和日志追踪。
- 后端完成校验优先比较 `noSubtitleVideo` 与 `hardSubtitleVideo` 的时长是否一致；当生成了硬字幕最终版时，不再用任务旧音频时长作为唯一基准，因为流程可能会把短音频拉伸到 30 分钟并额外增加封面片段。

## 2026-06-23 视频封面模板

- 视频封面就是最终视频开头 3 秒的 16:9 封面画面，必须按用户确认的模板生成，不作为单独的静态海报方案分叉。
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
## 2026-06-24 视频管线修正规则

- 视频片头封面段必须保持静态渲染，不使用 `zoompan` 或任何逐帧缩放表达式，避免前 3 秒封面文字和矩形元素出现抖动。
- 2026-07-02 减抖更新：ffmpeg 直出视频默认对封面和内容图都使用稳定静帧，不再默认启用 `zoompan` 缓慢缩放/平移，也移除逐帧随机噪声滤镜，避免小数坐标取整和时间噪声导致文字、线条、边框出现像素级抖动。需要保留电影感推拉镜头时，可在命令环境中显式设置 `ABOOK_CINEMATIC_MOTION=1`，此时才启用旧的运动配置。
- 2026-07-02 发布记录：版本号同步递增到 `0.1.122`，包含上述视频减抖优化。本次按用户要求只发布代码版本，不生成新的 Windows 安装包，因此不会新增安装包产物或更新安装包下载资产。
- 视频流水线必须使用固定 `header.mp3` 作为 3 秒无声片头音频。开发环境固定路径为 `a-book-in-30-minutes/tmp/assets/header.mp3`；打包或绿色版运行时也必须在 exe 同级或资源目录保留一份。脚本缺失该文件时可用 ffmpeg 生成同规格无声音频。
- 视频生成前必须把 `header.mp3` 前置拼接到旁白音频，输出 `书名.mp3` 作为最终视频旁白音频；视频渲染使用这个已带片头的音频。aeneas 对齐使用原始旁白音频，再在写出 SRT/ASS/LRC 时统一增加精确 `3000ms` 偏移，避免 3 秒静音片头误吸收第一句字幕。
- 双语硬字幕时间轴必须优先由 aeneas 基于原始旁白音频重新对齐生成，并在输出阶段补齐片头偏移；人工估算字幕只能作为缺少 aeneas 环境时的失败前占位，不能标记为最终成片。
- 当命令行传入 `--force-aeneas` 时，必须忽略输出目录中已经存在的 `.aeneas.zh-en.ass`，重新执行 aeneas 对齐，并重新生成 `hard_subtitle.aeneas.zh-en.srt` 与 `hard_subtitle.aeneas.zh-en.ass`。
- aeneas 对齐优先在当前 Python 进程内执行；如果当前 Python 缺少 aeneas，则必须自动调用 `AENEAS_PYTHON` 或 `C:\Program Files\Python39\python.exe -m aeneas.tools.execute_task`，让默认 Python 继续负责 Pillow/视频渲染，Python39 只负责 aeneas 对齐。
- 英文字幕文本优先来自 `subtitles_en.json` / `translation_cache.json`，也可以在条数完全一致时从已有双语 aeneas ASS 的 `English` 样式行抽取；如果都不存在，Tauri 必须把设置中的 OpenAI 兼容 `baseURL`、`model`、`apiKey` 通过环境变量传给视频脚本，由脚本调用 Codex/AI 自动意译并写入 `translation_cache.json`。最终时间戳仍必须以本次 aeneas 生成的单语 SRT 为准。
- aeneas 单语字幕文件必须按配置语言输出，例如 `hard_subtitle.aeneas.cmn.srt` 和 `hard_subtitle.aeneas.cmn.lrc`；双语字幕必须同时输出 `hard_subtitle.aeneas.zh-en.srt`、`hard_subtitle.aeneas.zh-en.ass` 和 `hard_subtitle.aeneas.zh-en.lrc`。
- 复用历史 aeneas ASS 时必须检测首条字幕开始时间：如果已经包含片头偏移，不得再次叠加；如果未包含片头偏移，才按当前 header 音频时长补齐。
- 端到端验证必须检查 `pipeline_manifest.json` 中 `subtitleTiming` 为 `aeneas`，并抽查 ASS 首条字幕应在片头之后约 3 秒开始，而不是 6 秒或 10 秒。
- 首页流水线按钮包含【素材】【音频】【视频】【发布】四步；【发布】读取当前任务 `output` 目录中的 `materials.json`、`hard_subtitle.aeneas.zh-en.srt` 和 `pipeline_manifest.json`，在同一 `output` 根目录生成 `youtube_publish.md`。Markdown 必须包含推荐标题、备选标题、视频简介、关键时间线、标签、Hashtags、置顶评论和发布文件路径，便于直接复制到 YouTube。
- 后续迁移到 MacMini n8n 时，n8n 只应调用同一套命令行脚本；SSH 免密访问按 `D:\0030_codex\tools\ssh-免密登录排障与复用指南.md` 复用已生效 key，并通过环境变量或参数显式传入 ffmpeg、Python/aeneas、header 音频和背景音乐路径。
## 2026-06-25 专业听书视频视觉规则

- `a-book-in-30-minutes` 的视频目标是“专业、漂亮的 30 分钟读一本书”成片，不再是静态封面加固定背景图轮播，也不是白板动画。
- 视频合成层必须把每张正式图片当作一个镜头处理：每个镜头有独立的 `motionProfile`，包括中心慢推、左右漂移、上升、下降和缓慢拉远，避免全片使用相同的缩放节奏。
- 无字幕母版视频必须应用统一的电影感处理：轻微压暗、适度对比度、低饱和、锐化、暗角和极轻微颗粒，让不同来源的视觉图统一到同一套听书频道气质。
- `visual_timeline.json` 必须记录封面和内容片段的 `motionProfile`，方便后续在 UI、n8n 或命令行中复查每张图的镜头语言。
- `visual_story_plan.json` 是吸收 `whiteboard-video-workflow` 思路后的视觉策划产物，必须包含统一风格规则、每个内容镜头的起止时间、图片路径、`motionProfile` 和可交给 Codex/n8n/Images API 的提示词。
- 后续正式图片生成层应产出少量高质量主题图和明确的视觉时间轴，而不是按每句字幕生成一张图。图片生成可以先由 Codex `$imagegen` 编排保证审美和连续性，量产后再迁移到 n8n + 真实 Images API。
- 0.1.84 起，视频脚本的默认本地合成效果应优先服务专业听书视频：慢节奏、稳定、细腻、有连续镜头感，避免占位图、机械推拉和过度动画。
- 0.1.85 起，流水线页面选择新的 EPUB 文件时必须同步更新当前任务并清空旧的批量勾选状态；视频和发布阶段的目标解析必须优先使用输入框中的 `epubPath`，避免旧选中任务抢占新路径。
- 0.1.86 起，应用启动日志和状态读取日志不得包含乱码；历史任务列表读取时不会把上次中断留下的 `generating` 状态继续显示为生成中，而是恢复为待手动继续，避免用户误以为应用启动后自动执行历史任务。
- 0.1.87 起，前端静态资源必须包含 `favicon.ico`，避免 Tauri/WebView 启动时反复输出 `Asset 'favicon.ico' not found` DEBUG 日志。
- 0.1.88 起，操作日志写入层必须在入库和写文件前拦截明显 mojibake/乱码内容，并按模块与动作替换为可读日志说明，避免历史硬编码乱码继续污染日志页面。

## 2026-06-26 0.1.89 日志与素材链路乱码清理

- 默认素材配置必须使用正常 UTF-8 中文：频道名、分类名、飞书测试消息和生成方向不得再包含历史 mojibake 字符。
- 操作日志写入层在入库和写入文本日志前统一识别明显 mojibake，并按 `module/action` 替换为可读英文日志说明，避免旧硬编码文案污染新任务日志。
- 本地素材兜底逻辑必须输出正常中文标题、简介、标签和旁白扩展，不得再生成连续问号串。
- 语音时长估算、句子切分和字幕切分统一使用正常中英文标点：`。？！；，,.!?;`，避免历史乱码标点造成 Rust warning 和切分异常。
- 端到端验证指定 EPUB：`D:\books\0625新书四本\2025-01《山茶的情书》\山茶的情书.epub`。验证时必须确认任务路径、生成产物和最新日志均指向该 EPUB，且可见日志不包含明显 mojibake、替换字符或连续问号串。

## 2026-06-26 0.1.90 持久化设置迁移

- 现有用户机器上的 `settings.json` 可能已经保存过旧版本乱码配置，不能只依赖代码默认值修复。
- 应用启动读取设置后必须自动清理明显 mojibake 的默认配置字段：飞书测试消息、素材频道名、分类名、分类列表和生成方向。
- 迁移只处理明显历史乱码默认文案，不修改用户真实连接配置，例如 OpenAI 兼容 `baseURL`、`model`、`apiKey`、Azure Speech Key、ffmpeg 路径和流水线开关。
- 系统托盘菜单和 settings get/save 日志必须使用正常 UTF-8 文案。

## 2026-06-26 0.1.91 端到端验证入口

- release exe 支持 `--e2e-materials <epub-path>` 开发验证入口，用于在不依赖 UI 自动化的情况下复验素材生成核心链路。
- 该入口必须读取真实应用数据目录 `%APPDATA%\com.abookin30minutes.desktop` 下的 `settings.json`、`app.db` 和 logs，复用真实 AI 配置、EPUB 解析、素材 JSON 解析/本地兜底、输出目录写入和操作日志写入。
- 指定 EPUB 端到端验证命令：`a_book_in_30_minutes.exe --e2e-materials "D:\books\0625新书四本\2025-01《山茶的情书》\山茶的情书.epub"`。
- 验证通过条件：命令退出码为 0；输出目录存在 `materials.json`、`narration.txt`、`subtitles.txt` 等素材文件；旁白中文字数位于配置目标范围内；SQLite/text logs 最新 trace 不含明显乱码；产物内容不包含历史问号串或 mojibake。

## 2026-06-26 0.1.92 音频端到端验证入口

- release exe 支持 `--e2e-audio <epub-path>` 开发验证入口，用于读取源书 `output/narration.txt` 并使用真实 Azure Speech 配置生成旁白音频。
- 该入口复用应用后端 `generate_audio_from_text()`，输出 `书名.mp3`、`part_*.mp3`、`part_*.ssml`、`narration.ssml` 和 `audio_manifest.json` 到同一 `output` 目录。
- 视频流水线验证前必须先确认 output 目录存在可被 `book_video_pipeline.py` 发现的根目录 mp3，否则视频阶段应明确失败为缺少音频。

## 2026-06-28 MacMini4 本地图片服务方案 A

- MacMini4 已部署轻量 OpenAI-compatible 图片服务，Tailscale 地址为 `http://100.96.199.26:30019`，接口为 `POST /v1/images/generations` 和 `GET /health`。
- 服务目录固定为 `/Volumes/System/docker/book-image-service`，仓库源码在 `tools/book-image-service`；默认模型为 `Lykon/dreamshaper-8-lcm`，输出尺寸默认 `768x432`，适合作为读书视频分镜配图的本地预览和中等质量素材来源。
- MacMini4 使用 Apple MPS 推理时默认必须走 float32；历史验证中 float16 会导致 VAE 输出 NaN，最终生成纯黑图。服务通过 `BOOK_IMAGE_DTYPE=auto` 在 MPS 上选择 float32。
- Hugging Face 直连不稳定，模型已通过 `HF_ENDPOINT=https://hf-mirror.com` 补齐缓存；运行服务时设置 `HF_HUB_OFFLINE=1`，避免生成请求临时访问外网导致失败。
- 当前常驻方式采用 `install-watchdog.sh` 安装的 cron watchdog，每分钟检查 `http://127.0.0.1:30019/health`，服务不可用时调用 `start.sh` 拉起。launchd plist 已保留在仓库中，但 MacMini4 当前验证时出现 `EX_CONFIG`，后续可单独修复。
- 端到端 smoke test 已通过：Windows 通过 Tailscale 调用 `http://100.96.199.26:30019/v1/images/generations`，样张保存到 `tmp/macmini_book_image_smoke/scene_02.png` 和 `scene_03.png`；`scene_03.png` 约 514KB，颜色数 24633，服务端耗时约 12.72 秒。
- 后续接入 `a-book-in-30-minutes` 时，应把该服务作为可配置 image provider，而不是写死在视频流水线里；配置项至少包含 `baseURL`、`model`、`size`、`steps`、`guidanceScale` 和 `seed`。图片内容提示词必须来自字幕区间/分镜摘要，避免生成与文本无关的通用背景图。

## 2026-06-30 0.1.95 发版与新 EPUB 验证

- `a-book-in-30-minutes` 版本同步提升到 `0.1.95`，覆盖 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json` 和 `src/config/app.ts`。
- 本轮发版验证指定 EPUB：`D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\001没有宽恕就没有未来.epub`。
- 端到端验证仍按素材、音频、视频三阶段拆分执行，避免失败时无法定位。素材和音频优先使用 release exe 的 `--e2e-materials` 与 `--e2e-audio`；视频阶段必须复用同一套 `tmp/book_video_pipeline.py` 和应用设置中的 AI/ffmpeg/aeneas 配置。
- 本轮触达的视频入口错误文案必须保持正常 UTF-8，不得新增 mojibake；历史遗留乱码仍由日志写入层拦截，后续可单独做全量清理。

## 0.1.96 Visual Regeneration Prompt Route

- BOOK_IMAGE_PROMPT_STYLE=book-illustration now switches the video pipeline's image prompts from minimal whiteboard icon prompts to professional editorial book-summary illustration prompts.
- The route still calls the installed whiteboard-video-workflow image generator, but prompts emphasize concrete scenes, people, late-20th-century South Africa context, relationship tension, forgiveness, public truth, everyday objects, weather, and nature details.
- Negative guidance blocks readable text, duplicate protagonists, logos, watermarks, and abstract single-object icons so generated visuals are more suitable for final videos.


## No Future Without Forgiveness Visual System v1

- Before regenerating images for D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\001没有宽恕就没有未来.epub, the workflow now pauses to define style, recurring elements, noun/object inventory, and scene-to-time mapping.
- Style target: professional 30-minute book-summary illustration, warm hand-painted storybook texture, cinematic mid/wide shots, concrete people and settings, no abstract single-object icons.
- Recurring elements: desk lamp, window and light, archival testimony papers, hearing microphone, empty chair, road, candle, ballot box, old photograph, family table.
- A dedicated design artifact was generated at D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_regen_design_001\`visual_style_bible.md` with noun extraction and 8 timeline scenes.


## No Future Without Forgiveness Programmatic Visual Regeneration

- The No Future Without Forgiveness regeneration now follows a visual-system-first process: style bible, recurring elements, noun/object inventory, timeline scenes, then image generation.
- AI image generation through MacMini Realistic Vision was tested but rejected for this book because it repeatedly produced open book spreads, fake text, or empty landscapes instead of the requested hearing/family/documentary scenes.
- Added `a-book-in-30-minutes`/tmp/build_no_future_visual_design.py to build clean UTF-8 design artifacts and prompts for the book without mojibake.
- Added `a-book-in-30-minutes`/tmp/render_no_future_programmatic_illustrations.py to render 8 controlled 1920x1080 documentary-style illustrations with people, scene objects, light/weather, and a lower subtitle-safe band.
- The regenerated video uses `visualSourceKind=task_visual_assets and `visualAssetCount=8, with assets copied into the material output under `visual_assets/originals/programmatic_no_future_v1.
- Current caveat: the programmatic visuals are stable and semantically matched but still visually simple; the video render adds a dark vignette/toning pass that should be brightened in a later polish pass.


## Programmatic Visual Polish v2

- The content-video filter chain was brightened by replacing the previous negative brightness and vignette pass with a light positive brightness/saturation treatment and lower noise, so regenerated videos no longer look heavily darkened.
- 
ender_no_future_programmatic_illustrations.py now adds paper texture, shadows, facial details, table highlights, window light, paper stacks, plants, candle flames, and additional scene objects.
- The v2 output for No Future Without Forgiveness is stored under D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_regen_programmatic_video_002.
- Verification: hard-subtitle video probes as H.264 1920x1080 at 30fps; ASS contains 816 Chinese and 816 English dialogue lines; Unicode mojibake marker scan returned no hits in key manifest/subtitle files; the 12:30 verification frame is brighter and shows the hearing-room visual plus bilingual subtitles.


## Programmatic Visual Polish v3

- The v3 programmatic renderer adds a richer illustration layer: integrated subtitle-band gradients, background paper texture, stronger character silhouettes, hair/face/clothing details, rugs, distant-town layers, and sun/moon glow.
- The v3 output for No Future Without Forgiveness is stored under D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_regen_programmatic_video_003.
- Verification: hard-subtitle video probes as H.264 1920x1080 at 30fps; ASS contains 816 Chinese and 816 English dialogue lines; Unicode mojibake marker scan returned no hits in key manifest/subtitle files; the 12:30 verification frame shows the richer hearing-room illustration plus bilingual subtitles.
- Remaining design gap: visuals are now coherent and brighter but still intentionally programmatic. A future model-backed or artist-authored layer could make characters and backgrounds more painterly while keeping the current visual bible and timeline as constraints.

## 2026-06-30 受控程序化视觉流程

为避免图片服务随机生成旧风格、空泛背景、假文字或与字幕不匹配的画面，视频阶段新增“先设计后生成”的受控程序化视觉流程。该流程适用于《没有宽恕就没有未来》这类历史、纪实、读书解读视频，也作为后续通用视觉包的基线。

流程顺序固定为：

1. 读取当前 EPUB 对应的 `materials.json`、旁白和字幕产物。
2. 生成视觉风格圣经，先确定画幅、色彩、时代背景、禁用项、固定母题和人物规则。
3. 从旁白文本中统计人物、地点、物品、自然天气和抽象主题等名词，形成可审阅元素库。
4. 按 30 分钟视频拆成 8 个核心时间段，每段绑定主题、人物、地点、道具、情绪和英文出图提示词。
5. 程序化渲染 8 张 `1920x1080` 正式场景图，文件名使用 `scene_*.png`；`contact_sheet_8.png` 只作为审阅联系表，不能进入视频时间轴。
6. 主流水线根据字幕事件生成 `visual_story_plan.json` 和 `visual_timeline.json`，封面段之外的 8 张图片必须绑定字幕时间区间。

命令行开关：

```powershell
python a-book-in-30-minutes\tmp\book_video_pipeline.py `
  --epub "<book.epub>" `
  --output-dir "<output_dir>" `
  --visual-assets-only `
  --controlled-programmatic-visuals `
  --ignore-existing-visual-assets
```

其中 `--controlled-programmatic-visuals` 表示启用受控程序化视觉；`--ignore-existing-visual-assets` 表示跳过素材包里已有的旧视觉素材，强制按当前书重新生成设计和图片，避免新 EPUB 或重跑任务误用旧图。

当前《没有宽恕就没有未来》的专用产物结构：

```text
<output_dir>\controlled_visual_design\`visual_style_bible.md`
<output_dir>\controlled_visual_design\`visual_style_bible.json`
<output_dir>\controlled_visual_design\`prompts_8.md`
<output_dir>\controlled_visual_design\`prompts_8.json`
<output_dir>\controlled_programmatic_visuals\scene_*.png
<output_dir>\controlled_programmatic_visuals\contact_sheet_8.png
<output_dir>\controlled_programmatic_visuals\programmatic_visual_manifest.json
```

质量约束：

- 正式图片只允许使用 `scene_*.png`，不能把联系表、截图、缩略图、旧 AI 图或预览图混入视频。
- 图片必须为 `1920x1080`，并保留底部字幕安全区。
- 图片中不得出现可读文字、伪文字、logo、水印、重复主角、血腥画面和现代手机电脑等违背时代背景的物件。
- 视觉设计文件、提示词、manifest、字幕和日志必须为 UTF-8 正常中文，不得出现替换字符、常见 mojibake 标记或连续问号替代中文。
- 目前渲染器仍是《没有宽恕就没有未来》的专用绘制器，后续要抽象为“风格包 + 组件库 + 分镜计划”的通用框架。

2026-06-30 端到端验证状态：

- `output_regen_integrated_video_001` 已验证受控程序化视觉可以进入完整视频阶段。
- 输出包括无字幕母版和中英双语硬字幕精修版，两者均为 H.264、`1920x1080`、`30fps`、约 `1803` 秒。
- 硬字幕文件包含 816 行中文和 816 行英文，符合当前字幕对齐结果。
- `visual_timeline.json` 包含 1 段封面和 8 段正式场景图，正式图仅引用 `scene_*.png`。
- 当前质量短板是画面仍偏简化，人物体块、表情、服装、生活物件和环境层次需要继续增强；后续优化应沿着程序化组件库升级，而不是回退到旧 placeholder 或不稳定的随机图片服务。

2026-06-30 第五版图片质量调整：

- `render_no_future_programmatic_illustrations.py` 的人物组件从线条四肢升级为带躯干高光、衣服分片、圆角四肢、手部和表情的统一人物。
- 新增照片、档案盒、茶杯、座椅和听众行等可复用道具组件。
- 第四场听证会补充墙面层次、听众席和更多档案道具；第五、八场室内空间补充墙面板和生活物件；其他场景补充植物、房屋门窗和桌面物件。
- `output_regen_integrated_visuals_005` 是当前推荐接入视频的图片版本：8 张正式图均为 `1920x1080`，来源为 `controlled_programmatic_visuals`，时间轴仍是 1 段封面加 8 段场景图。
- 仍需继续提升的方向：人物比例更自然、主角更突出、场景透视更强、物件细节更细腻、画面叙事更接近专业读书视频。

2026-06-30 完整视频第二版：

- `output_regen_integrated_video_002` 使用第五版程序化插画重新生成完整视频，输出无字幕母版和中英双语硬字幕版。
- 验证结果：两版视频均为 H.264、`1920x1080`、`30fps`、约 `1803` 秒；字幕仍为 816 行中文和 816 行英文；时间轴仍为 1 段封面和 8 段正式场景图。
- 抽帧确认第五版听证会场景已进入硬字幕视频，画面比上一版更丰富，人物体块和道具密度有所改善。
- 遗留质量问题：第 4 场听众席的一部分在硬字幕版中接近字幕区下沿，后续可继续将主体上移、减少底部小人物，或为硬字幕版生成专门留白更大的画面布局。

2026-06-30 完整视频第三版：

- `output_regen_integrated_video_003` 修正第 4 场听证会硬字幕安全构图：听众席上移、缩小并减少数量，桌面和主讲人物整体上移，底部留白更干净。
- 验证结果：两版视频均为 H.264、`1920x1080`、`30fps`、约 `1803` 秒；字幕仍为 816 行中文和 816 行英文；时间轴仍为 1 段封面和 8 段正式场景图。
- 抽帧 `frame_12m00s.jpg` 确认第 4 场硬字幕区不再被听众席干扰。第三版是当前推荐查看的视频版本。
- 后续质量重点转为整体美术精细度：人物自然比例、主角动作、透视层次、物件细节和更强的故事性。

2026-06-30 桌面端接入：

- `GenerateBookVideoRequest` 新增 `controlledProgrammaticVisuals` 和 `ignoreExistingVisualAssets`，后端默认启用受控程序化视觉并忽略旧视觉素材。
- 桌面端【视频】按钮现在显式传入 `allowPlaceholderVisuals=false`、`controlledProgrammaticVisuals=true`、`ignoreExistingVisualAssets=true`。
- Python 视频流水线会收到 `--controlled-programmatic-visuals` 与 `--ignore-existing-visual-assets`，默认不再用占位图或旧视觉资产兜底。
- 版本同步递增到 `0.1.96`，需要用 GNU Tauri 环境重新打包开发版。

## 先定风格、元素、名词再生成图片

本轮将《没有宽恕就没有未来》的图片生成流程固定为“设计先行”：

1. 读取已生成的 materials.json / 旁白文本，先统计与视频内容相关的名词。
2. 名词分为人物/群体、地点/空间、物品/道具、自然/天气、抽象主题五类。
3. 根据名词命中、书籍主题和 30 分钟时间轴，先生成以下设计产物：
   - `visual_style_bible.json`
   - `visual_style_bible.md`
   - `visual_decision_report.md`
   - `prompts_8.json`
   - `prompts_8.md`
4. 图片生成必须以后述设计产物为输入，不能直接凭空生成图片。
5. `visual_decision_report.md` 必须说明：生成顺序、风格结论、固定元素、名词命中摘要、每段分镜取舍理由。

本轮验证输出目录：

```text
D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_style_elements_nouns_003
`

该目录包含：

```text
controlled_visual_design\`visual_decision_report.md`
controlled_visual_design\`visual_style_bible.md`
controlled_visual_design\`prompts_8.md`
controlled_programmatic_visuals\scene_*.png
controlled_programmatic_visuals\contact_sheet_8.png
controlled_programmatic_visuals\programmatic_visual_manifest.json
`

验证结果：8 张正式图片均为 1920x1080，manifest 记录 8 张，设计文件与 manifest 未发现乱码标记。视觉抽检显示流程顺序正确、内容贴合书籍主题，但程序化插画仍偏简洁，后续重点是提高人物表情、服装、透视、场景层次和手绘精细度。

## 本轮第五版受控插画资产

在 output_style_elements_nouns_003 的基础上，本轮继续增强 
ender_no_future_programmatic_illustrations.py：

- 新增地板透视、墙面书架、窗帘、便签纸、蜡烛组、灌木、照片墙等复用组件。
- 8 个场景分别补充了生活物品、空间层次、户外自然元素和历史现场道具。
- 场景 6 中靠近字幕安全区的尖锐斜线已弱化并上移，避免干扰后续中英双语硬字幕。
- `build_no_future_visual_design.py` 的输出目录创建改为 parents=True，避免父目录不存在时报错。

推荐图片资产目录：

`	ext
D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_style_elements_nouns_005
`

验证结果：

- python -m py_compile 通过。
- 重新生成设计产物和 8 张正式图片成功。
- 8 张 scene_*.png 均为 1920x1080。
- programmatic_visual_manifest.json 记录 8 张图片。
- 设计产物输出 badFiles=[]。
- 视觉抽检 contact_sheet_8.png：第五版比第三版更丰富，书桌、窗帘、照片墙、纸张、灌木、蜡烛和地面透视都已进入画面；仍属于受控程序化插画，后续若继续追求 B 站案例质感，应把人物角色、表情和关键动作进一步放大。

## 第五版图片视频链路与第六版人物升级

本轮完成两件事：

1. 使用第五版图片资产生成完整视频，验证图片可以进入正式视频链路。
2. 根据视频抽帧结果继续升级人物组件，生成第六版图片资产。

第五版完整视频输出目录：

`	ext
D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_video_style_elements_nouns_005
`

验证结果：

- visualSourceKind=task_visual_assets。
- visualAssetCount=8。
- 无字幕母版与中英双语硬字幕版均为 H.264、1920x1080、30fps、1803s。
- ASS 字幕共 1632 行：中文 816 行，英文 816 行。
- visual_timeline.json 共 9 段：1 段封面，8 段正式图片。
- 抽帧 4:20、12:00、20:00、27:00 显示字幕安全区可用，视频链路正常。

第六版图片资产目录：

`	ext
D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_style_elements_nouns_006
`

第六版改进：

- 人物四肢从线条改成带描边的圆角体块。
- 增加手部和鞋，降低火柴人感。
- 保留第五版已经验证的场景层次、物品密度和字幕安全区。

当前判断：第六版图片比第五版人物更扎实，是下一次视频接入的推荐候选。但整体仍是受控程序化插画，不是高精细手绘成片；下一步如果继续提质，应围绕“角色驱动构图”推进：放大关键人物，增加动作、表情、侧身/背身差异，再把它交给白板动画过程表现。

## 第六版完整视频验证通过

第六版图片已经接入完整视频链路，并验证通过。

最终输出目录：

`	ext
D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_video_style_elements_nouns_006_final
`

关键验证结果：

- visual_assets_manifest.json 的 sourceDir 明确指向 output\visual_assets\originals\programmatic_v006。
- visualSourceKind=task_visual_assets。
- visualAssetCount=8。
- 无字幕母版：H.264、1920x1080、30fps、1803s。
- 中英双语硬字幕版：H.264、1920x1080、30fps、1803s。
- ASS 字幕共 1632 行：中文 816 行，英文 816 行。
- visual_timeline.json 共 9 段：1 段封面，8 段正式图片。
- 抽帧 4:20 和 12:00 显示第六版人物体块已进入视频，字幕安全区可用。

最终视频文件：

`	ext
D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_video_style_elements_nouns_006_final\没有宽恕就没有未来_无字幕母版.mp4
D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_video_style_elements_nouns_006_final\没有宽恕就没有未来_中英双语字幕_精修版.mp4
`

当前判断：第六版已经是当前可验证的完整视频候选。它解决了“旧素材误用”和“人物火柴人感过重”的主要问题，但整体仍是程序化卡通插画。下一步质量方向是角色驱动构图：放大关键人物、增加动作与表情差异，再进入白板动画过程表现。

## 第七版抗锯齿图片候选

用户反馈第六版内容可以，但锯齿仍明显。本轮在不改变分镜内容的前提下，为程序化插画增加最终抗锯齿后处理：

- 新增 ANTIALIAS_SCALE = 2。
- 新增 ntialias_finish()。
- 最终画面经过 2 倍 Lanczos 重采样、轻微 Gaussian blur、轻量 UnsharpMask。
- 目标是减少斜线、圆角、人物四肢和物件边缘的硬毛刺，同时避免整体发糊。

第七版图片输出目录：

`	ext
D:\books\理想国译丛系列（74册）整理截止2026.018\001没有宽恕就没有未来\output_style_elements_nouns_007
`

验证结果：

- python -m py_compile 通过。
- 8 张 scene_*.png 均为 1920x1080。
- programmatic_visual_manifest.json 记录 8 张图片。
- 设计产物输出 badFiles=[]。
- 预览 contact_sheet_8.png 和 scene_02_0345_0730.png，边缘比第六版更顺，没有明显过度模糊。

当前判断：第七版适合作为下一次完整视频接入候选。仍有少量几何斜线锯齿，这是 Pillow 程序化几何绘制路线的自然上限；如果还要进一步提升，需要改为更高倍 supersampling，或切换到 SVG/Cairo/Skia 这类矢量渲染后再输出位图。

## 0.1.98 打包版流水线脚本定位修复

用户在 0.1.97 打包版流水线页面看到“找不到视频流水线脚本 tmp/book_video_pipeline.py”。源码和构建输出中脚本存在，问题在于安装版资源目录不一定等同于开发目录。

本版调整 `find_video_pipeline`：

- 保留开发目录查找：`book_video_pipeline.py`、`tmp/book_video_pipeline.py`。
- 增加 Tauri 打包资源查找：`_up_/tmp/book_video_pipeline.py`。
- 增加安装版资源目录查找：`resources/tmp/book_video_pipeline.py`。

目标是让开发版、构建输出目录和安装版都能找到同一套视频流水线脚本，避免用户在流水线页面无法继续生成视频。

## 0.1.99 从零开始任务选择与状态重置

用户要求《亲爱的老爸》全部从零开始。0.1.98 中步骤页会优先选择最近更新或旧选中任务，导致正确输入路径是 `D:\books\...` 时，界面仍展示旧的 `E:\迅雷下载...` 历史任务，并把素材失败、音频成功、视频失败拼在同一条流程里。

本版调整：

- 步骤页 `pickCurrentTask` 优先使用当前输入框 `epubPath`，其次才使用手动选中任务和运行中任务。
- 当前输入路径暂未出现在任务列表时，步骤页会构造一个全 pending 的占位任务，不再跳到旧任务。
- 首页 `loadStoredTasks` 加载历史任务后，如果当前输入路径存在于列表中，会自动选中当前输入路径。
- `reset_material_tasks` 后端接口从只重置素材状态，改为同时清空素材、音频、视频三段状态和产物路径，保证“从零开始”不会残留旧音频或旧视频。
- 前端清理单个任务/全部任务时，也同步清空三段状态和产物字段。

## 0.1.100 当前任务补读与分类修复

0.1.99 仍可能出现当前输入任务不在流水线列表里的情况：任务状态更新时如果 category 曾经被错误写成问号，`get_material_tasks` 按当前分类查询就会漏掉正在执行的任务，步骤页只能显示占位 pending，进度停在 0%。

本版调整：

- `update_material_task_status` 每次更新状态时同步写回规范 category，修复历史错误分类。
- 新增 `get_material_task` 命令，可按 path 读取单条任务真实状态。
- 首页 `loadStoredTasks` 在分类列表缺少当前输入路径时，按 path 补读真实任务并插入列表。
- 如果数据库确实没有当前任务，才使用全 pending 占位，避免步骤页跳到旧任务。
## 0.1.101 字幕翻译断点缓存

用户要求通过桌面快捷方式对《亲爱的老爸》重新端到端生成素材、音频、视频和发布材料。排查发现当前视频准备阶段卡在中英双语字幕生成：中文 aeneas 对齐已经完成，但 895 条字幕需要调用 Codex 中转逐批翻译为英文，旧脚本在长时间翻译过程中没有落盘进度，进程中断后会从头开始，界面上也容易表现为 `VID_PREP` 长时间无进展。

本版本调整 `tmp/book_video_pipeline.py`：

- AI 字幕翻译每批完成后写入 `translation_cache.partial.json`。
- 再次运行时自动从 partial cache 续跑，避免重新翻译已完成字幕。
- 完整翻译完成后写入 `translation_cache.json`，并删除 partial cache。
- stderr 输出 `Translated subtitle cues 当前/总数`，便于日志和后续 UI 进度排查。

本地验证：使用《亲爱的老爸》输出目录，以 `ABOOK_TRANSLATE_BATCH_SIZE=5` 运行视频素材准备流程，5 分钟内从 0 翻译到 230/895，并生成 `translation_cache.partial.json`，证明 Codex 中转配置可用且断点缓存有效。

## 0.1.102 恢复真实任务刷新模型

用户反馈 0.1.101 中新增 EPUB 后列表首屏只有当前 1 条、切换菜单后历史任务才回来，步骤跟踪没有及时显示素材解析进度，日志页看起来也没有持续刷新。回看 2026-06-22 日报和 0.1.49-0.1.53 历史实现后确认：正确模型应以 SQLite `material_tasks` 为唯一任务真相，前端只展示真实入库任务；日志按主 trace 聚合子 trace；视频后台启动后前端释放 busy，真实进度从任务表和操作日志读取。

本版本恢复这一模型：

- 流水线页扫描结果与历史任务列表合并，不再用单文件扫描结果覆盖整个任务列表。
- 当前输入路径只有在后端 `get_material_task` 读到真实任务时才插入列表，不再构造前端假 pending 任务。
- 步骤跟踪页只展示真实任务，不再因为输入框路径构造全 pending 占位。
- 操作日志页保留 2 秒刷新，并在用户停留底部时自动跟随最新日志，同时显示最后刷新时间和日志条数。

## 0.1.103 修复素材流转显示与素材日志乱码

用户选择《亲爱的老爸》后点击【素材】，日志显示后端实际已启动：09:46:32 读取 EPUB，09:46:33 发起 AI 请求，主请求 226 秒后返回，09:51:11 完成素材生成并导出。但 UI 表现仍像没有正常流转：步骤页可能展示输入框默认路径而非选中任务，前端初始状态短暂显示 0%，后端素材生成 message 中还残留乱码。

本版本调整：

- 步骤跟踪页当前任务选择改为优先使用用户在流水线列表中选中的任务，其次才使用输入框路径。
- 点击【素材】后前端立即写入 `generating / 10% / 正在准备素材生成任务`，避免长时间 AI 请求期间被误判为未启动。
- 清理素材生成主链路运行时文案，覆盖任务准备、源书读取、AI 请求、AI 返回、旁白修复、字幕切分、素材导出等阶段，避免 `material_tasks.message` 和 `operate_log.message` 继续写入 mojibake。
- 本轮《亲爱的老爸》素材生成成功，输出旁白 7629 个汉字、字幕 918 行，素材包位于书籍目录的 `output`。

## 0.1.104 历史任务消息净化

0.1.103 修复了新写入的素材生成 message，但本机数据库中仍有旧版本写入的乱码 `message`、`audio_message`、`video_message`。这些旧数据会在读取历史任务时继续显示到流水线列表和步骤页。

本版本在 `material_task_from_row` 读取任务时增加 message 净化：如果检测到 mojibake，则按阶段替换为可读文案。该处理只影响展示层读取结果，不会自动续跑历史任务，也不会修改任务状态。

## 0.1.105 修复后台视频任务被展示层重置

用户点击【视频】后，后端已经写入 `video_status=generating / video_progress=45` 并启动 Python 视频流水线，但前端轮询 `get_material_tasks` 时，后端展示层会把所有 `generating` 状态改成 `pending`，这是历史“启动时不自动续跑未完成任务”的保护逻辑，误伤了当前正在运行的视频后台任务。

本版本调整：

- `get_material_tasks` 和 `get_material_task` 读取任务时不再把 `generating` 改成 `pending`，只展示数据库真实状态。
- 防止自动续跑仍由前端按钮触发模型保证：应用启动、页面加载和读取历史列表只展示状态，不主动调用素材、音频、视频或发布任务。
- 扩展乱码检测，覆盖旧库中常见中文 mojibake 片段，减少旧任务说明污染步骤页。

## 0.1.106 六阶段按钮与 A-F 步骤编码

用户要求流水线顶部阶段改成 6 个横向按钮，当前正式顺序为：【文本】、【音频】、【字幕】、【图片】、【视频】、【发布】，并同步把步骤跟踪编码改成 `A-01`、`A-02`、`B-01`、`B-02` 到 `F-01` 的分组格式。

本版本调整：

- 首页阶段按钮改为六阶段并横向一字排开，按钮宽度固定，窄视图下允许横向滚动但不换成两行。
- 【文本】承接原素材生成；【音频】承接原音频生成；【视频】承接原视频生成；【发布】承接发布资料生成。
- 【图片】和【字幕】先接入现有视频流水线入口，因为当前图片、字幕由视频流水线统一生成；后续后端拆出独立图片/字幕命令时，前端阶段入口已就位。
- 步骤跟踪改为：A=文本、B=图片、C=音频、D=字幕、E=视频、F=发布。

## 0.1.107 配置页同步六阶段跳过策略

用户指出设置页里的“已有则跳过”配置仍然只有原来的 3 项，和首页 6 个阶段按钮不一致。

本版本调整：

- 设置页跳过配置改为 6 项：文本、图片、音频、字幕、视频、发布资料。
- 前端 `PipelineProfile`、默认配置、Tauri 设置模型同步扩展 6 个字段。
- 本地配置读取时会合并默认值，旧 settings.json 缺少新字段时仍能正常显示和保存。
- `skipExistingMaterials` 作为旧版兼容字段保留，和 `skipExistingText` 双向同步；首页文本阶段判断优先使用 `skipExistingText`，旧字段只作为后备。

## 0.1.108 图片和字幕状态拆分

用户反馈 0.1.107 虽然已有六个阶段按钮，但任务列表仍只有素材、音频、视频三段状态；点击【图片】时还会进入视频流水线前置逻辑并补生成音频。

本版本调整：

- `MaterialFile` 和 SQLite `material_tasks` 增加图片阶段字段：`imageStatus`、`imageProgress`、`imageOutputDir`、`imageMessage`。
- 增加字幕阶段字段：`subtitleStatus`、`subtitleProgress`、`subtitleFile`、`subtitleMessage`。
- 首页任务列表新增“图片 / 图片进度 / 字幕 / 字幕进度”四列，让六阶段状态可见。
- 【图片】按钮只补文本素材，不再补音频；后端收到 `pipelineStage=image` 后给脚本传 `--visual-assets-only`，只生成封面、分镜图、视觉计划和时间线，不渲染音频与视频。
- 【字幕】阶段仍允许补音频，因为 aeneas 对齐需要旁白音频作为时间轴输入。
- 修复 `material_tasks` 新建表和迁移中的默认分类乱码，统一为“半小时听完一本书”。

## 0.1.109 图片阶段提前退出字幕流程

用户点击【图片】后，列表显示图片失败。排查任务表发现 `imageMessage` 写入的是字幕翻译进度，说明 `--visual-assets-only` 参数虽然传给了脚本，但脚本分支位于音频准备、aeneas 对齐和英文字幕翻译之后，图片阶段仍会先进入字幕流程。

本版本调整：

- `book_video_pipeline.py` 在解析素材包后立即处理 `--visual-assets-only`。
- 图片阶段使用 `subtitles.txt` 和音频 manifest 的预估时长构造视觉分段，不再准备音频、不再执行 aeneas、不再翻译英文字幕。
- 图片阶段仍输出 `cover.jpg`、`controlled_programmatic_visuals`、`visual_story_plan.json`、`visual_timeline.json` 和 `pipeline_manifest.json`。

## 0.1.110 图片按钮目标选择修复

用户想重新生成图片时发现【图片】按钮灰掉，并且状态提示仍在“补生成文本”。根因是图片、字幕、视频、发布按钮共用 `getVideoPipelineTarget`，目标筛选一直要求 `shouldGenerateVideo`；同时旧任务虽然已经有 `materialOutputDir`，但 `status=generating` 会触发文本补生成。

本版本调整：

- 按阶段拆分目标判断：图片按钮使用 `shouldGenerateImage`，字幕按钮使用 `shouldGenerateSubtitle`，视频按钮才使用 `shouldGenerateVideo`。
- 发布按钮不再被视频是否待生成限制。
- `needsMaterialGeneration` 看到已有 `materialOutputDir` 时不再强制补文本，避免旧状态卡住图片重生成。
- 已对《天会亮的，你有我呢》重新生成一版图片，输出 8 张 scene 图、封面、contact sheet 和 visual timeline。

## 0.1.111 文本阶段字幕语义分句

用户指出 `subtitles.txt` 不能只按固定长度硬切。旧逻辑会把“完美”拆成“完 / 美”，把“的书”“你有我呢”等短语拆成孤立行，影响后续字幕、图片分镜和视频节奏。

本版本调整：

- AI 素材生成 JSON 增加可选 `subtitles` 字段。
- 文本生成提示词要求模型先写完并理解最终 `narration`，再按语义、朗读停顿和短句完整性生成 `subtitles` 数组。
- 提示词明确要求字幕行通常不超过 16-20 个中文，长句只在语义边界拆为两句。
- 提示词明确禁止机械拆分短书名、人名、固定短语，例如“完美”“的书”“你有我呢”“天会亮的，你有我呢”。
- 后端优先使用 AI 返回的语义字幕；如果缺失或覆盖明显不足，再用本地分句兜底。
- 本地兜底从“逗号也断 + 14 字硬切”改为：句号/问号/叹号/分号优先断句，逗号等软停顿只在 16 字以上才断；超长句按 20 字上限柔性拆分，并合并 1-5 字短尾，避免孤立短词。
- 视频脚本中的 Python 兜底分句同步同一策略，避免旧素材被二次硬切。

## 字幕策略演进记录

字幕生成必须作为可持续演进的独立策略维护，每次改动都要记录目标、失败案例、规则变化和验证方式。

当前策略：

要什么：

- 优先让 AI 在理解最终 `narration` 后输出 `subtitles` 数组，而不是只在后端机械切分。
- `subtitles.txt` 必须保留自然中文标点，包括逗号、句号、问号、叹号、书名号和必要停顿。
- 每行是一句语义完整的朗读短句，通常 16-22 个中文字符含标点。
- 长句只在标点或清晰语义边界拆分，不拆固定短语、短书名、人名和短词。
- 字幕要像人自然朗读时会停顿的地方：一个画面、一口气、一层意思。
- 同一段情绪或动作可以拆成两句，但每句都必须单独成立。

不要什么：

- 不要为了凑长度硬切短词、短语、书名、人名。
- 不要把“完美”“希望”“的书”“你有我呢”这类短语拆成孤立行。
- 不要把标点全部删除，字幕需要保留自然阅读节奏。
- 不要出现大量 1-5 个字的孤行，除非它本身是刻意的短句。
- 不要让一行塞入多个无关分句，导致字幕太长、读不完。
- 不要为了断句改变原文含义、增删内容或把前后句拼成新意思。

失败案例库：

- 明确失败案例：“完 / 美”、“的书”、“你有我呢”、“天会亮的 / 你有我呢”、“三十三 / 个四季小故事”。
- 后端本地兜底只用于模型未返回字幕或字幕覆盖明显不足时，兜底规则必须比 AI 规则更保守。

后续每次字幕相关问题都要在本节追加一版，不能覆盖历史判断。

## 0.1.112 字幕保留标点与旁白总长度约束

用户指出 0.1.111 生成的 `narration.txt` 总字符达到 9000+，体感过长；同时 `subtitles.txt` 被去掉标点，断句仍然生硬。

本版本调整：

- 默认文本目标从 7000-8300 下调为 6200-7600 中文字，默认提示改为 25-30 分钟听书节奏。
- 提示词新增总长度约束：`narration.txt` 通常控制在 8200 个总字符以内，避免只按中文字符统计导致文件总长过大。
- 提示词改为要求字幕保留自然中文标点，不再去标点。
- 后端 `clean_subtitle_line` 改为只清理首尾空白，不删除逗号、句号、问号、书名号等。
- Python 视频兜底分句同步保留标点。
- 旁白长度修复提示词也必须继承同一套字幕规则库，避免第一次生成过长后进入 repair 流程时，又回到机械切分、删除标点或拆碎书名短语。
- 当前字幕提示词边界分为四层：先理解最终旁白、保留标点和语义短句；明确禁止机械定长切分和孤词孤句；维护失败案例库；用好例子约束模型输出节奏。

## 0.1.114 字幕提示词结构化升级

用户提供了 Gemini 对《天会亮的，你有我呢》第一段字幕断句的参考结果。该结果的优势不是规则更多，而是提示词结构更接近专业任务说明：先设定“资深短视频字幕编辑与治愈系电台文案策划”的角色，再给出任务、技术限制、断句准则和输出示例。

本版本将素材生成提示词和旁白长度修复提示词同步改为结构化字幕提示词：

- `Role`：要求模型以短视频字幕编辑和治愈系深夜电台文案策划身份处理字幕，关注朗读节奏、情绪起伏和呼吸感。
- `Constraint`：字幕行尽量控制在 18 个中文字符以内，含标点；必须保留自然中文标点；输出仍是 JSON 字符串数组，无时间戳。
- `Segmentation logic`：优先在标点和语义停顿处断句，长短结合，避免 1-5 字孤行，保护书名、人名、固定短语、数字表达和情绪短句。
- `What to avoid`：禁止机械按 14、16、18、20 字切分，禁止去标点，禁止把《天会亮的，你有我呢》、“蒲公英”、“三十三个四季小故事”等拆碎。
- `Output example`：直接引用更接近目标效果的断句样式，例如“今晚要一起读的是，”“一平著绘的《天会亮的，你有我呢》。”“先把灯光调暗一点，”“把白天没有说完的话，”“轻轻放在枕边。”

后续字幕策略继续以该结构为模板演进：新增规则要放入对应栏目，新增失败案例放入 `What to avoid` 或失败案例库，不能把提示词退回散装规则堆叠。

## 0.1.115 独立字幕生成单元测试

用户指出当前 `subtitles.txt` 与 `Gemini.txt` 差异仍然很大，并明确新的流程边界：`narration.txt` 已经生成就不要动；当前只优化 `subtitles.txt`；后续音频会根据字幕生成，因此字幕可以作为更高质量的“字幕成稿”，不必逐字对齐旧旁白音频。

本版本在 `tmp/book_video_pipeline.py` 新增独立字幕测试入口：

- `--subtitles-only`：只读取当前 output 中的 `narration.txt`，生成新的字幕文本，不执行音频、图片、视频流程。
- `--subtitle-output-name`：指定输出文件名，默认不覆盖原 `subtitles.txt`。
- `--subtitle-max-input-chars`：只处理前 N 个字符，便于做第一段单元测试。
- `--subtitle-batch-chars`：按句子边界分批调用 AI，规避中转平台 120 秒 524 超时。
- 修复 Python 兜底分句中的旧乱码标点表，改用正常中文标点/Unicode 转义，避免再次出现标点识别失败。

本轮对《天会亮的，你有我呢》完成了多轮字幕单测：

- 旧 `subtitles.txt`：552 行、7828 字、平均行长 13.18。
- `Gemini.txt`：267 行、4009 字、平均行长 14.02。
- AI 完整分批原稿 `subtitles_ai_full_v2_b1000.txt`：690 行、7133 字、平均行长 9.34，仍偏碎。
- 后处理合并候选 `subtitles_ai_full_v2_merged.txt`：367 行、6810 字、平均行长 17.56，是当前推荐候选结果，但尚未覆盖原 `subtitles.txt`。

结论：如果以完整 6415 字 `narration.txt` 为输入，候选字幕不可能与 4009 字的 `Gemini.txt` 达到 99% 字符相似；要达到 99%，必须把目标定义为“同长度压缩改写稿”，或直接以 `Gemini.txt` 作为黄金样本覆盖。后续正式接入应用时，应把【字幕】阶段改为独立 AI 字幕成稿流程，先生成候选、展示统计与预览，再由用户确认是否覆盖 `subtitles.txt`。

## 0.1.116 桌面文本阶段字幕标点修复

用户用桌面快捷方式《A Book in 30 Minutes 开发版》生成《亲爱的老爸》后，发现 `output/subtitles.txt` 仍然没有标点，并且出现“信 / 纸”这类错误断句。排查确认桌面文本阶段实际写入 `subtitles.txt` 的 Rust 后端仍使用旧乱码标点表，导致中文 `，。？！；：` 无法被识别，最终退化为按长度硬切。

本版本修复 Rust 文本阶段字幕切分：

- `split_subtitles` 改为保留当前字符后再判断断点，硬断点使用 `。？！；!?;`，软断点使用 `，、：“”‘’`。
- `best_subtitle_split` 改为使用统一的标点判断函数，移除旧乱码标点表。
- `normalize_ai_subtitles` 增加标点密度质量门槛：AI 返回字幕如果标点过少，会被视为不可用，自动回退到本地保留标点分句，避免再次写出无标点 `subtitles.txt`。
- `MAX_SUBTITLE_CHARS` 调整为 24，降低把“信纸”等短词拆开的概率。

已对现有《亲爱的老爸》输出目录执行一次文件级修复：备份旧 `subtitles.txt` 为 `subtitles_before_punctuation_fix_*.txt`，并基于现有 `narration.txt` 重新生成带标点的 `subtitles.txt`。修复后前几行包括“一个名字在信纸上慢慢长大。”，不再拆成“信 / 纸”。

## 2026-07-02 0.1.117 Pipeline Stage Semantics

This version fixes the main pipeline contract for the desktop workflow:

1. Text: generate `narration.txt`, `subtitles.txt`, and material metadata.
2. Audio: generate narration audio from `subtitles.txt`; it must ensure Text exists first.
3. Subtitle: generate timed `SRT/ASS` files from `subtitles.txt` plus audio; it must ensure Text and Audio exist first.
4. Image: generate visual assets from subtitle text and timing context; it must ensure Text exists first and must not generate Audio.
5. Video: generate video from visual assets, audio, and timed subtitles; it must ensure Text, Audio, and Subtitle exist first.
6. Publish: generate release/publish materials from the completed task outputs.

Frontend buttons are now six separate actions: Text, Image, Audio, Subtitle, Video, Publish. The Subtitle backend stage maps to `--audio-subtitle-only`, so it only creates timed subtitle files and does not render images or video.

## 2026-07-02 0.1.118 Text Skip Fix

The Text stage skip decision now follows the visible setting `skipExistingText` first and falls back to the legacy `skipExistingMaterials` value only for compatibility. When text skipping is enabled, an existing `materialOutputDir` is treated as sufficient evidence that text assets already exist, so later Video status changes must not force Text regeneration.

The Step Tracking page now reads Image status from `imageStatus/imageProgress`, Subtitle status from `subtitleStatus/subtitleProgress`, and treats Text as complete when a material output directory exists. This prevents Video progress from polluting Image/Subtitle rows and prevents a stale top-level `generating` state from making existing text assets look unfinished.

## 2026-07-02 0.1.120 Background Music and Chinese Status Text

The video pipeline restores background music for generated videos. `toolProfile.backgroundMusicPath` defaults to `D:\04_GitHub\world-cup-issue\a-book-in-30-minutes\music\01-蝴蝶飞呀.mp3`, and `toolProfile.backgroundMusicMode` records whether the UI uses single-track loop or playlist loop. The current renderer passes one background music file to `book_video_pipeline.py`; the Python renderer loops that track and mixes it into the no-subtitle video at low volume. The hard-subtitle video is rendered from the no-subtitle video, so it keeps the same mixed audio.

Visible stage status and the pipeline messages touched by this change must be Chinese. Step status labels render as `成功`、`失败`、`进行中`、`待处理`. Missing text/audio/image/subtitle/video artifact messages are normalized to Chinese when task rows are loaded.

When only updating the desktop shortcut development build, use `pnpm -C a-book-in-30-minutes tauri build --ci --target x86_64-pc-windows-gnu --no-bundle`; this updates the release exe and embeds frontend assets without creating an installer.

## 2026-07-02 配置与跳过逻辑修复设计记录

- 配置持久化统一使用 SQLite：应用配置保存到 `app.db` 的 `app_settings` 表，键名为 `settings`，值为完整 `AppSettings` JSON。`settings.json` 只作为旧版本迁移来源；数据库无配置且旧文件存在时读取一次，写入 SQLite 成功后删除旧文件。
- 前端点击【图片】【字幕】【视频】等阶段按钮时，先调用后端 `get_settings` 刷新最新配置，再调用 `get_material_task` 刷新当前任务。阶段跳过判断不再依赖页面缓存中的旧任务对象。
- 文本阶段跳过以归一化后的任务为准：`status=success`、`progress=100` 且 `materialOutputDir` 存在即可跳过，不再因为 `narrationChars` 为空而误触发文本生成。后端读取任务时会根据 `narration.txt` 自动补齐 `narration_chars`。
- 后端 `material_task_from_row` 会在返回前按磁盘真实产物归一化阶段状态：文本、音频、图片、字幕、视频产物缺失时对应阶段回到 `pending`；已有产物时保持或修正为可跳过状态。
- 阶段流水线提示必须使用中文；新增或修改的提示优先使用源码安全写法，避免 Windows 控制台编码把中文写成问号或 mojibake。
## 2026-07-03 0.1.126 Background Music bf.mp3 98 Percent

This version switches the default background music path to `D:\04_GitHub\world-cup-issue\a-book-in-30-minutes\music\bf.mp3` in both frontend defaults and Rust backend defaults. The compatibility fallback in the video pipeline also checks `bf.mp3` directly, so newly generated videos use the processed ASCII-named background music file instead of the old Chinese filename.

`bf.mp3` is regenerated from the local performance recording at `98%` speed with `ffmpeg atempo=0.98`. The verified output is MP3, 44100 Hz, stereo, approximately `304.666` seconds, size `6,753,058` bytes. This keeps the workflow compatible with the desktop shortcut Release build and avoids generating an installer.

## 2026-07-04 0.1.128 Garbled Text Cleanup

This version tightens the text-quality contract for code, comments, logs, daily reports, and documentation. Chinese text, icons, and emoji must render normally; replacement characters, Chinese mojibake, continuous question marks, and boxed/missing symbols are treated as defects.

Changes in this pass:

- AI profile validation and AI HTTP failure messages now use readable Chinese, so configuration-page errors are actionable.
- AI text generation, Feishu message sending, material scanning, and Microsoft TTS voice-region labels no longer use garbled strings.
- Mojibake detection samples in Rust are represented with Unicode escapes or descriptive wording, so the detection rules do not themselves pollute scans.
- Historical docs and daily reports avoid embedding literal garbled samples and instead describe them as replacement-character, mojibake, or continuous-question-mark markers.

## 2026-07-04 0.1.129 GPT/Gemini 双 AI 配置

This version adds provider-specific AI configuration in the Settings page.

- The AI panel now has GPT and Gemini tabs. Existing OpenAI-compatible settings are treated as GPT; Gemini has its own name, Base URL, model, API Key, proxy switch, and proxy URL.
- `AppSettings` adds `activeAiProvider` and `geminiProfile`. Both GPT and Gemini profiles include `proxyEnabled` and `proxyUrl`; the full settings JSON is persisted in SQLite `app_settings.settings`.
- GPT defaults to no proxy. Gemini defaults to `proxyEnabled=true` and `proxyUrl=http://127.0.0.1:1080`, matching the local VPN requirement.
- The backend dispatches by `activeAiProvider`: GPT uses Chat Completions with Bearer Auth; Gemini uses Google Generative Language `generateContent` with the `X-goog-api-key` header and `contents[].parts[].text` request body.
- The top model pill, AI connection test, AI text generation test, and browser preview all read the currently selected provider profile.

## 2026-07-04 0.1.130 Explicit Pipeline AI Selector

This version adds an explicit `流水线使用 AI` selector to the Settings AI panel. The selector writes `settings.activeAiProvider`, which is the same field used by `generate_book_materials`, `test_ai_profile`, and `generate_ai_text`. The GPT/Gemini segmented control remains the detail editor for each provider, while the selector makes the pipeline choice visible and unambiguous.

## 2026-07-04 0.1.131 Pipeline Task List and Step Tracking Fix

This version removes the top stage-card strip from the Pipeline page so the task list becomes the primary progress surface.

- Selecting or typing a current EPUB and clicking Text must immediately create a visible task row, even before the next SQLite refresh.
- Backend task queries must use real SQL for category lookup, path lookup, and deletion. Literal status/error text such as `Operation completed.` must never be passed to `prepare`, `query_row`, or `execute` as SQL.
- Step Tracking must always show the full A-F step template for the current selected/requested task. If the task has not been read from SQLite yet, the page synthesizes a pending task from the current path, then replaces it with persisted task/step state when available.
- The Speech voices locale query also uses a real parameterized SQL statement, avoiding the same historical string-replacement bug.

## 2026-07-04 0.1.132 Persisted Selection and Multi Task Step Tracking

The Pipeline task checkbox selection is now part of the shared materials workbench state instead of local page state. Switching from the Pipeline page to Step Tracking keeps the checked task set; rescanning or removing tasks filters out paths that no longer exist.

Step Tracking now expands the full 17-step A-F template for every checked task. One checked task renders 17 rows, and N checked tasks render 17*N rows. When nothing is checked, the page keeps the previous single-task fallback based on the selected task, request path, running task, or first available task.

The step table includes a task column so repeated A-01 through F-01 rows remain distinguishable. Step duration display is normalized to `MM分SS.SSS秒`, including running steps calculated from `startedAt` and completed steps using persisted `elapsedMs`.

## 2026-07-04 0.1.133 Pipeline AI Save Barrier

Settings updates are saved through an asynchronous frontend queue. The Pipeline page now waits for that queue with `flushSettings()` before starting Text, Audio, Image, Subtitle, or Video work, then reloads settings from the Tauri backend before invoking AI-dependent commands.

This prevents the Settings page from visually showing Gemini while the backend still has the previous GPT snapshot. The Text stage also builds its request from the refreshed settings, so channel, language, target length, and active AI provider stay aligned with the persisted SQLite configuration.

## 2026-07-04 0.1.134 Step Persistence and Stop Task Control

This version fixes the SQLite writes that drive the Pipeline task list and Step Tracking page. Material task progress updates now execute real SQL again, and Step Tracking can query the latest trace for a path from `material_task_steps`. The generated material model and AI request log now use the active provider profile, so Gemini runs no longer display GPT model/base URL in material-generation logs.

The Pipeline panel adds a `终止任务` control. When clicked, the current frontend trace is marked as terminated, the selected/requested task is written as failed with a user-terminated message, spinner/highlight state is cleared, and late frontend responses for that trace are ignored.

Text progress display now shows the actual backend percentage instead of rounding to 0/25/50/75/100 buckets, so early parsing and AI-request progress is visible immediately.

Subtitle normalization now targets lines within 20 Chinese characters where possible. Overlong AI subtitle lines are re-split locally at semantic punctuation or the best available pause, and every written `subtitles.txt` line is guaranteed to end with punctuation. Mid-sentence forced splits receive a Chinese comma, while the final subtitle receives a full stop when needed.

## 2026-07-04 0.1.135 Material Output SQL Cleanup

This version completes the cleanup of historical placeholder SQL in material task persistence. Updating `material_output_dir`, reconciled text/image/audio/subtitle/video state, and moved output references now uses real `UPDATE material_tasks ... WHERE path = ?` statements. This keeps CLI and UI text generation from failing at the final package-save step and lets the Pipeline task list refresh from SQLite after backend work completes.

## 2026-07-04 0.1.136 Narration Source Subtitle Control

Subtitle length control now starts at narration generation. The book-materials prompt and narration rewrite prompt tell the AI to write `narration.txt` with short sentences or short semantic clauses, normally within 20 Chinese characters per rhythm unit and ending with punctuation. Long ideas should be rewritten into natural short clauses before subtitles are produced, so the subtitle file can follow the narration rhythm instead of relying on mechanical local cuts.

Local subtitle normalization no longer hard-splits overlong AI subtitle lines. It cleans blank lines and guarantees line-ending punctuation only. If generated subtitles still contain over-20-Chinese-character lines or missing punctuation, the backend asks the active AI provider to rewrite the subtitle array against the full narration, preserving coverage and avoiding word breaks such as `白血病`. If the AI rewrite cannot produce a complete valid array, the system keeps the prior safe subtitles with punctuation rather than cutting words locally.

## 2026-07-04 0.1.137 Configurable 30-35 Minute Text Target

The material-generation target remains configurable in Settings and persisted in SQLite through `materialProfile.targetMinChars` and `materialProfile.targetMaxChars`. The default target is now `7500~7800` Chinese characters, matching the desired 30-35 minute listening length. The Settings page labels these fields as the 30-35 minute text target, and the Pipeline page continues to read the latest persisted settings before Text generation.

Subtitle acceptance is stricter: AI-provided subtitle arrays must cover at least 95% of the narration's Chinese-character count. Incomplete subtitle arrays are rejected and rebuilt from the full narration, preventing `narration.txt` from meeting the target while `subtitles.txt` is thousands of characters short. The final total-character trim was also relaxed so punctuation and short subtitle rhythm do not force the 7500-7800 target back below the configured range.

## 2026-07-04 0.1.138 No Repetitive Padding and Single-Clause Subtitles

The text generator no longer pads short narration with local template paragraphs. If AI generation is below the configured target, the app asks the active AI provider for additional narration and rejects empty or repetitive extensions; if the result still misses the target, generation fails instead of producing repeated filler. This prevents the end of `narration.txt` and `subtitles.txt` from being filled with repeated generic lines.

Subtitle line validation now requires each line to be one sentence or half sentence: no more than 20 Chinese characters, ending with punctuation, and without sentence-ending punctuation such as `。？！；` in the middle of the line. Invalid AI subtitle arrays are rebuilt from the full narration, and rebuilt arrays are rejected if they still contain overlong or multi-sentence lines.

## 2026-07-04 0.1.139 CLI Length Repair Parity

The CLI `--e2e-materials` path now uses the same AI length-repair behavior as the UI generation path. If narration is shorter than the configured range, it asks AI for a non-repetitive extension; if narration is longer than the configured range, it asks AI to rewrite the full material JSON into the configured target. This keeps end-to-end validation from failing immediately on overlong AI drafts while still avoiding local repetitive padding.

## 2026-07-04 0.1.140 Near-Complete Subtitle Coverage

Subtitle coverage validation is tightened from 95% to 99.5% of narration Chinese characters. This prevents a final narration paragraph from being omitted while still passing validation.

## 2026-07-04 0.1.141 Disable Local Material Fallback

Initial AI material generation no longer falls back to local template-based material payloads. Empty or failed AI responses now fail the Text stage directly. This prevents repeated local excerpt paragraphs from being written to `narration.txt` and then propagated into `subtitles.txt`.

## 2026-07-05 0.1.144 MacMini Image Model Isolation

Formal image generation uses the MacMini image service by default through `OPENAI_IMAGE_MODE=macmini-realistic` and `MACMINI_IMAGE_ENDPOINT=http://100.96.199.26:30020/v1/images/generations` for the home-network Tailscale path. In this mode the pipeline must not inherit the text-generation model from `ABOOK_AI_MODEL`; it explicitly passes a valid image model, defaulting to `SG161222/Realistic_Vision_V5.1_noVAE`, so text models such as `gpt-5.5` are never sent to the Hugging Face image backend.

The whiteboard image skill may still have its own `.env` for standalone use. The pipeline therefore passes the image mode, endpoint, and image model in the subprocess environment so app runs override any stale standalone text-model setting.

## 2026-07-05 0.1.145 Pipeline Stage Status Isolation

The Pipeline page now treats Image, Subtitle, and Video as separate persisted stages. Starting Image uses an `image` trace id, writes `image_status=generating` with `image_progress=0`, clears the old image output path, and initializes the current trace's image step rows so Step Tracking no longer shows stale 100% rows from an earlier image run.

The Stop Task action updates only the active stage. Stopping Image marks the image stage failed without changing the Text status, narration character count, or material output directory. Backend operation logs use the current stage label, so clicking Image logs 图片流水线 instead of 视频流水线.

## 2026-07-05 0.1.152 Xiaohei Sequence Image Backend

The Image stage now defaults to `BOOK_IMAGE_BACKEND=xiaohei-sequence` instead of Qwen or MacMini Realistic Vision. This backend generates 32-64 lightweight 16:9 PNG images locally from subtitle-aligned scene groups, using the `helloianneo/ian-xiaohei-illustrations` visual contract: pure white background, black hand-drawn 小黑 character, sparse red/orange/blue Chinese annotations, and one conceptual action per image.

Each generated image is written as `visual_XX_xiaohei_sequence.png`; source images are kept in `xiaohei_sequence_images`; `xiaohei_sequence_manifest.json` and `visual_assets_manifest.json` record `sourceKind=xiaohei_sequence`, scene count, paths, short labels, preview text, and `startMs`/`endMs` coverage. Minimal-image validation uses dimensions, file size, and color count rather than the high-detail photographic checks used for AI-generated illustrations.

The Qwen Image and MacMini image paths remain available only when explicitly selected through environment variables or future configuration, but the packaged app no longer sends the default Image stage to remote heavy image models.

## 2026-07-17 0.1.164 Narration Metadata Filter And Short Sentence Rhythm

`generate_book_materials` now treats publisher/copyright-page facts as source metadata, not narration material. Source excerpts are filtered before entering the AI prompt, and generated narration plus repair extensions are sanitized again after AI responses. The sanitizer removes fragments containing publisher names, publication dates, ISBN/book numbers, edition/printing/price/copyright markers, and exact repeated sentences, so copyright-page details cannot appear twice at the opening and ending simply to fill the target length.

Narration prompting now asks for natural 8-12 Chinese-character rhythm units, with longer sentences allowed only when the meaning needs them. The subtitle splitter also prefers 8-18 character lines, uses a 10-character soft break, and rejects most lines shorter than 6 Chinese characters during AI subtitle rewrite validation, reducing two-character or four-character fragments in `subtitles.txt` and downstream TTS pacing.

## 2026-07-17 0.1.163 Y9000P Official Xiaohei Quality Pass

`xiaohei-ai-y9000p` now defaults to KaiTi Chinese labels, textless ComfyUI input guides, text-layer-only restoration, flowing hand-drawn arrows/underlines, and 1536x864 / 32-step quality-first img2img generation on the local RTX 3070 Laptop GPU. The local GNU packaging config now points at the rebuilt Y9000P user path `C:\Users\CoderDream\scoop\apps\mingw\current\bin`, and `.tooling\cargo\bin` is created before validation so release builds do not depend on the old Administrator profile. The goal is to match the official Xiaohei reference style more closely while keeping Chinese text crisp and avoiding model-generated pseudo-Chinese.

## 2026-07-06 0.1.153 Switchable Xiaohei Production Backend

`settings.pipelineProfile.imageBackend` now controls the Image stage backend. The Settings page exposes a 图片生成方案 selector with `xiaohei-production` as the default production path, while `xiaohei-sequence` remains available as a fast local fallback and `qwen-image-2512` / `whiteboard-skill` remain explicit experimental or compatibility options.

`xiaohei-ai-y9000p` is available as an explicit local AI image backend. It calls the Y9000P / 187 ComfyUI node at `http://127.0.0.1:8188` by default, now using official-style controlled img2img rather than free txt2img: the pipeline renders sparse programmatic Xiaohei guide images with KaiTi Chinese labels, fewer boxes, and curved hand-drawn lines, copies textless resized guides into `D:\AI\apps\ComfyUI\input\xiaohei_y9000p_guides`, and runs DreamShaper LCM at the quality-first default of 1536x864 / 32 steps / cfg 1.9 / denoise 0.38 so the GPU refines line texture without learning or inventing Chinese text. Because the model corrupts Chinese annotations, the final image restores only the labeled-vs-textless guide difference layer over the AI output by default. Raw ComfyUI outputs are written to `xiaohei_ai_y9000p`, final video images are resized to 1920x1080, and the manifest records `sourceKind=xiaohei_ai_y9000p`, `workflow=controlled-img2img`, `restoreGuideLineArt`, guide paths, prompts, and generation parameters. Set `Y9000P_COMFYUI_WORKFLOW=txt2img` only for the older free-generation smoke path.

`xiaohei-production` follows `docs/xiaohei-production-solution-handoff.md`: the Windows pipeline writes one JSON spec per subtitle-aligned scene, copies those specs to MacMini4, runs `/Volumes/System/AI/apps/xiaohei-local-generator/xiaohei_local_generate.py` over SSH, pulls back the 3200x1800 PNG outputs, then downsizes them to 1920x1080 `visual_XX_xiaohei_production.png` files for video assembly. The manifest records `sourceKind=xiaohei_production`, `remoteHost=macmini4`, raw image paths, final image paths, labels, preview text, and `startMs`/`endMs` coverage.

The current MacMini4 generator only implements the `trust_bridge` template, so the first end-to-end integration intentionally produces structurally consistent images with different labels. The pipeline contract is ready for future template expansion without changing the app-side backend switch.

## 2026-07-05 0.1.146 Timeline Driven Image Stage

The canonical pipeline order is now Text -> Audio -> Subtitle -> Image -> Video -> Publish. Image generation is downstream of subtitle alignment, because each image needs the final Chinese SRT timestamp range to know when it appears in the finished video.

The Image stage must read the aligned Chinese SRT and generate visual prompts from timed subtitle groups. Each generated image records its `startMs`, `endMs`, covered text, and file path in the visual manifest/timeline. Video generation consumes that timeline instead of guessing display intervals from raw subtitle text.

## 2026-07-05 0.1.147 Audio Failure Closure and Log Hygiene

This version tightens the Audio stage contract after a failed speech request.

- Polling reads such as `get_material_tasks` must not create user-visible operation-log rows. Logs should describe user actions, backend work, and failures, not the UI heartbeat.
- Audio stage UI state is persisted through the shared stage-status command. `setTaskAudioState()` writes SQLite just like Image and Subtitle, so a refresh cannot overwrite an in-flight or failed Audio status with stale local state.
- The Audio button participates in the same `currentTraceId` lock and terminate flow as the other pipeline buttons. Completion, failure, or user termination clears the trace and releases the UI lock.
- `generate_material_task_audio` writes B-01 through B-04 step records into `material_task_steps`: reading narration, splitting chunks, generating speech, and merging final audio. A speech failure marks B-03 failed with the chunk and SSML file context.
- SSML generation must produce a valid Microsoft Speech request body with `<speak>`, `<voice>`, and `<prosody>` elements. Placeholder text such as `Operation completed` is rejected locally before sending the request.

## 2026-07-05 0.1.148 Forward Pipeline Completion

Pipeline stage buttons are ordered actions, not isolated commands. Clicking a later stage must automatically complete every unfinished prerequisite in order:

- Text runs before Audio when text output is missing or configured for regeneration.
- Audio runs before Subtitle, Image, Video, and Publish when final mp3 output is missing or configured for regeneration.
- Subtitle runs before Image, Video, and Publish. When used as a prerequisite, the frontend waits for the background subtitle stage to reach success before starting the next stage.
- Image runs before Video and Publish. When used as a prerequisite, the frontend waits for the image timeline stage to reach success before starting video assembly.
- Publish first ensures Text -> Audio -> Subtitle -> Image -> Video, waiting for Video success when it had to start video generation, then generates the publish Markdown.

The only time the UI should ask the user to click an earlier button is when there is no valid selected/requested task. Missing prerequisite artifacts are handled by the pipeline itself.

## 2026-07-06 0.1.154 Staged Output Directory Contract

Each book `output` directory is now the stable task root and contains six stage folders that match the pipeline buttons: `01_content`, `02_audio`, `03_subtitles`, `04_images`, `05_video`, and `06_publish`.

The Text stage writes the material package (`materials.json`, `narration.txt`, `subtitles.txt`, title, description, tags, prompt, overview, draft SRT, README) into `01_content`. The Audio stage writes SSML, part mp3 files, the final narration mp3, and `audio_manifest.json` into `02_audio`. Subtitle alignment writes Aeneas input, Chinese SRT, bilingual SRT/ASS/LRC, translation cache, and subtitle manifests into `03_subtitles`. Image generation writes source image folders, final `visual_XX_*` PNGs, cover art, visual story plans, visual timelines, and image manifests into `04_images`. Video assembly writes `pipeline_manifest.json` and final mp4 outputs into `05_video`. Publish material generation writes `youtube_publish.md` into `06_publish`.

`material_output_dir` remains the root `output` path so the UI opens a single organized folder. Stage-specific database fields point to their stage folders or files: `audio_output_dir` to `02_audio`, `image_output_dir` to `04_images`, `subtitle_file` to `03_subtitles/...srt`, and `video_file` to `05_video/...mp4` when a video exists.

The backend keeps backward-compatible readers for older root-level outputs. Path resolution first checks the staged location, then falls back to the legacy root-level file. This allows existing tasks to keep working during migration while all new writes use the staged layout.

## 2026-07-06 0.1.155 Xiaohei Production Diversity and Subtitle Safe Area

The Image stage no longer sends the default `xiaohei-production` path to the MacMini4 single-template `trust_bridge` generator. That remote generator currently only supports one composition, which made finished videos look like the same picture repeated with different labels. By default, `xiaohei-production` now uses the app-side Ian Xiaohei multi-template renderer and records `generationMode=local_multi_template` in `xiaohei_production_manifest.json`.

The local production renderer follows the bundled `docs/ref/ian-xiaohei-illustrations` contract: white 16:9 canvas, black hand-drawn linework, small solid-black Xiaohei as the action subject, sparse red/orange/blue Chinese handwritten annotations, and one concrete metaphor per scene. The templates rotate through workflow, filter, balance, repair, route map, layers, information well, and choice/decision compositions. Each template includes concrete objects such as paper stacks, loose notes, boxes, route nodes, wells, or low-tech machines so the image reads as a content illustration rather than a repeated icon.

MacMini4 remote generation remains available only when explicitly enabled with `XIAOHEI_PRODUCTION_REMOTE=1`. The generated specs now include varied template names and `subtitleSafeBottomPx`, so the remote generator can adopt the same contract after its template library is expanded.

All Xiaohei images pass through a subtitle-safe-area transform before being saved: the illustration content is scaled into the upper part of the 1920x1080 frame and the bottom 300 pixels are left as clean white space. This keeps the hard Chinese/English subtitle overlay from covering the main drawing.

## 2026-07-06 0.1.156 Programmatic Xiaohei Object Grounding and Quality Boundary

The programmatic Xiaohei renderer now extracts concrete nouns and scene words from each subtitle group and maps them to visible objects. The mapping includes books, biographies, company signs, price tags, coins, stock tickets, medical cards, toys, houses, friendship bridges, clocks, keys, stamps, rulers, ladders, stones, lamps, and trash buckets. Each scene records these extracted `objects` plus richer handwritten `notes` in `xiaohei_production_manifest.json`, so image debugging can show which subtitle concepts were grounded into the picture.

The renderer draws these objects into the existing local multi-template compositions and expands annotation density from a few keywords to a small cluster of labels and humorous handwritten notes. This improves semantic grounding and avoids empty two-label diagrams while preserving the 300px bottom subtitle-safe area.

This version also clarifies the quality boundary of the local renderer. It is a deterministic Pillow-based line-art fallback and should be treated as a low-fidelity storyboard or emergency placeholder. It cannot reach the official `ian-xiaohei-illustrations` sample quality because those samples rely on image-model drawing ability and stronger visual composition. The intended production direction is to keep the programmatic renderer as `xiaohei-programmatic` fallback and route the default `xiaohei-production` path to a real image-generation backend that follows the same Xiaohei prompt contract and then applies the subtitle-safe-area postprocess.

## 2026-07-08 0.1.157 Reinstalled Windows Release Verification

This version is a release-only verification build after the local Windows operating system was reinstalled.

No product workflow or UI behavior is changed in this version. The purpose is to confirm the restored local environment can still run the fixed GNU Rust/Tauri build flow and produce the desktop shortcut release exe.

The expected artifact remains `a-book-in-30-minutes/src-tauri/target/x86_64-pc-windows-gnu/release/a_book_in_30_minutes.exe`. Installer generation is intentionally skipped unless explicitly requested.

## 2026-07-08 0.1.158 Reinstalled Windows MinGW Path Recovery

After the Y9000P Windows reinstall, the old Administrator Scoop MinGW path no longer exists. The GNU build contract now points Cargo and the packaging helper script at the verified current MinGW toolchain under `C:/Users/CoderDream/scoop/apps/mingw/current/bin`.

The Tauri release build continues to use `x86_64-pc-windows-gnu` and the repo-local Rust toolchain under `.tooling/rustup`. This keeps the desktop shortcut exe build independent from MSVC and avoids the missing `link.exe` path.

## 2026-07-08 0.1.159 Scoop MinGW Release Path Restore

After the Y9000P reinstall, the release build contract was updated from the old Administrator profile to the current Scoop MinGW location under `C:/Users/CoderDream/scoop/apps/mingw/current/bin`.

Cargo uses the restored Scoop `x86_64-w64-mingw32-gcc.exe` as the GNU linker and the available `ar.exe` from the same directory for archiving. The packaging helper script also places the Scoop MinGW `bin` directory before the repo-local Rust toolchain in `PATH`, matching the previously verified build order.

## 2026-07-08 0.1.160 AI Streaming Error Visibility

The Text stage now preserves the original AI request error in the task message and A-02 step detail instead of converting every request failure into a generic empty-response message. This makes Step Tracking show the actionable failure reason directly.

OpenAI-compatible streaming parsing now tolerates non-content `data:` chunks that do not contain `choices`, such as provider metadata or heartbeat packets. If a streaming chunk contains an `error` object, the backend reports that error explicitly; malformed chunks include a short preview in the error message for log diagnosis.

## 2026-07-08 0.1.161 Text Stage Lock Release

The Text pipeline button now releases the frontend run lock after success, failure, or a skipped no-op. `generateMaterials()` clears `currentTraceId`, sets `busy=false`, resets the active pipeline stage, and reloads stored task rows after completion.

This keeps the Stop Task button hidden once the Text stage has completed and restores Audio/Subtitle/Image/Video/Publish button availability for the selected task.

## 2026-07-18 0.1.165 Text Content Gate and Local Convergence

The Text stage now validates usable正文 content after EPUB parsing. Publisher, date, ISBN, and copyright-page fragments are excluded from this count; sources below 200 Chinese content characters fail at A-01 with a clear message instead of spending an AI request inventing a full narration from metadata.

Narration repair now uses a deterministic local trim when the result is only slightly above the configured maximum (within 360 Chinese characters). This avoids sending the complete narration back to the model repeatedly for small length differences. Larger deviations still use the existing AI repair path and remain subject to the configured range check.

Subtitle segmentation remains local by default. The expensive whole-narration AI subtitle rewrite is disabled unless the process environment explicitly sets `ABOOK_ENABLE_AI_SUBTITLE_REWRITE=1` or `true`; this keeps normal Text generation predictable and fast while retaining an opt-in fallback for unusual source rhythm.

## 2026-07-18 0.1.166 ffmpeg Tool Path Defaults

The Tool Paths section now uses the Y9000P machine's verified ffmpeg executable at `D:/03_Dev/ffmpeg/bin/ffmpeg.exe` by default. The Rust persisted-settings sanitizer migrates an existing empty `ffmpegPath` to this path, while the frontend default keeps fresh local settings consistent. The Settings page explains the default and retains the file picker and explicit ffmpeg test button.

## 2026-07-19 0.1.169 Image Model Settings Tab

The Configuration page now has a separate `图像模型` tab beside `基础配置`. It persists a local ComfyUI profile for the Y9000P/187 node: base URL, checkpoint, workflow mode, output directory, dimensions, steps, and CFG. The tab can start and stop the D-drive ComfyUI service, refresh status, validate the checkpoint and service, and submit a text prompt for a local test image. Generated test images are saved under `D:/AI/outputs/ComfyUI/ui-tests` and previewed in the page.

The existing pipeline image backend selector remains the source of truth for production image generation. Choosing `小黑 AI（本机 187）` continues to route the Image stage to the existing controlled Xiaohei ComfyUI pipeline, while the new tab provides the same local node with an explicit, inspectable configuration and smoke-test workflow.

## 2026-07-19 0.1.170 ComfyUI Startup Readiness

The local image model Start action now waits up to 60 seconds for ComfyUI `/system_stats` to become reachable after launching the D-drive process. This covers first-load Torch/CUDA/model initialization and avoids showing a transient connection error after a successful start request. If the service still needs more time, the page reports that the model is loading and the user can refresh status.

## 2026-07-19 0.1.171 Image Model Status Race Fix

The Configuration page no longer schedules a second status request 2.5 seconds after clicking Start. The Rust command owns the readiness wait and returns the authoritative ComfyUI status after `/system_stats` is reachable, so a transient request during Torch/CUDA initialization cannot overwrite a successful status with `无法连接 ComfyUI`. Manual Refresh remains available for services started outside the application.

## 2026-07-19 0.1.172 Controlled Configuration Test

The Configuration page test action now uses the same controlled img2img shape as the verified local Xiaohei route. It copies the verified D-drive official-style guide into ComfyUI input, encodes it with `VAEEncode`, runs LCM KSampler with the configured denoise clamped to `0.25~0.50`, and saves the result through a dedicated controlled output node. This prevents the test action from silently falling back to free txt2img with `denoise=1.0`, which produced oversized abstract black shapes and photographic hands.

## 2026-07-18 0.1.167 Azure Speech Proxy

Azure Speech synthesis now uses Speech Profile proxy settings for preview, test, and audio chunk requests. Existing settings files receive compatible defaults `proxyEnabled=true` and `proxyUrl=http://127.0.0.1:1080`; the Settings page exposes the URL and toggle so deployments without the local proxy can disable it.

## 2026-07-18 0.1.168 Subtitle Python Compatibility

The bundled `book_video_pipeline.py` now enables postponed annotation evaluation so the pipeline can start under the machine's Python 3.9 runtime while retaining its modern type annotations. Pillow must be installed in the Python interpreter selected by the application (`python -m pip install Pillow`).



## 2026-07-19 0.1.173 Performance Mode Before Image Generation

Before a local image test is submitted to ComfyUI, the Rust command now runs the repository's D-drive Y9000P performance-mode script. A failed or missing script stops generation with an explicit error; a successful run records an `image_model.performance_mode` operation log entry. This ensures local GPU generation does not silently run under a balanced Windows power plan.

## 2026-07-19 0.1.174 Local Xiaohei Backend Default

The default production image backend is now `xiaohei-ai-y9000p`, the local RTX 3070 controlled img2img route. Persisted settings that still contain the former default `xiaohei-production` are migrated to the local backend so an existing fresh install does not silently keep producing the fixed Pillow flowchart templates. The old production template remains available only when explicitly selected.

## 2026-07-19 0.1.175 Structure-First Quality Preset

The local image model defaults now use the quality preset `1536x864`, `32` LCM steps, `cfg 1.9`, and `denoise 0.38`. The generation contract is structure-first: the program creates the composition and short Chinese annotation layer, ComfyUI receives a textless guide for controlled img2img refinement, and the final text layer is restored after generation. This separates layout/Chinese QA from model texture so the RTX 3070 improves hand-drawn detail without changing the intended scene.

## 2026-07-19 0.1.176 Image Prompt Operation Logging

The local image test now writes a dedicated `image_model.prompt` operation log entry before queuing ComfyUI. The entry includes the checkpoint, workflow, Guide path, dimensions, steps, CFG, denoise, full positive prompt, and negative prompt, making a generated result auditable instead of only reporting its output path.

The formal Y9000P image pipeline now uses the installed SD1.5 Lineart ControlNet when running controlled Guide img2img. Its default strength is `0.40` with an `0.85` end percent, keeping composition stable while allowing line texture refinement. The positive prompt emphasizes fine varied ink strokes, relaxed hand-drawn curves, natural imperfect pen texture, and detailed uncluttered objects while explicitly discouraging rigid geometric diagram lines. The final Guide restoration layer also supports optional 3px MaxFilter thinning through `Y9000P_COMFYUI_LINE_THIN_RADIUS`. The reusable `scripts/generate-direct-press-guide.py` provides a higher-detail press benchmark for visual QA; it is a test Guide generator, not a replacement for scene-specific Guides.
