import { Bot, Clipboard, FolderOpen, MessageCircle, Mic2, Play, Plus, RefreshCw, Save, Send, Settings, Sparkles, Wrench } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Panel, SectionTitle, Switch } from "@/pages/primitives";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import type { AiGenerateResult, AiProfileShare, AiProvider, AiTestResult, FeishuSendResult, SpeechTestResult, SpeechVoice, ToolTestResult, UpdateInfo } from "@/types";

const defaultPrompt = "请用三句话说明“半小时听完一本书”频道适合做什么内容。";
const defaultSpeechPreviewText = "夜深了，我们用半小时，慢慢听完一本书。愿故事里的光，也照进你今晚的梦里。";

const speechRegions = [
  { code: "eastasia", label: "东亚 East Asia" },
  { code: "southeastasia", label: "东南亚 Southeast Asia" },
  { code: "eastus", label: "美国东部 East US" },
  { code: "westus", label: "美国西部 West US" },
  { code: "westeurope", label: "西欧 West Europe" },
  { code: "northeurope", label: "北欧 North Europe" },
  { code: "japaneast", label: "日本东部 Japan East" },
  { code: "koreacentral", label: "韩国中部 Korea Central" },
  { code: "australiaeast", label: "澳大利亚东部 Australia East" },
  { code: "centralindia", label: "印度中部 Central India" }
];

const speechLocales = [
  { code: "zh-CN", label: "中文（普通话，简体）" },
  { code: "en-US", label: "英语（美国）" },
  { code: "en-GB", label: "英语（英国）" }
];

