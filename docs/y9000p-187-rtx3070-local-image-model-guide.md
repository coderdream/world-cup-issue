# Y9000P 187 RTX 3070 本地图像模型操作文档

## 目标

把 187 / Y9000P 修成可稳定运行本地图像模型的 Windows 生图节点，用于《A Book in 30 Minutes》后续小黑读书视频插图生产。

优先路线：

```text
修复 RTX 3070 Laptop GPU 驱动
-> 验证 nvidia-smi / CUDA
-> 安装 Python / Git / ComfyUI
-> 下载轻量 SD / SDXL 模型
-> 生成 2 张芒格传样张
-> 再接入 a-book-in-30-minutes 图片阶段
```

## 当前机器信息

187 当前访问信息以 `D:\0030_codex\tools\ssh-免密登录排障与复用指南.md` 为准：

```text
机器：Y9000P / 187
Windows 主机名：DESKTOP-MOIQTV4
局域网地址：192.168.1.187
Tailscale 地址：100.108.139.47
SSH 用户：Administrator
SSH 别名：win-y9000p-187 / win-y9000p-187-ts
```

远程验证命令：

```powershell
ssh -o BatchMode=yes win-y9000p-187 "hostname & whoami"
ssh -o BatchMode=yes win-y9000p-187-ts "hostname & whoami"
```

## 为什么选 187

MacMini4 能跑一些图像工具，但它更适合做调度、文件管理、自动化和轻量样张。真正要稳定做本地 AI 生图，187 如果 RTX 3070 Laptop GPU 修好，会更合适：

- CUDA 生态更成熟，ComfyUI、SD WebUI、PyTorch 支持更直接。
- RTX 3070 Laptop GPU 通常有 8GB 显存，适合 SD 1.5、LCM、Turbo、部分轻量 SDXL 工作流。
- 后续可以通过 SSH / HTTP API 让 Windows 主机作为生图节点，不必让 MacMini4 硬扛模型。

## 第一步：本机修复显卡驱动

请在 187 / Y9000P 本机操作，建议先接电源，并切到独显或混合显卡模式。

### 1. 检查设备管理器

打开：

```text
设备管理器 -> 显示适配器 -> NVIDIA GeForce RTX 3070 Laptop GPU
```

如果看到黄色感叹号、设备状态 Error、代码 43、代码 31、代码 10，说明驱动需要重装或系统显卡模式有问题。

### 2. 下载官方驱动

推荐优先使用 NVIDIA 官方驱动：

```text
NVIDIA App / GeForce Experience
或 NVIDIA 官网 Notebook Driver
显卡：GeForce RTX 3070 Laptop GPU
系统：Windows 10/11 64-bit
```

安装时选择：

```text
自定义安装 -> 执行清洁安装
```

如果官方驱动无法修复，再用联想官网的 Y9000P 对应机型显卡驱动。

### 3. 必要时使用 DDU 干净卸载

如果反复安装仍然 Error，建议：

1. 下载 DDU。
2. 进入 Windows 安全模式。
3. 用 DDU 清理 NVIDIA 显卡驱动。
4. 正常重启。
5. 重新安装 NVIDIA 官方 Notebook 驱动。

注意：DDU 是强清理工具，操作前关闭正在运行的任务，必要时先创建系统还原点。

## 第二步：验证 RTX / CUDA

驱动装好后，在 187 本机 PowerShell 执行：

```powershell
nvidia-smi
```

正常输出应该包含：

```text
NVIDIA GeForce RTX 3070 Laptop GPU
Driver Version
CUDA Version
Memory-Usage
```

然后从当前机器远程验证：

```powershell
ssh win-y9000p-187 "nvidia-smi"
ssh win-y9000p-187-ts "nvidia-smi"
```

如果 `nvidia-smi` 仍报错：

```text
NVIDIA-SMI has failed because it couldn't communicate with the NVIDIA driver
```

说明驱动仍未恢复，先不要安装 ComfyUI。

## 第三步：准备工作目录

建议在 187 上统一放到 D 盘：

```text
D:\AI
D:\AI\apps
D:\AI\models
D:\AI\outputs
D:\AI\workflows
```

PowerShell：

```powershell
New-Item -ItemType Directory -Force D:\AI\apps,D:\AI\models,D:\AI\outputs,D:\AI\workflows
```

## 第四步：安装基础工具

建议安装：

```text
Git for Windows
Python 3.10.x 或 3.11.x
```

