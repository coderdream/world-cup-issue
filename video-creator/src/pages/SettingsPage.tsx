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
        <h2>基础配置</h2>
        <label className="form-row"><span>旧 Java 项目目录</span><input value={draft.javaProjectDir} onChange={(event) => setDraft({ ...draft, javaProjectDir: event.target.value })} /></label>
        <label className="form-row"><span>默认输出目录</span><input value={draft.outputDir} onChange={(event) => setDraft({ ...draft, outputDir: event.target.value })} /></label>
        <label className="form-row"><span>默认课程号</span><input value={draft.defaultEpisode} onChange={(event) => setDraft({ ...draft, defaultEpisode: event.target.value })} /></label>
        <label className="form-row"><span>Quark 同步年份</span><input value={draft.quarkYears} onChange={(event) => setDraft({ ...draft, quarkYears: event.target.value })} /></label>
        <button type="button" onClick={() => void save()}>保存配置</button>
        {message && <p className="run-message">{message}</p>}
      </div>
      <div className="studio-panel">
        <h2>集成配置</h2>
        <label className="form-row"><span>AI Base URL</span><input value={draft.aiProfile.baseURL} onChange={(event) => setDraft({ ...draft, aiProfile: { ...draft.aiProfile, baseURL: event.target.value } })} /></label>
        <label className="form-row"><span>AI 模型</span><input value={draft.aiProfile.model} onChange={(event) => setDraft({ ...draft, aiProfile: { ...draft.aiProfile, model: event.target.value } })} /></label>
        <label className="form-row"><span>飞书 Webhook</span><input value={draft.feishuProfile.webhookUrl} onChange={(event) => setDraft({ ...draft, feishuProfile: { ...draft.feishuProfile, webhookUrl: event.target.value } })} /></label>
      </div>
    </section>
  );
}

