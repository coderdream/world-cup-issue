# 小黑读书视频插图本地生产方案交接文档

## 目标

为读书视频批量生成 Ian 小黑风格中文正文配图。

目标图像风格：

- 16:9 横版。
- 纯白背景。
- 黑色手绘线稿。
- 小黑作为核心动作主体。
- 少量红色、橙色、蓝色中文手写标注。
- 大量留白。
- 结构清楚但不像 PPT，不像正式流程图。
- 可直接进入视频生产，中文必须准确，布局必须稳定。

这份文档用于交给另一个 Codex 对话继续实现完整工具链。

## 最终结论

已经测试过三条模型路线：

| 方案 | 状态 | 是否适合作为生产主链路 | 结论 |
| --- | --- | --- | --- |
| SD 1.5 / DreamShaper LCM | 已在 MacMini4 跑通 | 否 | 能出图，但风格、结构、中文标注不可控 |
| ComfyUI + MPS | 已在 MacMini4 API 跑通 | 否 | 能自动化出图，但结果偏 3D/插画，中文和结构不稳定 |
| Draw Things | 已安装 | 否 | 适合人工挑图和试模型，不适合无人值守批量生产 |

真正可作为生产主链路的是：

```text
ian-xiaohei-illustrations Skill
  -> 结构/主题/中文标注 JSON
  -> MacMini4 本地 Pillow 结构化绘图生成器
  -> 1600x900 预览图 / 3200x1800 生产图
```

原因：

- 速度稳定：单图约 0.25s - 0.30s。
- 中文稳定：文字由程序绘制，不让扩散模型生成中文。
- 布局稳定：结构由模板控制，可批量复现。
- 成本稳定：MacMini4 本地执行，不依赖外部图像 API。
- 可迭代：后续可逐步增加模板，而不是赌模型一次出对。

## 当前 MacMini4 状态

远程主机：

```text
Host: macmini4
IP: 100.96.199.26
User: coderdream
SSH: ssh macmini4
```

外网下载必须使用 MacMini4 上的 Hysteria2 代理：

```text
SOCKS: 127.0.0.1:1080
HTTP: 127.0.0.1:8080
```

Skill 路径：

```text
/Users/coderdream/.codex/skills/ian-xiaohei-illustrations
```

本地生成器路径：

```text
/Volumes/System/AI/apps/xiaohei-local-generator/xiaohei_local_generate.py
```

示例 spec：

```text
/Volumes/System/AI/apps/xiaohei-local-generator/examples/trust_bridge_spec.json
```

输出目录：

```text
/Volumes/System/AI/outputs/xiaohei-local
```

当前生成器也在 Windows 工作区有一份：

```text
D:\0030_codex\MacMini4\tools\xiaohei_local_generate.py
D:\0030_codex\MacMini4\examples\trust_bridge_spec.json
```

## 已验证产物

1x 预览图：

```text
/Volumes/System/AI/outputs/xiaohei-local/trust-bridge-smooth-1x.png
D:\0030_codex\MacMini4\outputs\remote\trust-bridge-smooth-1x.png
```

2x 生产图：

```text
/Volumes/System/AI/outputs/xiaohei-local/trust-bridge-smooth-2x.png
D:\0030_codex\MacMini4\outputs\remote\trust-bridge-smooth-2x.png
```

2x 版本尺寸：

```text
3200x1800
```

2x 版本用于视频合成时缩放到 1080p/720p，可减少锯齿。

## 当前生成命令

生成 1600x900 预览图：

```bash
/Volumes/System/AI/apps/ComfyUI/venv/bin/python \
  /Volumes/System/AI/apps/xiaohei-local-generator/xiaohei_local_generate.py \
  --spec /Volumes/System/AI/apps/xiaohei-local-generator/examples/trust_bridge_spec.json \
  --out /Volumes/System/AI/outputs/xiaohei-local/trust-bridge-smooth-1x.png \
  --output-scale 1
```

生成 3200x1800 生产图：

```bash
/Volumes/System/AI/apps/ComfyUI/venv/bin/python \
  /Volumes/System/AI/apps/xiaohei-local-generator/xiaohei_local_generate.py \
  --spec /Volumes/System/AI/apps/xiaohei-local-generator/examples/trust_bridge_spec.json \
  --out /Volumes/System/AI/outputs/xiaohei-local/trust-bridge-smooth-2x.png \
  --output-scale 2
```

从 Windows 拉回：

```powershell
scp macmini4:/Volumes/System/AI/outputs/xiaohei-local/trust-bridge-smooth-2x.png outputs\remote\trust-bridge-smooth-2x.png
```

## 当前 JSON spec 格式

示例：

```json
{
  "template": "trust_bridge",
  "seed": 7,
  "title": "信任桥",
  "labels": {
    "before": "写之前",
    "after": "写完之后",
    "nope": "不是哦",
    "stuck": "素材枯了",
    "missing": "没承接",
    "content": "内容"
  }
}
```

当前只实现了一个模板：

```text
trust_bridge
```

## 已完成的关键技术点

### 1. 抗锯齿

生成器已经从低分辨率直接绘制改为：

```text
3x 内部画布绘制
-> 轻微 GaussianBlur
-> Lanczos 降采样
-> 输出 1x 或 2x PNG
```

这解决了用户反馈的“太粗糙、锯齿明显”问题。

### 2. 中文可控

中文不交给 SD/ComfyUI/Draw Things 生成，而是由 Pillow 使用系统字体绘制。

这样可以保证：