export function SettingsPage() {
  const settings = useAppStore((state) => state.settings);
  const updateSettings = useAppStore((state) => state.updateSettings);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [aiTest, setAiTest] = useState<AiTestResult | null>(null);
  const [feishuTest, setFeishuTest] = useState<FeishuSendResult | null>(null);
  const [speechTest, setSpeechTest] = useState<SpeechTestResult | null>(null);
  const [ffmpegTest, setFfmpegTest] = useState<ToolTestResult | null>(null);
  const [aiResult, setAiResult] = useState<AiGenerateResult | null>(null);
  const [speechVoices, setSpeechVoices] = useState<SpeechVoice[]>([]);
  const [speechVoiceSource, setSpeechVoiceSource] = useState("");
  const [prompt, setPrompt] = useState(defaultPrompt);
  const [speechPreviewText, setSpeechPreviewText] = useState(defaultSpeechPreviewText);
  const [newMaterialCategory, setNewMaterialCategory] = useState("");
  const [busyAction, setBusyAction] = useState<"test" | "generate" | "copy" | "feishu" | "speech" | "speechPreview" | "speechSave" | "ffmpeg" | null>(null);
  const activeAiProvider = settings.activeAiProvider === "gemini" ? "gemini" : "gpt";
  const activeAiProfile = activeAiProvider === "gemini" ? settings.geminiProfile : settings.aiProfile;

  const shareText = useMemo(() => {
    const payload: AiProfileShare = {
      data: activeAiProfile,
      kind: "ai.profile",
      v: 1
    };
    return JSON.stringify(payload, null, 2);
  }, [activeAiProfile]);

  useEffect(() => {
    let cancelled = false;
    const locale = settings.speechProfile.locale || "zh-CN";
    frameworkApi
      .getSpeechVoices(locale)
      .then((result) => {
        if (cancelled) return;
        setSpeechVoices(result.voices);
        setSpeechVoiceSource(result.sourceUrl);
        if (result.voices.length > 0 && !result.voices.some((voice) => voice.voiceName === settings.speechProfile.voiceName)) {
          void updateSpeechProfile({
            locale,
            voiceName: result.voices[0].voiceName
          });
        }
      })
      .catch((error) => {
        if (cancelled) return;
        setSpeechTest({ ok: false, message: error instanceof Error ? error.message : String(error) });
      });
    return () => {
      cancelled = true;
    };
  }, [settings.speechProfile.locale]);

  return (
    <div className="page">
      <Panel>
        <SectionTitle icon={<Settings size={16} />} title="基础配置" inline />
        <div className="setting-row">
          <div>
            <b>开机启动</b>
            <span>预留给后续批量生成和后台任务。</span>
          </div>
          <Switch checked={settings.launchOnBoot} onChange={(value) => void updateSettings({ launchOnBoot: value })} />
        </div>
        <div className="setting-row">
          <div>
            <b>系统通知</b>
            <span>生成完成或后续音视频任务完成时提醒。</span>
          </div>
          <Switch checked={settings.notificationsEnabled} onChange={(value) => void updateSettings({ notificationsEnabled: value })} />
        </div>
        <div className="field-grid">
          <label className="field">
            <span>菜单字体</span>
            <input value={settings.uiProfile.menuFontFamily} onChange={(event) => void updateUiProfile({ menuFontFamily: event.target.value })} />
          </label>
          <label className="field">
            <span>菜单字号</span>
            <input
              max={18}
              min={10}
              type="number"
              value={settings.uiProfile.menuFontSize}
              onChange={(event) => void updateUiProfile({ menuFontSize: Number(event.target.value) })}
            />
          </label>
          <label className="field">
            <span>内容字体</span>
            <input value={settings.uiProfile.contentFontFamily} onChange={(event) => void updateUiProfile({ contentFontFamily: event.target.value })} />
          </label>
          <label className="field">
            <span>内容字号</span>
            <input
              max={18}
              min={10}
              type="number"
              value={settings.uiProfile.contentFontSize}
              onChange={(event) => void updateUiProfile({ contentFontSize: Number(event.target.value) })}
            />
          </label>
        </div>
        <p className="settings-help">菜单字体作用于左侧导航；内容字体作用于页面正文、表格和配置项。默认菜单 13px、内容 12px。</p>
        <div className="field-grid">
          <label className="field">
            <span>图片生成方案</span>
            <select
              value={settings.pipelineProfile.imageBackend ?? "xiaohei-production"}
              onChange={(event) => void updatePipelineProfile({ imageBackend: event.target.value as typeof settings.pipelineProfile.imageBackend })}
            >
              <option value="xiaohei-production">小黑生产版（MacMini4）</option>
              <option value="xiaohei-sequence">小黑快速版（可回退）</option>
              <option value="xiaohei-ai-y9000p">小黑 AI（本机 187）</option>
              <option value="qwen-image-2512">Qwen Image（实验）</option>
              <option value="whiteboard-skill">白板图片 Skill</option>
            </select>
          </label>
          <label className="field">
            <span>文本已有则跳过</span>
            <select
              value={(settings.pipelineProfile.skipExistingText ?? settings.pipelineProfile.skipExistingMaterials) ? "yes" : "no"}
              onChange={(event) => void updatePipelineProfile({ skipExistingText: event.target.value === "yes", skipExistingMaterials: event.target.value === "yes" })}
            >
              <option value="yes">是</option>
              <option value="no">否，每次重新生成</option>
            </select>
          </label>
          <label className="field">
            <span>图片已有则跳过</span>
            <select
              value={settings.pipelineProfile.skipExistingImages ? "yes" : "no"}
              onChange={(event) => void updatePipelineProfile({ skipExistingImages: event.target.value === "yes" })}
            >
              <option value="yes">是</option>
              <option value="no">否，每次重新生成</option>
            </select>
          </label>
          <label className="field">
            <span>音频已有则跳过</span>
            <select
              value={settings.pipelineProfile.skipExistingAudio ? "yes" : "no"}
              onChange={(event) => void updatePipelineProfile({ skipExistingAudio: event.target.value === "yes" })}
            >
              <option value="yes">是</option>
              <option value="no">否，每次重新生成</option>
            </select>
          </label>
          <label className="field">
            <span>字幕已有则跳过</span>
            <select
              value={settings.pipelineProfile.skipExistingSubtitles ? "yes" : "no"}
              onChange={(event) => void updatePipelineProfile({ skipExistingSubtitles: event.target.value === "yes" })}
            >
              <option value="yes">是</option>
              <option value="no">否，每次重新生成</option>
            </select>
          </label>
          <label className="field">
            <span>视频已有则跳过</span>
            <select
              value={settings.pipelineProfile.skipExistingVideo ? "yes" : "no"}
              onChange={(event) => void updatePipelineProfile({ skipExistingVideo: event.target.value === "yes" })}
            >
              <option value="yes">是</option>
              <option value="no">否，每次重新生成</option>
            </select>
          </label>
          <label className="field">
            <span>发布资料已有则跳过</span>
            <select
              value={settings.pipelineProfile.skipExistingPublish ? "yes" : "no"}
              onChange={(event) => void updatePipelineProfile({ skipExistingPublish: event.target.value === "yes" })}
            >
              <option value="yes">是</option>
              <option value="no">否，每次重新生成</option>
            </select>
          </label>
        </div>
        <p className="settings-help">默认选择“是”，流水线会跳过已有产物；选择“否”时，对应阶段每次点击都会重新生成。图片阶段依赖已对齐的中文字幕 SRT，应在音频和字幕完成后执行。</p>
      </Panel>

      <Panel>
        <SectionTitle icon={<Bot size={16} />} title="AI 模型配置" inline />
        <label className="field">
          <span>流水线使用 AI</span>
          <select value={activeAiProvider} onChange={(event) => void changeAiProvider(event.target.value as AiProvider)}>
            <option value="gpt">GPT</option>
            <option value="gemini">Gemini</option>
          </select>
        </label>
        <div className="segmented-control" role="tablist" aria-label="AI 模型提供商">
          <button className={activeAiProvider === "gpt" ? "active" : undefined} type="button" onClick={() => void changeAiProvider("gpt")}>
            GPT
          </button>
          <button className={activeAiProvider === "gemini" ? "active" : undefined} type="button" onClick={() => void changeAiProvider("gemini")}>
            Gemini
          </button>
        </div>
        <div className="field-grid">
          <label className="field">
            <span>配置名称</span>
            <input value={activeAiProfile.name} onChange={(event) => void updateActiveAiProfile({ name: event.target.value })} />
          </label>
          <label className="field">
            <span>模型名称</span>
            <input value={activeAiProfile.model} onChange={(event) => void updateActiveAiProfile({ model: event.target.value })} />
          </label>
        </div>
        <label className="field">
          <span>{activeAiProvider === "gemini" ? "Gemini Base URL" : "AI Base URL"}</span>
          <input value={activeAiProfile.baseURL} onChange={(event) => void updateActiveAiProfile({ baseURL: event.target.value })} />
        </label>
        <label className="field">
          <span>{activeAiProvider === "gemini" ? "Gemini API Key" : "AI API Key"}</span>
          <input type="password" value={activeAiProfile.apiKey} onChange={(event) => void updateActiveAiProfile({ apiKey: event.target.value })} />
        </label>
        <label className="field">
          <span>启用代理</span>
          <select
            value={activeAiProfile.proxyEnabled ? "yes" : "no"}
            onChange={(event) => void updateActiveAiProfile({ proxyEnabled: event.target.value === "yes" })}
          >
            <option value="no">否</option>
            <option value="yes">是</option>
          </select>
        </label>
        {activeAiProfile.proxyEnabled && (
          <label className="field">
            <span>代理地址</span>
            <input value={activeAiProfile.proxyUrl} onChange={(event) => void updateActiveAiProfile({ proxyUrl: event.target.value })} />
          </label>
        )}
        <div className="button-row">
          <button className="outline-btn" disabled={busyAction === "test"} type="button" onClick={() => void testAi()}>
            <RefreshCw className={busyAction === "test" ? "spin" : undefined} size={15} /> 测试连接
          </button>
          <button className="outline-btn" disabled={busyAction === "copy"} type="button" onClick={() => void copyAiProfile()}>
            <Clipboard size={15} /> 复制当前配置分享
          </button>
        </div>
        {aiTest && <p className={aiTest.ok ? "status success" : "status error"}>{aiTest.message}{aiTest.content ? `：${aiTest.content}` : ""}</p>}
      </Panel>

      <Panel>
        <SectionTitle icon={<MessageCircle size={16} />} title="飞书消息配置" inline />
        <label className="field">
          <span>飞书 Webhook 地址</span>
          <input
            placeholder="https://open.feishu.cn/open-apis/bot/v2/hook/..."
            value={settings.feishuProfile.webhookUrl}
            onChange={(event) => void updateFeishuProfile({ webhookUrl: event.target.value })}
          />
        </label>
        <label className="field">
          <span>消息标题</span>
          <input value={settings.feishuProfile.title} onChange={(event) => void updateFeishuProfile({ title: event.target.value })} />
        </label>
        <label className="field">
          <span>测试消息</span>
          <textarea
            value={settings.feishuProfile.testMessage}
            onChange={(event) => void updateFeishuProfile({ testMessage: event.target.value })}
            rows={3}
          />
        </label>
        <div className="button-row">
          <button className="outline-btn" disabled={busyAction === "feishu"} type="button" onClick={() => void testFeishu()}>
            <Send className={busyAction === "feishu" ? "spin" : undefined} size={15} /> 测试飞书连通性
          </button>
        </div>
        {feishuTest && <p className={feishuTest.ok ? "status success" : "status error"}>{feishuTest.message}</p>}
      </Panel>

      <Panel>
        <SectionTitle icon={<Sparkles size={16} />} title="素材生成默认配置" inline />
        <div className="field-grid">
          <label className="field">
            <span>分类 / 播放列表</span>
            <select
              value={settings.materialProfile.categoryName}
              onChange={(event) => void selectMaterialCategory(event.target.value)}
            >
              {settings.materialProfile.categories.map((category) => (
                <option key={category} value={category}>{category}</option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>目标语言</span>
            <select
              value={settings.materialProfile.language}
              onChange={(event) => void updateMaterialProfile({ language: event.target.value as "zh-CN" })}
            >
              <option value="zh-CN">中文</option>
            </select>
          </label>
        </div>
        <div className="inline-add-row">
          <input
            placeholder="新增分类 / 播放列表名称"
            value={newMaterialCategory}
            onChange={(event) => setNewMaterialCategory(event.target.value)}
          />
          <button className="outline-btn" type="button" onClick={() => void addMaterialCategory()}>
            <Plus size={15} /> 新增
          </button>
        </div>
        <p className="settings-help">分类会写入素材任务数据库，对应后续 YouTube 播放列表名称；没有选择时使用默认分类。</p>
        <div className="field-grid">
          <label className="field">
            <span>最少字数（30~35分钟）</span>
            <input
              min={1000}
              type="number"
              value={settings.materialProfile.targetMinChars}
              onChange={(event) => void updateMaterialProfile({ targetMinChars: Number(event.target.value) })}
            />
          </label>
          <label className="field">
            <span>最多字数（30~35分钟）</span>
            <input
              min={1001}
              type="number"
              value={settings.materialProfile.targetMaxChars}
              onChange={(event) => void updateMaterialProfile({ targetMaxChars: Number(event.target.value) })}
            />
          </label>
        </div>
        <p className="settings-help">这组字数会写入 SQLite，流水线生成文本时直接读取；默认 7500~7800，用来匹配 30~35 分钟语音。</p>
        <label className="field">
          <span>生成方向</span>
          <textarea
            value={settings.materialProfile.extraDirection}
            onChange={(event) => void updateMaterialProfile({ extraDirection: event.target.value })}
            rows={4}
          />
        </label>
      </Panel>

      <Panel>
        <SectionTitle
          icon={<Mic2 size={16} />}
          title="微软语音配置"
          action={
            <span className="link-actions">
              <a href="https://portal.azure.com/#view/HubsExtension/BrowseResource/resourceType/Microsoft.CognitiveServices%2Faccounts" rel="noreferrer" target="_blank">
                打开 Azure 语音资源
              </a>
              <a href="https://learn.microsoft.com/azure/ai-services/speech-service/rest-text-to-speech" rel="noreferrer" target="_blank">
                官方文档
              </a>
            </span>
          }
          inline
        />
        <p className="settings-help">
          在 Azure Portal 进入你的 Speech 资源，左侧找到 Keys and Endpoint，复制 Key 1 或 Key 2，并把同页显示的区域填到“区域”。
        </p>
        <div className="field-grid">
          <label className="field">
            <span>区域</span>
            <select
              value={settings.speechProfile.region}
              onChange={(event) => void changeSpeechRegion(event.target.value)}
            >
              {speechRegions.map((region) => (
                <option key={region.code} value={region.code}>
                  {region.label} ({region.code})
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>语音语言</span>
            <select
              value={settings.speechProfile.locale || "zh-CN"}
              onChange={(event) => void changeSpeechLocale(event.target.value)}
            >
              {speechLocales.map((locale) => (
                <option key={locale.code} value={locale.code}>
                  {locale.label} ({locale.code})
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>人声音色</span>
            <select
              value={settings.speechProfile.voiceName}
              onChange={(event) => void updateSpeechProfile({ voiceName: event.target.value })}
              disabled={speechVoices.length === 0}
            >
              {speechVoices.map((voice) => (
                <option key={voice.voiceName} value={voice.voiceName}>
                  {formatSpeechVoice(voice)}
                </option>
              ))}
            </select>
          </label>
        </div>
        {speechVoiceSource && (
          <p className="settings-help">
            音色列表来自 SQLite 内置种子数据，来源：<a href={speechVoiceSource} rel="noreferrer" target="_blank">微软语音语言支持</a>。
          </p>
        )}
        <label className="field">
          <span>Speech Key</span>
          <input
            type="password"
            value={settings.speechProfile.speechKey}
            onChange={(event) => void updateSpeechProfile({ speechKey: event.target.value })}
          />
        </label>
        <div className="button-row">
          <button className="outline-btn" disabled={busyAction === "speechSave"} type="button" onClick={() => void saveSpeechKey()}>
            <Save className={busyAction === "speechSave" ? "spin" : undefined} size={15} /> 保存默认语音配置
          </button>
        </div>
        <label className="field">
          <span>输出格式</span>
          <input
            value={settings.speechProfile.outputFormat}
            onChange={(event) => void updateSpeechProfile({ outputFormat: event.target.value })}
          />
        </label>
        <div className="field-grid">
          <label className="field">
            <span>语速</span>
            <input value={settings.speechProfile.rate} onChange={(event) => void updateSpeechProfile({ rate: event.target.value })} />
          </label>
          <label className="field">
            <span>音调</span>
            <input value={settings.speechProfile.pitch} onChange={(event) => void updateSpeechProfile({ pitch: event.target.value })} />
          </label>
        </div>
        <label className="field">
          <span>试听文字</span>
          <textarea
            value={speechPreviewText}
            onChange={(event) => setSpeechPreviewText(event.target.value)}
            rows={3}
          />
        </label>
        <div className="button-row">
          <button className="outline-btn" disabled={busyAction === "speechPreview"} type="button" onClick={() => void previewSpeech()}>
            <Play className={busyAction === "speechPreview" ? "spin" : undefined} size={15} /> 播放试听
          </button>
          <button className="outline-btn" disabled={busyAction === "speech"} type="button" onClick={() => void testSpeech()}>
            <Mic2 className={busyAction === "speech" ? "spin" : undefined} size={15} /> 测试微软语音
          </button>
        </div>
        {speechTest && (
          <p className={speechTest.ok ? "status success" : "status error"}>
            {speechTest.message}{speechTest.audioFile ? `：${speechTest.audioFile}` : ""}
          </p>
        )}
      </Panel>

      <Panel>
        <SectionTitle icon={<Wrench size={16} />} title="工具路径" inline />
        <div className="field-grid">
          <label className="field">
            <span>背景音乐循环方式</span>
            <select
              value={settings.toolProfile.backgroundMusicMode ?? "single"}
              onChange={(event) => void updateToolProfile({ backgroundMusicMode: event.target.value as "single" | "playlist" })}
            >
              <option value="single">单曲循环</option>
              <option value="playlist">列表循环</option>
            </select>
          </label>
          <label className="field">
            <span>背景音乐文件</span>
            <input
              value={settings.toolProfile.backgroundMusicPath ?? ""}
              onChange={(event) => void updateToolProfile({ backgroundMusicPath: event.target.value })}
            />
          </label>
        </div>
        <label className="field">
          <span>ffmpeg.exe 路径</span>
          <div className="path-input-row">
            <input
              placeholder="D:\tools\ffmpeg\bin\ffmpeg.exe"
              value={settings.toolProfile.ffmpegPath}
              onChange={(event) => void updateToolProfile({ ffmpegPath: event.target.value })}
            />
            <button className="icon-btn" type="button" title="选择 ffmpeg.exe" onClick={() => void chooseFfmpeg()}>
              <FolderOpen size={16} />
            </button>
          </div>
        </label>
        <div className="button-row">
          <button className="outline-btn" disabled={busyAction === "ffmpeg"} type="button" onClick={() => void testFfmpeg()}>
            <Wrench className={busyAction === "ffmpeg" ? "spin" : undefined} size={15} /> 测试 ffmpeg
          </button>
        </div>
        {ffmpegTest && (
          <p className={ffmpegTest.ok ? "status success" : "status error"}>
            {ffmpegTest.message}{ffmpegTest.version ? `：${ffmpegTest.version}` : ""}
          </p>
        )}
      </Panel>

      <Panel>
        <SectionTitle icon={<Sparkles size={16} />} title="AI 生成测试" inline />
        <label className="field">
          <span>Prompt</span>
          <textarea value={prompt} onChange={(event) => setPrompt(event.target.value)} rows={4} />
        </label>
        <button className="primary-btn" disabled={busyAction === "generate"} type="button" onClick={() => void generateAi()}>
          <Sparkles size={15} /> {busyAction === "generate" ? "生成中..." : "生成 AI 文本"}
        </button>
        {aiResult && (
          <div className="ai-result">
            <b>{aiResult.model}</b>
            <p>{aiResult.content}</p>
          </div>
        )}
      </Panel>

      <Panel>
        <SectionTitle icon={<RefreshCw size={16} />} title="更新检查" inline />
        <button className="outline-btn" type="button" onClick={() => void checkUpdate()}>
          <RefreshCw size={15} /> 检查更新
        </button>
        {update && <p className="muted">{update.notes}</p>}
      </Panel>
    </div>
  );

  function updateAiProfile(profile: Partial<typeof settings.aiProfile>) {
    void updateSettings({
      aiProfile: {
        ...settings.aiProfile,
        ...profile
      }
    });
  }

  function updateGeminiProfile(profile: Partial<typeof settings.geminiProfile>) {
    void updateSettings({
      geminiProfile: {
        ...settings.geminiProfile,
        ...profile
      }
    });
  }

  function updateActiveAiProfile(profile: Partial<typeof settings.aiProfile> | Partial<typeof settings.geminiProfile>) {
    if (activeAiProvider === "gemini") {
      updateGeminiProfile(profile as Partial<typeof settings.geminiProfile>);
      return;
    }
    updateAiProfile(profile as Partial<typeof settings.aiProfile>);
  }

  async function changeAiProvider(provider: AiProvider) {
    await updateSettings({ activeAiProvider: provider });
    setAiTest(null);
    setAiResult(null);
  }

  function updateFeishuProfile(profile: Partial<typeof settings.feishuProfile>) {
    void updateSettings({
      feishuProfile: {
        ...settings.feishuProfile,
        ...profile
      }
    });
  }

  function updateMaterialProfile(profile: Partial<typeof settings.materialProfile>) {
    void updateSettings({
      materialProfile: {
        ...settings.materialProfile,
        ...profile
      }
    });
  }

  function updateUiProfile(profile: Partial<typeof settings.uiProfile>) {
    void updateSettings({
      uiProfile: {
        ...settings.uiProfile,
        ...profile
      }
    });
  }

  function updatePipelineProfile(profile: Partial<typeof settings.pipelineProfile>) {
    const nextProfile = {
      ...settings.pipelineProfile,
      ...profile
    };
    if ("skipExistingText" in profile && !("skipExistingMaterials" in profile)) {
      nextProfile.skipExistingMaterials = nextProfile.skipExistingText;
    }
    if ("skipExistingMaterials" in profile && !("skipExistingText" in profile)) {
      nextProfile.skipExistingText = nextProfile.skipExistingMaterials;
    }
    void updateSettings({
      pipelineProfile: nextProfile
    });
  }

  function selectMaterialCategory(categoryName: string) {
    void updateSettings({
      materialProfile: {
        ...settings.materialProfile,
        categoryName,
        channelName: categoryName
      }
    });
  }

  async function addMaterialCategory() {
    const categoryName = newMaterialCategory.trim();
    if (!categoryName) return;
    const categories = Array.from(new Set([...settings.materialProfile.categories, categoryName]));
    await updateSettings({
      materialProfile: {
        ...settings.materialProfile,
        categoryName,
        channelName: categoryName,
        categories
      }
    });
    setNewMaterialCategory("");
  }

  function updateSpeechProfile(profile: Partial<typeof settings.speechProfile>) {
    void updateSettings({
      speechProfile: {
        ...settings.speechProfile,
        ...profile
      }
    });
  }

  async function changeSpeechRegion(region: string) {
    try {
      const result = await frameworkApi.getSpeechRegionKey(region);
      await updateSettings({
        speechProfile: {
          ...settings.speechProfile,
          region,
          speechKey: result.speechKey,
          voiceName: result.voiceName || settings.speechProfile.voiceName,
          outputFormat: result.outputFormat || settings.speechProfile.outputFormat,
          rate: result.rate || settings.speechProfile.rate,
          pitch: result.pitch || settings.speechProfile.pitch,
          regionKeys: {
            ...settings.speechProfile.regionKeys,
            ...(result.hasKey ? { [region]: result.speechKey } : {})
          }
        }
      });
      setSpeechTest({
        ok: result.hasKey,
        message: result.hasKey ? `已读取 ${region} 的默认语音配置。` : `${region} 还没有保存默认语音配置。`
      });
    } catch (error) {
      setSpeechTest({ ok: false, message: error instanceof Error ? error.message : String(error) });
    }
  }

  async function changeSpeechLocale(locale: string) {
    await updateSettings({
      speechProfile: {
        ...settings.speechProfile,
        locale
      }
    });
  }

  function updateToolProfile(profile: Partial<typeof settings.toolProfile>) {
    void updateSettings({
      toolProfile: {
        ...settings.toolProfile,
        ...profile
      }
    });
  }

  async function checkUpdate() {
    setUpdate(await frameworkApi.checkUpdateMock());
  }

  async function testAi() {
    setBusyAction("test");
    try {
      await updateSettings(activeAiProvider === "gemini" ? { geminiProfile: settings.geminiProfile } : { aiProfile: settings.aiProfile });
      const result = await frameworkApi.testAiProfile();
      if (result.ok) {
        await updateSettings(activeAiProvider === "gemini" ? { geminiProfile: settings.geminiProfile } : { aiProfile: settings.aiProfile });
        setAiTest({
          ...result,
          message: result.message.includes("已保存") ? result.message : `${result.message} API Key 已保存。`
        });
        return;
      }
      setAiTest(result);
    } catch (error) {
      setAiTest({ ok: false, message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function copyAiProfile() {
    setBusyAction("copy");
    try {
      await navigator.clipboard.writeText(shareText);
      setAiTest({ ok: true, message: "AI 配置已复制。" });
    } catch (error) {
      setAiTest({ ok: false, message: error instanceof Error ? error.message : "复制失败。" });
    } finally {
      setBusyAction(null);
    }
  }

  async function testFeishu() {
    setBusyAction("feishu");
    try {
      setFeishuTest(await frameworkApi.testFeishuProfile());
    } catch (error) {
      setFeishuTest({ ok: false, message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function testSpeech() {
    setBusyAction("speech");
    try {
      await updateSettings({ speechProfile: settings.speechProfile });
      setSpeechTest(await frameworkApi.testSpeechProfile());
    } catch (error) {
      setSpeechTest({ ok: false, message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function saveSpeechKey() {
    setBusyAction("speechSave");
    try {
      const result = await frameworkApi.saveSpeechRegionKey({
        region: settings.speechProfile.region,
        speechKey: settings.speechProfile.speechKey,
        voiceName: settings.speechProfile.voiceName,
        outputFormat: settings.speechProfile.outputFormat,
        rate: settings.speechProfile.rate,
        pitch: settings.speechProfile.pitch
      });
      await updateSettings({
        speechProfile: {
          ...settings.speechProfile,
          region: result.region,
          speechKey: result.speechKey,
          voiceName: result.voiceName,
          outputFormat: result.outputFormat,
          rate: result.rate,
          pitch: result.pitch,
          regionKeys: {
            ...settings.speechProfile.regionKeys,
            [result.region]: result.speechKey
          }
        }
      });
      setSpeechTest({ ok: true, message: `已保存 ${result.region} 的默认语音配置到 SQLite。` });
    } catch (error) {
      setSpeechTest({ ok: false, message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function previewSpeech() {
    setBusyAction("speechPreview");
    try {
      await updateSettings({ speechProfile: settings.speechProfile });
      const result = await frameworkApi.previewSpeech({ text: speechPreviewText });
      setSpeechTest(result);
      if (result.audioDataUrl) {
        const audio = new Audio(result.audioDataUrl);
        await audio.play();
      } else if (result.audioFile && !result.audioFile.includes("浏览器预览模式")) {
        const { convertFileSrc } = await import("@tauri-apps/api/core");
        const audio = new Audio(convertFileSrc(result.audioFile));
        await audio.play();
      }
    } catch (error) {
      setSpeechTest({ ok: false, message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function chooseFfmpeg() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: false,
        multiple: false,
        title: "选择 ffmpeg.exe"
      });
      if (typeof selected === "string" && selected) {
        await updateSettings({
          toolProfile: {
            ...settings.toolProfile,
            ffmpegPath: selected
          }
        });
      }
    } catch (error) {
      setFfmpegTest({ ok: false, message: error instanceof Error ? error.message : "当前环境无法打开文件选择器。" });
    }
  }

  async function testFfmpeg() {
    setBusyAction("ffmpeg");
    try {
      await updateSettings({ toolProfile: settings.toolProfile });
      setFfmpegTest(await frameworkApi.testFfmpegPath());
    } catch (error) {
      setFfmpegTest({ ok: false, message: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function generateAi() {
    setBusyAction("generate");
    try {
      setAiResult(await frameworkApi.generateAiText({ prompt }));
    } finally {
      setBusyAction(null);
    }
  }
}

function formatSpeechVoice(voice: SpeechVoice) {
  const gender = voice.gender === "Male" ? "男声" : voice.gender === "Female" ? "女声" : voice.gender;
  const shortName = voice.voiceName
    .replace(/^zh-CN-/, "")
    .replace(/Neural$/, "");
  const style = voice.styles && voice.styles !== "general" ? `，${voice.styles.split(",")[0].trim()}` : "";
  return `${shortName} ${gender}${style} (${voice.voiceName})`;
}
