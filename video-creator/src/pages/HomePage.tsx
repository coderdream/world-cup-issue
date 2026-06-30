import { useState } from "react";
import { FolderOpen, Play, RefreshCw } from "lucide-react";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import type { SkillConfigEntry } from "@/types";
import { useDashboard } from "@/pages/useDashboard";

export function HomePage() {
  const settings = useAppStore((state) => state.settings);
  const updateSettings = useAppStore((state) => state.updateSettings);
  const { dashboard, loading, error, refresh } = useDashboard();
  const [episode, setEpisode] = useState(settings.defaultEpisode);
  const [outputDir, setOutputDir] = useState(settings.outputDir);
  const [previewType, setPreviewType] = useState("VOCAB_CARD");
  const [prepare, setPrepare] = useState(true);
  const [running, setRunning] = useState<string | null>(null);
  const [message, setMessage] = useState("");

  async function runSkill(skill: SkillConfigEntry) {
    setRunning(skill.command);
    setMessage(`正在执行 ${skill.title}...`);
    await updateSettings({ defaultEpisode: episode, outputDir });
    try {
      const result = await frameworkApi.runVideoWorkflow({
        command: skill.command,
        episode,
        outputDir,
        preparePublishMaterials: prepare,
        previewType
      });
      setMessage(result.message);
      await refresh();
    } catch (err) {
      setMessage(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(null);
    }
  }

  const skills = dashboard?.skills.filter((skill) => skill.enabled).sort((a, b) => a.sortOrder - b.sortOrder) ?? [];

  return (
    <section className="studio-page">
      <div className="execute-grid">
        <div className="studio-panel">
          <h2>执行参数</h2>
          <label className="form-row">
            <span>课程号参数/文件夹</span>
            <input value={episode} onChange={(event) => setEpisode(event.target.value)} />
          </label>
          <label className="form-row">
            <span>输出目录</span>
            <input value={outputDir} onChange={(event) => setOutputDir(event.target.value)} />
          </label>
          <label className="check-row">
            <input checked={prepare} onChange={(event) => setPrepare(event.target.checked)} type="checkbox" />
            流程结束后整理发布素材
          </label>
          <label className="form-row">
            <span>预览类型</span>
            <select value={previewType} onChange={(event) => setPreviewType(event.target.value)}>
              <option value="VOCAB_CARD">词汇卡预览</option>
              <option value="VOCAB_SUMMARY">词汇汇总预览</option>
              <option value="ADVANCED_TABLE">高级词汇表预览</option>
            </select>
          </label>
          <div className="button-row">
            <button type="button" onClick={() => void frameworkApi.openVideoCreatorPath("output")}>
              <FolderOpen size={15} /> 打开目录
            </button>
            <button type="button" onClick={() => void frameworkApi.openVideoCreatorPath("ppt_config")}>
              打开 PPT 配置
            </button>
            <button type="button" onClick={() => void refresh()}>
              <RefreshCw size={15} /> 刷新全部
            </button>
          </div>
        </div>

        <aside className="studio-panel summary-panel">
          <h2>运行摘要</h2>
          <dl>
            <dt>最近课程号</dt>
            <dd>{dashboard?.latestEpisode ?? "-"}</dd>
            <dt>最近状态</dt>
            <dd className={statusClass(dashboard?.latestStatus)}>{dashboard?.latestStatus ?? "-"}</dd>
            <dt>最近耗时</dt>
            <dd>{dashboard?.latestDurationMs ?? 0} ms</dd>
            <dt>最近步骤数</dt>
            <dd>{dashboard?.latestStepCount ?? 0}</dd>
            <dt>VPN 状态</dt>
            <dd className="success">{dashboard?.vpnStatus ?? "-"}</dd>
          </dl>
          <h3>近期历史</h3>
          <pre>{dashboard?.recentHistory.slice(0, 4).map((item) => `${item.startedAt} | ${item.episodeCode} | ${item.status} | ${item.durationMs} ms`).join("\n") || "-"}</pre>
        </aside>
      </div>

      <div className="studio-panel">
        <h2>快捷技能</h2>
        <div className="skill-buttons">
          {skills.map((skill) => (
            <button disabled={Boolean(running)} key={skill.key} type="button" onClick={() => void runSkill(skill)}>
              <Play size={14} /> {running === skill.command ? "执行中..." : skill.title}
            </button>
          ))}
        </div>
        {message && <p className="run-message">{message}</p>}
        {loading && <p className="muted">正在读取旧项目状态...</p>}
        {error && <p className="error-text">{error}</p>}
      </div>

      <div className="studio-panel">
        <h2>使用说明</h2>
        <ol className="tips">
          <li>输入 6 位课程号，例如 260409。</li>
          <li>点击技能按钮后，任务会通过旧 Java 能力层执行。</li>
          <li>步骤跟踪页会显示总步骤、开始时间、结束时间和实时耗时。</li>
          <li>执行日志页同时展示业务事件日志和运行日志文件。</li>
          <li>应用启动和页面加载不会自动续跑上次未完成任务。</li>
        </ol>
      </div>
    </section>
  );
}

function statusClass(status?: string) {
  return status?.toLowerCase() === "success" ? "success" : status?.toLowerCase() === "failed" ? "failed" : "";
}

