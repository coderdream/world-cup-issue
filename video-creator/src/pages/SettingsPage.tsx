import { useState } from "react";
import { useAppStore } from "@/store/useAppStore";

export function SettingsPage() {
  const settings = useAppStore((state) => state.settings);
  const updateSettings = useAppStore((state) => state.updateSettings);
  const [draft, setDraft] = useState(settings);
  const [message, setMessage] = useState("");

  async function save() {
    await updateSettings(draft);
    setMessage("配置已保存。");
  }

  return (
    <section className="studio-page">
      <div className="studio-panel">
        <h2>项目参数</h2>
        <label className="form-row"><span>旧 Java 项目目录</span><input value={draft.javaProjectDir} onChange={(event) => setDraft({ ...draft, javaProjectDir: event.target.value })} /></label>
        <label className="form-row"><span>默认输出目录</span><input value={draft.outputDir} onChange={(event) => setDraft({ ...draft, outputDir: event.target.value })} /></label>
        <label className="form-row"><span>默认课程号</span><input value={draft.defaultEpisode} onChange={(event) => setDraft({ ...draft, defaultEpisode: event.target.value })} /></label>
        <label className="form-row"><span>Quark 同步年份</span><input value={draft.quarkYears} onChange={(event) => setDraft({ ...draft, quarkYears: event.target.value })} /></label>
      </div>

      <div className="studio-panel">
        <h2>应用开关</h2>
        <label className="form-row">
          <span>主题</span>
          <select value={draft.theme} onChange={(event) => setDraft({ ...draft, theme: event.target.value as "dark" | "light" })}>
            <option value="dark">暗色</option>
            <option value="light">亮色</option>
          </select>
        </label>
        <label className="check-row">
          <input checked={draft.launchOnBoot} onChange={(event) => setDraft({ ...draft, launchOnBoot: event.target.checked })} type="checkbox" />
          开机启动
        </label>
        <label className="check-row">
          <input checked={draft.notificationsEnabled} onChange={(event) => setDraft({ ...draft, notificationsEnabled: event.target.checked })} type="checkbox" />
          启用通知
        </label>
      </div>

      <div className="studio-panel">
        <h2>API 与 AI 配置</h2>
        <label className="form-row"><span>通用 API Base URL</span><input value={draft.apiBaseUrl} onChange={(event) => setDraft({ ...draft, apiBaseUrl: event.target.value })} /></label>
        <label className="form-row"><span>通用 API Key</span><input type="password" value={draft.apiKey} onChange={(event) => setDraft({ ...draft, apiKey: event.target.value })} /></label>
        <label className="form-row"><span>AI 配置名称</span><input value={draft.aiProfile.name} onChange={(event) => setDraft({ ...draft, aiProfile: { ...draft.aiProfile, name: event.target.value } })} /></label>
        <label className="form-row"><span>AI Base URL</span><input value={draft.aiProfile.baseURL} onChange={(event) => setDraft({ ...draft, aiProfile: { ...draft.aiProfile, baseURL: event.target.value } })} /></label>
        <label className="form-row"><span>AI 模型</span><input value={draft.aiProfile.model} onChange={(event) => setDraft({ ...draft, aiProfile: { ...draft.aiProfile, model: event.target.value } })} /></label>
        <label className="form-row"><span>AI API Key</span><input type="password" value={draft.aiProfile.apiKey} onChange={(event) => setDraft({ ...draft, aiProfile: { ...draft.aiProfile, apiKey: event.target.value } })} /></label>
      </div>

      <div className="studio-panel">
        <h2>飞书配置</h2>
        <label className="form-row"><span>飞书 Webhook</span><input value={draft.feishuProfile.webhookUrl} onChange={(event) => setDraft({ ...draft, feishuProfile: { ...draft.feishuProfile, webhookUrl: event.target.value } })} /></label>
        <label className="form-row"><span>通知标题</span><input value={draft.feishuProfile.title} onChange={(event) => setDraft({ ...draft, feishuProfile: { ...draft.feishuProfile, title: event.target.value } })} /></label>
        <label className="form-row"><span>测试消息</span><input value={draft.feishuProfile.testMessage} onChange={(event) => setDraft({ ...draft, feishuProfile: { ...draft.feishuProfile, testMessage: event.target.value } })} /></label>
      </div>

      <div className="toolbar right">
        <button type="button" onClick={() => setDraft(settings)}>恢复当前配置</button>
        <button type="button" onClick={() => void save()}>保存配置</button>
      </div>
      {message && <p className="run-message">{message}</p>}
    </section>
  );
}
