import { useMemo, useState } from "react";
import { useDashboard } from "@/pages/useDashboard";

export function HistoryPage() {
  const { dashboard, refresh } = useDashboard();
  const [status, setStatus] = useState("全部");
  const [keyword, setKeyword] = useState("");
  const rows = useMemo(() => {
    const normalized = keyword.trim().toLowerCase();
    return (dashboard?.recentHistory ?? []).filter((item) => {
      const statusOk = status === "全部" || item.status === status;
      const keywordOk = !normalized || `${item.id} ${item.ability} ${item.episodeCode} ${item.summary} ${item.currentStage}`.toLowerCase().includes(normalized);
      return statusOk && keywordOk;
    });
  }, [dashboard, keyword, status]);

  return (
    <section className="studio-page">
      <div className="toolbar">
        <label>状态 <select value={status} onChange={(event) => setStatus(event.target.value)}><option>全部</option><option>SUCCESS</option><option>FAILED</option><option>PENDING</option></select></label>
        <label>关键字 <input value={keyword} onChange={(event) => setKeyword(event.target.value)} /></label>
        <button type="button" onClick={() => void refresh()}>刷新</button>
      </div>
      <div className="table-wrap">
        <table>
          <thead><tr><th>ID</th><th>能力</th><th>课程序号</th><th>状态</th><th>当前阶段</th><th>摘要</th><th>开始时间</th><th>结束时间</th><th>耗时(ms)</th></tr></thead>
          <tbody>
            {rows.map((item) => (
              <tr key={item.id}>
                <td>{item.id}</td><td>{item.ability}</td><td>{item.episodeCode}</td><td className={`status-${item.status.toLowerCase()}`}>{item.status}</td><td>{item.currentStage}</td><td>{item.summary}</td><td>{item.startedAt}</td><td>{item.finishedAt}</td><td>{item.durationMs}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

