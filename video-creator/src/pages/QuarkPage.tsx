import { useEffect, useState } from "react";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import { useDashboard } from "@/pages/useDashboard";

export function QuarkPage() {
  const settings = useAppStore((state) => state.settings);
  const [years, setYears] = useState(settings.quarkYears);
  const [message, setMessage] = useState("");
  const [busyCommand, setBusyCommand] = useState<string | null>(null);
  const { dashboard, refresh } = useDashboard();

  useEffect(() => {
    const timer = window.setInterval(() => void refresh(), 2000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  async function run(command: string, label: string, useInputYears = false) {
    setBusyCommand(command);
    setMessage(`正在执行：${label}`);
    try {
      const result = await frameworkApi.runVideoWorkflow({ command, years: useInputYears ? years : undefined });
      setMessage(result.message);
      await refresh();
    } catch (err) {
      setMessage(err instanceof Error ? err.message : String(err));
    } finally {
      setBusyCommand(null);
    }
  }

  return (
    <section className="studio-page">
      <div className="quark-grid">
        <div className="studio-panel">
          <h2>Quark 状态</h2>
          <dl className="quark-status">
            <dt>Token有效性</dt><dd>{dashboard?.quark.tokenValid ?? "-"}</dd>
            <dt>Cookie文件</dt><dd>{dashboard?.quark.cookieFile ?? "-"}</dd>
            <dt>Cookie更新时间</dt><dd>{dashboard?.quark.cookieUpdatedAt ?? "-"}</dd>
            <dt>根目录返回数量</dt><dd>{dashboard?.quark.rootItemCount ?? 0}</dd>
            <dt>最近校验结果</dt><dd>{dashboard?.quark.latestResult ?? "-"}</dd>
          </dl>
        </div>
        <div className="studio-panel">
          <h2>同步操作</h2>
          <label className="form-row"><span>同步年份</span><input value={years} onChange={(event) => setYears(event.target.value)} /></label>
          <div className="button-row">
            <button type="button" disabled={busyCommand !== null} onClick={() => void run("daily-sync", "同步默认年份")}>同步默认年份</button>
            <button type="button" disabled={busyCommand !== null} onClick={() => void run("daily-sync", "同步输入年份", true)}>同步输入年份</button>
            <button type="button" onClick={() => void frameworkApi.openVideoCreatorPath("quark_cookie")}>打开 Cookie 目录</button>
            <button type="button" onClick={() => void frameworkApi.openVideoCreatorPath("quark_sync")}>打开同步目录</button>
          </div>
          {message && <p className="run-message">{message}</p>}
        </div>
      </div>
      <div className="studio-panel">
        <h2>Quark 日志</h2>
        <pre className="log-box">{dashboard?.quark.logs.join("\n") || "等待手动检查..."}</pre>
      </div>
    </section>
  );
}