安装 Python 时勾选：

```text
Add python.exe to PATH
```

验证：

```powershell
git --version
python --version
pip --version
```

## 第五步：安装 ComfyUI

在 187 本机 PowerShell：

```powershell
cd D:\AI\apps
git clone https://github.com/comfyanonymous/ComfyUI.git
cd D:\AI\apps\ComfyUI
python -m venv venv
.\venv\Scripts\activate
python -m pip install --upgrade pip
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121
pip install -r requirements.txt
```

验证 PyTorch 能看到 CUDA：

```powershell
python -c "import torch; print(torch.__version__); print(torch.cuda.is_available()); print(torch.cuda.get_device_name(0) if torch.cuda.is_available() else 'NO_CUDA')"
```

期望输出：

```text
True
NVIDIA GeForce RTX 3070 Laptop GPU
```

## 第六步：准备模型

先不要上太大的模型。为了验证链路，建议先用以下类型：

```text
SD 1.5 插画模型
LCM / Turbo 类快速模型
轻量小黑风格 LoRA，后续再训练或挑选
```

模型目录：

```text
D:\AI\apps\ComfyUI\models\checkpoints
D:\AI\apps\ComfyUI\models\loras
D:\AI\apps\ComfyUI\models\vae
```

第一轮目标不是追求最终风格，而是确认：

- 187 能稳定出图。
- API 能远程调用。
- 显存不会爆。
- 2 张样张能在可接受时间内完成。

## 第七步：启动 ComfyUI

本机测试：

```powershell
cd D:\AI\apps\ComfyUI
.\venv\Scripts\activate
python main.py --listen 0.0.0.0 --port 8188
```

浏览器打开：

```text
http://127.0.0.1:8188
```

从当前 Windows 机器验证：

```powershell
curl http://192.168.1.187:8188/system_stats
curl http://100.108.139.47:8188/system_stats
```

如果局域网不通，检查 187 Windows 防火墙是否允许 Python / 8188 端口。

## 第八步：生成 2 张芒格传样张

样张建议用《芒格传》字幕中的两段内容：

```text
样张 1：芒格不愿意被情绪牵着走，喜欢先把情况搞清楚，然后再下注。
样张 2：芒格说好主意会让他像小鳟鱼一样活蹦乱跳，一个冷静的人忽然像孩子。
```

小黑风格提示词方向：

```text
clean white background, black ink line art, chinese editorial illustration,
Ian Xiaohei inspired composition, funny visual metaphor, expressive little black character,
investment thinking, rational decision, books, checklist, magnifying glass, tiny casino chip,
large blank lower area for Chinese subtitles, no realistic photo, no 3d render
```

反向提示词：

```text
photorealistic, 3d, oil painting, dense background, messy text, watermark,
logo, bad hands, extra fingers, horror, dark scene, cluttered layout
```

画面要求：

```text
16:9
白底
底部至少留 20% 空白给硬字幕
不要让模型生成中文正文
可以生成少量图形符号，但中文标注后续由程序叠加
```

## 第九步：交给 Codex 远程验收

你修好驱动并启动 ComfyUI 后，告诉 Codex：

```text
187 已修好，ComfyUI 已启动，端口 8188。
```

Codex 后续要验证：

```powershell
ssh win-y9000p-187 "nvidia-smi"
curl http://192.168.1.187:8188/system_stats
curl http://100.108.139.47:8188/system_stats
```

然后继续做：

```text
1. 远程提交 ComfyUI 工作流。
2. 生成 2 张芒格传样张。
3. 拉回到 D:\books\0701新书四本\芒格传\output\04_images\_samples。
4. 对比 MacMini4 / 程序化小黑图效果。
5. 决定是否把 187 作为 xiaohei-ai-production 后端。
```

## 常见问题

### nvidia-smi 不存在

通常是驱动没装好，或 NVIDIA 安装目录没有进入 PATH。先从开始菜单搜索 NVIDIA 控制面板；如果控制面板都没有，重装驱动。

### nvidia-smi 能看到显卡，但 torch.cuda.is_available() 是 False

通常是 PyTorch 装成 CPU 版本。重新安装 CUDA 版：

```powershell
pip uninstall -y torch torchvision torchaudio
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121
```

### ComfyUI 本机能打开，远程打不开

检查启动参数是否有：

```text
--listen 0.0.0.0
```

然后检查 Windows 防火墙是否放行 8188。

### 8GB 显存不够

