import { Bot, Clipboard, RefreshCw, Settings, Sparkles } from "lucide-react";
import { useMemo, useState } from "react";
import { Panel, SectionTitle, Switch } from "@/pages/primitives";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import type { AiGenerateResult, AiProfileShare, AiTestResult, UpdateInfo } from "@/types";

const defaultPrompt = "请用三句话说明“半小时听完一本书”频道适合做什么内容。";

export function SettingsPage() {
  const settings = useAppStore((state) => state.settings);
  const updateSettings = useAppStore((state) => state.updateSettings);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [aiTest, setAiTest] = useState<AiTestResult | null>(null);
  const [aiResult, setAiResult] = useState<AiGenerateResult | null>(null);
  const [prompt, setPrompt] = useState(defaultPrompt);
  const [busyAction, setBusyAction] = useState<"test" | "generate" | "copy" | null>(null);

  const shareText = useMemo(() => {
    const payload: AiProfileShare = {
      data: settings.aiProfile,
      kind: "ai.profile",
      v: 1
    };
    return JSON.stringify(payload, null, 2);
  }, [settings.aiProfile]);

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
      </Panel>

      <Panel>
        <SectionTitle icon={<Bot size={16} />} title="AI 模型配置" inline />
        <div className="field-grid">
          <label className="field">
            <span>配置名称</span>
            <input value={settings.aiProfile.name} onChange={(event) => void updateAiProfile({ name: event.target.value })} />
          </label>
          <label className="field">
            <span>模型名称</span>
            <input value={settings.aiProfile.model} onChange={(event) => void updateAiProfile({ model: event.target.value })} />
          </label>
        </div>
        <label className="field">
          <span>AI Base URL</span>
          <input value={settings.aiProfile.baseURL} onChange={(event) => void updateAiProfile({ baseURL: event.target.value })} />
        </label>
        <label className="field">
          <span>AI API Key</span>
          <input type="password" value={settings.aiProfile.apiKey} onChange={(event) => void updateAiProfile({ apiKey: event.target.value })} />
        </label>
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

  async function checkUpdate() {
    setUpdate(await frameworkApi.checkUpdateMock());
  }

  async function testAi() {
    setBusyAction("test");
    try {
      setAiTest(await frameworkApi.testAiProfile());
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

  async function generateAi() {
    setBusyAction("generate");
    try {
      setAiResult(await frameworkApi.generateAiText({ prompt }));
    } finally {
      setBusyAction(null);
    }
  }
}
