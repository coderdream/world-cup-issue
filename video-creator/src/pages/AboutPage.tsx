import { APP_IDENTIFIER, APP_NAME, APP_VERSION } from "@/config/app";

export function AboutPage() {
  return (
    <section className="studio-page">
      <div className="studio-panel">
        <h2>{APP_NAME}</h2>
        <p className="muted">基于 Tauri 的 BBC 六分钟视频创作工作台，第一版复用 Video Easy Creator 的 Java 命令能力。</p>
        <dl className="about-list">
          <dt>版本</dt><dd>{APP_VERSION}</dd>
          <dt>标识</dt><dd>{APP_IDENTIFIER}</dd>
          <dt>能力层</dt><dd>D:\04_GitHub\world-cup-issue\video-creator</dd>
          <dt>启动策略</dt><dd>只展示历史状态，不自动续跑未完成任务。</dd>
        </dl>
      </div>
    </section>
  );
}