先用 SD 1.5、LCM、Turbo、小尺寸测试。建议参数：

```text
分辨率：1024x576 或 1280x720
步数：6-16
batch size：1
```

不要一开始就跑大 SDXL 工作流。

## 结论

这条路线值得走。187 的 RTX 3070 Laptop GPU 一旦恢复，应该优先作为本地图像模型节点；MacMini4 继续做调度、文档、轻量样张和自动化辅助。最终目标是让《A Book in 30 Minutes》的图片阶段可以在配置里切换：

```text
xiaohei-programmatic：程序化兜底，稳定但画风上限低
xiaohei-ai-y9000p：187 本地 AI 生图，主力生产候选
midjourney：外部高质量备选
```

## 2026-07-08 本机 187 落地记录

当前机器就是 187 / Y9000P；重装系统后的主机名为 `Y9000P-23`。本轮已经按 D 盘优先原则恢复并验证基础 ComfyUI 节点：

```text
D:\AI\runtimes\Python310
D:\AI\apps\ComfyUI
D:\AI\apps\ComfyUI\venv
D:\AI\apps\ComfyUI\models\checkpoints\v1-5-pruned-emaonly.safetensors
D:\AI\outputs\ComfyUI
D:\AI\logs
```

外网下载必须走本机 VPN 代理：

```powershell
$env:HTTP_PROXY='http://127.0.0.1:1080'
$env:HTTPS_PROXY='http://127.0.0.1:1080'
$env:ALL_PROXY='socks5://127.0.0.1:1080'
```

仓库已新增辅助脚本：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\update-y9000p-comfyui.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\start-y9000p-comfyui.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-y9000p-comfyui.ps1
D:\AI\apps\ComfyUI\venv\Scripts\python.exe scripts\generate-y9000p-munger-samples.py
```

已验证：

```text
nvidia-smi 正常识别 NVIDIA GeForce RTX 3070 Laptop GPU
Driver Version: 546.30
CUDA Version: 12.3
PyTorch: 2.5.1+cu121
ComfyUI: 0.27.0
ComfyUI API: http://127.0.0.1:8188/system_stats
```

本机 GPU 性能模式已做 Windows 侧配置：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\set-y9000p-performance-mode.ps1
```

该脚本会切到高性能/卓越性能电源计划，AC 供电下设置 CPU 最小/最大性能 100%、主动散热、关闭 PCIe 链路省电，并把 `D:\AI\apps\ComfyUI\venv\Scripts\python.exe` 标记为 Windows 高性能 GPU 偏好。当前 NVIDIA 驱动在 RTX 3070 Laptop GPU / WDDM 平台下不支持 `nvidia-smi -pm 1` 持久模式，也不支持 `nvidia-smi -pl 150` 命令行修改功耗上限；这是笔记本平台限制。要获得最高持续性能，还需要保持接电，并在联想 Vantage / Legion 或 Fn+Q 中切到性能模式。

因为新系统缺少 `VCOMP140.DLL`，已从 D 盘驱动备份复制到 ComfyUI venv 的 `torch\lib` 和 `torchaudio\lib`，避免安装系统级运行库。

两张芒格传 smoke test 样张已生成到：

```text
D:\books\0701新书四本\芒格传\output\04_images\_samples\munger_sample_01.png
D:\books\0701新书四本\芒格传\output\04_images\_samples\munger_sample_02.png
D:\books\0701新书四本\芒格传\output\04_images\_samples\munger_samples_manifest.json
```

当前样张只证明本机 RTX + ComfyUI + API 出图链路打通。`v1-5-pruned-emaonly.safetensors` 是基础 SD1.5 模型，不是小黑读书正式生产模型；实际效果存在裁切、风格不稳定和 logo/水印风险，不能直接作为正式视频图片后端。下一步应切换到更合适的插画 checkpoint、LoRA、ControlNet/IPAdapter 工作流，或把 187 接成可配置的 `xiaohei-ai-y9000p` 后端后再做质量评估。

## 2026-07-08 受控 img2img 路线

自由 txt2img 已验证链路可用，但不适合作为《A Book in 30 Minutes》正式图片路线：基础 SD1.5 风格不稳，DreamShaper8 LCM 自由生成也容易偏向写实、抽象形状或不可控构图。当前推荐路线改为：

```text
字幕分段
-> 程序化小黑 guide PNG
-> 复制 guide 到 D:\AI\apps\ComfyUI\input\xiaohei_y9000p_guides
-> ComfyUI LoadImage + VAEEncode + KSampler 低 denoise img2img
-> 输出 xiaohei_ai_y9000p raw PNG
-> 缩放为 1920x1080 视频图
```

