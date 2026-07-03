# 图片生成器

## 前置条件

默认可使用 OpenAI 兼容网关三件套；如果要生成《山茶的情书》这类读书视频配图，优先使用 MacMini4 本地 Realistic Vision 图片服务。

### MacMini4 Realistic Vision 模式

公司局域网优先使用：

```env
OPENAI_IMAGE_MODE=macmini-realistic
MACMINI_IMAGE_ENDPOINT=http://192.168.1.9:30020/v1/images/generations
OPENAI_IMAGE_MODEL=SG161222/Realistic_Vision_V5.1_noVAE
OPENAI_IMAGE_CONCURRENCY=1
BOOK_IMAGE_PROMPT_STYLE=book-realistic
```

如果读书视频更需要内容贴合和角色一致性，而不是真人照片，可改用统一插画风：

```env
BOOK_IMAGE_PROMPT_STYLE=book-illustration
```

该模式会生成“日本故事书 / 2D 动画电影 still / 水彩质感”的提示词，重点表达书中主题、信件、母女关系、旧信、离别和自我修复，并减少真人摄影感。

在家里或跨网段时再切回 Tailscale：

```env
MACMINI_IMAGE_ENDPOINT=http://100.96.199.26:30020/v1/images/generations
```

`macmini-realistic` 模式不会拼接白板 prompt；脚本会直接调用 MacMini4 Images API，并自动传入 negative prompt、steps、guidance 和固定 seed 序列。当前 MacMini4 16GB 内存下，Realistic Vision 生成 768x432 单图约 40 秒，再由后续流程按需处理。

### OpenAI 兼容网关模式

如需使用 OpenAI 兼容网关，必须在 skill 目录的 `.env` 文件中设置 OpenAI 兼容网关三件套，或使用等价环境变量：

```env
url=http://81.68.73.15:3000/openai/v1
model=gpt-5.5
key=cr_xxx
```

等价环境变量为 `OPENAI_API_BASE`/`OPENAI_BASE_URL`、`OPENAI_IMAGE_MODEL`/`OPENAI_MODEL`、`OPENAI_API_KEY`/`CODEX_API_KEY`。

如需通过本机代理访问网关，可在 `.env` 增加：

```env
proxy=127.0.0.1:1080
```

也可使用 `HTTP_PROXY`、`HTTPS_PROXY` 或 `ALL_PROXY` 环境变量。

如果网关能通但 `/images/generations` 返回 404，或者 `/chat/completions` 只回 `data: [DONE]`，这说明中转平台还没做到完整 OpenAI 兼容，不是本机 VPN 问题。脚本会继续走 chat JSON fallback，最后再退回本地白板渲染。

## 用法

运行内置脚本：

```bash
python3 <skill-dir>/scripts/generate-image.py "<提示词>" "<宽高比>" "<输出目录>"
```

**注意**：`<skill-dir>` 是 `whiteboard-video-workflow` skill 的绝对路径，由主 agent 在 subagent 指令中提供。

**参数：**
1. `prompt`（必填）— 图片生成提示词。支持两种模式：
   - **单张模式**：传入普通字符串，如 `"一只猫坐在窗台上"`。
   - **批量模式**：传入 JSON 编码的字符串数组，如 `'["提示词1","提示词2","提示词3"]'`。每个数组元素对应一张图片，脚本会以 10 个并发同时生成。
2. `aspect-ratio`（可选，默认值：`"16:9"`）— 图片宽高比（如 `"1:1"`、`"9:16"`、`"16:9"`、`"4:3"`）。
3. `output-dir`（可选，默认值：当前工作目录）— 生成图片的保存目录。

**示例：**

单张生成：
```bash
python3 <skill-dir>/scripts/generate-image.py "一只猫坐在窗台上，夕阳西下" "16:9" "./output"
```

批量生成：
```bash
python3 <skill-dir>/scripts/generate-image.py '["一只猫坐在窗台上","一只狗在草地上奔跑","日落时分的海边"]' "16:9" "./output"
```

## 工作流程

1. 验证 `prompt` 不为空。如果缺失，询问用户。
2. 检测 `prompt` 是否为 JSON 数组格式，自动区分单张/批量模式。
3. 使用三个参数运行 `scripts/generate-image.py`。
4. 脚本会自动处理：
   - `macmini-realistic` 模式：调用 `MACMINI_IMAGE_ENDPOINT`
   - 其他模式：调用配置的 OpenAI 兼容 Images API：`<url>/images/generations`
   - 使用配置的 `model` 作为图片生成模型
   - 对限流、网络和服务端错误自动重试
   - 保存返回的 base64 图片或下载返回的图片 URL，文件名基于时间戳命名（批量模式下文件名会附加序号后缀）
   - **批量模式**：默认以 3 个并发 worker 同时执行生成任务，可用 `OPENAI_IMAGE_CONCURRENCY` 覆盖
5. 向用户报告保存的文件路径。

## 批量模式说明

- 当 `prompt` 参数是 JSON 字符串数组时自动进入批量模式
- 并发数默认 3，即同时最多运行 3 个生成任务；可用环境变量 `OPENAI_IMAGE_CONCURRENCY` 调整
- 每张图片独立处理，单张失败不影响其他图片
- 输出文件名格式：`img_<timestamp>_<序号>.<ext>`（如 `img_1714700000000_01.jpg`）
- 执行结束后会输出汇总信息：成功数和失败数
- 脚本输出的最后一行以 `__RESULTS__` 前缀加上 JSON 数组，包含每张图片的保存路径或错误信息

## 资源文件

- `scripts/generate-image.py` — 独立的 Python 脚本，调用 OpenAI 兼容 Images API，支持单张和批量并发模式
