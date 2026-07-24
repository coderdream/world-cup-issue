import { useDashboard } from "@/pages/useDashboard";

export function StepsPage() {
  const { dashboard, loading, error, refresh } = useDashboard();

  return (
    <section className="studio-page">
      <div className="studio-panel stat-strip">
        <div><span>当前任务</span><b>{dashboard?.currentTask ?? "-"}</b></div>
        <div><span>步骤进度</span><b>{dashboard ? `${dashboard.successfulSteps} 成功 / ${dashboard.failedSteps} 失败 / ${dashboard.runningSteps} 进行中` : "-"}</b></div>
        <div><span>总步数</span><b>{dashboard?.totalSteps ?? 22}</b></div>
        <div><span>任务摘要</span><b>{dashboard?.summary ?? "-"}</b></div>
        <button type="button" onClick={() => void refresh()}>刷新</button>
      </div>
      <DataState loading={loading} error={error} />
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th>序号</th>
              <th>步骤编码</th>
              <th>步骤名称</th>
              <th>状态</th>
              <th>开始时间</th>
              <th>结束时间</th>
              <th>耗时(ms)</th>
              <th>说明</th>
            </tr>
          </thead>
          <tbody>
            {dashboard?.steps.map((step) => (
              <tr key={`${step.seq}-${step.code}`}>
                <td>{step.seq}</td>
                <td>{step.code}</td>
                <td>{step.name}</td>
                <td className={statusClass(step.status)}>{step.status}</td>
                <td>{step.startedAt}</td>
                <td>{step.finishedAt}</td>
                <td>{formatDuration(step.durationMs)}</td>
                <td>{step.description}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function formatDuration(durationMs: number) {
  const safeMs = Math.max(0, Math.round(durationMs || 0));
  const minutes = Math.floor(safeMs / 60_000);
  const seconds = ((safeMs % 60_000) / 1000).toFixed(3).padStart(6, "0");
  return `${String(minutes).padStart(2, "0")} 分 ${seconds} 秒`;
}

function DataState({ loading, error }: { loading: boolean; error: string | null }) {
  if (loading) return <p className="muted">正在读取步骤...</p>;
  if (error) return <p className="error-text">{error}</p>;
  return null;
}

function statusClass(status: string) {
  return `status-${status.toLowerCase()}`;
}