当前默认参数：

```text
BOOK_IMAGE_BACKEND=xiaohei-ai-y9000p
Y9000P_COMFYUI_BASE_URL=http://127.0.0.1:8188
Y9000P_COMFYUI_WORKFLOW=img2img
Y9000P_COMFYUI_CHECKPOINT=DreamShaper8_LCM.safetensors
Y9000P_COMFYUI_WIDTH=1536
Y9000P_COMFYUI_HEIGHT=864
Y9000P_COMFYUI_STEPS=32
Y9000P_COMFYUI_CFG=1.9
Y9000P_COMFYUI_DENOISE=0.38
Y9000P_COMFYUI_SAMPLER=lcm
Y9000P_COMFYUI_SCHEDULER=sgm_uniform
Y9000P_COMFYUI_INPUT_DIR=D:\AI\apps\ComfyUI\input
Y9000P_COMFYUI_RESTORE_GUIDE_LINE_ART=1
Y9000P_COMFYUI_GUIDE_CLEANUP_RADIUS=5
```

两张 controlled img2img 芒格样张已验证：构图、留白和小黑角色明显比自由 txt2img 稳定，基本没有 logo 或复杂背景风险。随后又用用户提供的官方小黑风格截图裁剪 3 个面板，分别测试 denoise 0.28、0.36、0.44，共 9 张样张，输出到：

```text
D:\AI\tests\official-xiaohei-style\references
D:\AI\tests\official-xiaohei-style\outputs
D:\AI\tests\official-xiaohei-style\official_xiaohei_comparison_sheet.png
```

结论：本机 RTX 3070 + DreamShaper8 LCM 在 reference img2img 下能保住官方白底、黑线、小黑角色和构图；denoise 0.28 最接近参考，0.36 可作为轻微重绘，0.44 会明显模型化并改变角色。后续用户要求把效果从“低成本快速图”切到“30~60 秒一张的质量优先图”，因此当前默认升级为 1536x864、32 steps、cfg 1.9、denoise 0.38，单张约 30 秒。扩散模型会破坏中文标注，因此正式生产不能让 AI 负责最终中文。当前后端已加入双 guide 机制：有中文 guide 用于最终覆盖，无中文 guide 用于 ComfyUI 输入，避免模型在采样阶段生成伪中文；ComfyUI 生成后默认只把有中文 guide 与无中文 guide 的差异文字层回贴到最终图，确保中文清晰，同时保留 AI 对无文字线稿的细节增强。

正式后端已按这条路线接入 `a-book-in-30-minutes\tmp\book_video_pipeline.py`：`xiaohei-ai-y9000p` 默认生成 32~64 张官方风格 guide，并通过本机 ComfyUI 精修；`Y9000P_COMFYUI_WORKFLOW=txt2img` 仅保留为旧 smoke 路线。guide 的中文标注优先使用 Windows 楷体 `simkai.ttf`，也可通过 `XIAOHEI_KAITI_FONT` 指向自定义楷体；画法要求尽量贴近官方参考图：白底、黑色小黑角色、少用方框和直线，标签用楷体文字加手绘波浪下划线，箭头和连接线用橙/蓝/红的弯曲手绘线。

芒格传真实书稿 40 张图片 smoke 已完成，输出到：

```text
D:\AI\tests\official-xiaohei-style\munger-book-smoke
D:\AI\tests\official-xiaohei-style\munger-book-smoke\munger_book_smoke_contact_sheet.png
```

本轮验证中，40 张图片约 145 秒生成完成；最终 `visual_XX_xiaohei_ai_y9000p.png` 中文清晰、无伪中文重影，系列风格稳定，适合作为下一步视频阶段试跑输入。

这条路线的定位是“本机 GPU 受控增强”，不是把构图完全交给模型。后续质量提升优先顺序：

1. 继续保持程序化 guide 作为构图约束。
2. 寻找更贴近小黑/手绘线稿的 SD1.5 checkpoint 或 LoRA，仍放在 `D:\AI\apps\ComfyUI\models`。
3. 如安装 ControlNet/IPAdapter，也必须放在 D 盘，并通过 `127.0.0.1:1080` 代理下载外网资源。
4. 每次替换模型后先跑 2~4 张样张，再跑完整 32~64 张图片阶段。