- 不错字。
- 不乱码。
- 标注位置可控。
- 可以批量替换。

### 3. 图像结构可控

小黑、传送带、纸张、箭头、洞、内容块等元素由模板函数绘制。

这比扩散模型更适合读书视频配图，因为读书视频需要表达稳定的概念结构，而不是只追求画面随机美感。

## 另一个 Codex 的下一步任务

### 任务 1：整理代码为正式项目结构

建议目标结构：

```text
/Volumes/System/AI/apps/xiaohei-local-generator/
  README.md
  xiaohei/
    __init__.py
    cli.py
    canvas.py
    templates/
      trust_bridge.py
      common.py
  examples/
    trust_bridge_spec.json
  outputs/
```

Windows 工作区同步结构：

```text
D:\0030_codex\MacMini4\tools\xiaohei_local_generate.py
D:\0030_codex\MacMini4\docs\xiaohei-production-solution-handoff.md
```

### 任务 2：把模板能力抽象出来

当前 `trust_bridge` 写在单文件里。下一步要抽象：

- `Canvas`：线条、箭头、文字、纸张、小黑、传送带等基础绘图 API。
- `Template`：每个配图结构一个模板。
- `Spec`：统一 JSON schema。

建议通用 spec：

```json
{
  "template": "trust_bridge",
  "slug": "trust-bridge",
  "title": "信任桥",
  "seed": 7,
  "output_scale": 2,
  "labels": {},
  "elements": {},
  "style": {
    "line": "handdrawn",
    "density": "sparse"
  }
}
```

### 任务 3：补 5-8 个生产模板

优先模板：

1. `trust_bridge`：信任桥/承接断点。
2. `two_breakpoints`：两个断点。
3. `handoff_path`：承接路径。
4. `minimum_loop`：最小闭环。
5. `sort_by_purpose`：按目的分拣。
6. `information_well`：信息井。
7. `idea_press`：观点压榨机。
8. `content_fermentation`：内容发酵。

模板可以参考 Skill 示例图，但不要照抄构图；核心是让小黑参与动作。

### 任务 4：实现文章到 shot list/spec 的接口

输入可以是 Markdown：

```text
article.md
```

输出：

```text
illustrations/
  shot-list.md
  01-trust-bridge.json
  01-trust-bridge.png
  02-minimum-loop.json
  02-minimum-loop.png
```

第一版可以不接 LLM，只手写 JSON。

第二版再接 `ian-xiaohei-illustrations` Skill，让 Codex 从文章生成 JSON specs。

### 任务 5：实现批量 CLI

建议命令：

```bash
xiaohei-generate \
  --spec-dir /path/to/specs \
  --out-dir /path/to/images \
  --output-scale 2
```

单张命令：

```bash
xiaohei-generate \
  --spec 01-trust-bridge.json \
  --out 01-trust-bridge.png \
  --output-scale 2
```

### 任务 6：加入 QA 检查

至少自动检查：

- 图片尺寸是否为 `1600x900` 或 `3200x1800`。
- 背景四角是否接近白色。
- 输出文件是否大于合理阈值，例如 `50KB`。
- 中文标签是否都出现在 spec 中。
- 不允许生成纯黑图、空图。

人工 QA 检查：

- 小黑是否参与核心动作。
- 画面是否有足够留白。
- 是否像白纸手绘，不像 PPT。
- 中文是否可读。
- 是否适合直接放进视频。

### 任务 7：输出使用文档

最终 README 至少包含：

- 安装说明。
- MacMini4 远程生成命令。
- JSON spec 格式。
- 批量生成示例。
- 如何新增模板。
- 如何把输出用于读书视频。

## 验收标准

另一个 Codex 完成后，需要满足：

1. 在 MacMini4 上一条命令生成 3200x1800 PNG。
2. 单图生成时间小于 1 秒。
3. 中文标注准确，不依赖模型写字。
4. 至少支持 5 个模板。
5. 能批量处理一个 spec 目录。
6. 输出文件命名稳定。
7. 生成图边缘平滑，无明显锯齿。
8. 不启动 Qwen，不依赖 ComfyUI 常驻服务。
9. 保留 SD1.5/ComfyUI/Draw Things 作为探索方案说明，但不作为主链路。

## 需要避免的坑

- 不要再把 Qwen Image 作为主路线，用户已明确表示效率太低。
- 不要让扩散模型直接生成中文标注。
- 不要依赖 Draw Things GUI 做批量自动化。
- 不要让 ComfyUI 常驻占用 8188，除非用户明确要继续调模型。
- 不要只生成 1600x900 低清图；生产默认应输出 3200x1800。
- 不要把图做成 PPT 流程图，要保持白纸手绘和怪诞感。

## 给另一个 Codex 的启动提示词

可以把下面这段直接发给另一个 Codex：

```text
请在 D:\0030_codex\MacMini4 工作区继续实现“小黑读书视频插图本地生产工具”。先读取 AGENTS.md 和 docs/xiaohei-production-solution-handoff.md。目标是把当前 tools/xiaohei_local_generate.py 重构成可维护项目，并在 MacMini4 上形成可批量生成 3200x1800 PNG 的 CLI。不要继续 Qwen 路线，不要依赖 ComfyUI 常驻服务；SD1.5/ComfyUI/Draw Things 只保留为探索说明。生产主链路是 ian-xiaohei-illustrations Skill 生成结构/spec，Pillow 本地结构化绘图输出最终图片。请完成至少 5 个模板、批量 spec 目录生成、基础 QA 检查和 README。
```

