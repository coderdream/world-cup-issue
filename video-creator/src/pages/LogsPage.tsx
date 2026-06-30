import { useMemo, useState } from "react";
import { useDashboard } from "@/pages/useDashboard";

const levels = ["ALL", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

export function LogsPage() {
  const { dashboard, refresh } = useDashboard();
  const [eventLevel, setEventLevel] = useState("ALL");
  const [runtimeLevel, setRuntimeLevel] = useState("ALL");
  const events = useMemo(
    () => (dashboard?.eventLogs ?? []).filter((item) => eventLevel === "ALL" || item.level === eventLevel),
    [dashboard, eventLevel]
  );
  const runtimeLogs = useMemo(
    () => (dashboard?.runtimeLogs ?? []).filter((line) => runtimeLevel === "ALL" || line.includes(`[${runtimeLevel}]`)),
    [dashboard, runtimeLevel]
  );

  return (
    <section className="studio-page">
      <div className="log-head">
        <div><b>当前任务:</b> {dashboard?.currentTask ?? "-"}</div>
        <div><b>运行日志文件:</b> {dashboard?.runtimeLogPath ?? "-"}</div>
        <button type="button" onClick={() => void refresh()}>刷新日志</button>
      </div>
      <div className="studio-panel">
        <div className="panel-title-row">
          <h2>业务事件日志</h2>
          <label>事件级别 <select value={eventLevel} onChange={(event) => setEventLevel(event.target.value)}>{levels.map((level) => <option key={level}>{level}</option>)}</select></label>
        </div>
        <pre className="log-box">{events.map((item) => `[${item.createdAt}] ${item.level} / ${item.stage} / ${item.message}`).join("\n") || "暂无业务事件日志。"}</pre>
      </div>
      <div className="studio-panel">
        <div className="panel-title-row">
          <h2>运行日志文件</h2>
          <label>运行级别 <select value={runtimeLevel} onChange={(event) => setRuntimeLevel(event.target.value)}>{levels.map((level) => <option key={level}>{level}</option>)}</select></label>
        </div>
        <pre className="log-box">{runtimeLogs.join("\n") || "暂无运行日志。"}</pre>
      </div>
    </section>
  );
}

