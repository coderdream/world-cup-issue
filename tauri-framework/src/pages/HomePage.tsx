import { Activity, Boxes, MonitorCog } from "lucide-react";
import { Panel, SectionTitle } from "@/pages/primitives";

const features = [
  { icon: <MonitorCog size={18} />, title: "桌面壳", desc: "内置侧栏、顶部栏、窗口按钮与拖拽区域。" },
  { icon: <Boxes size={18} />, title: "模块化", desc: "页面、状态、服务和 Tauri 命令按目录拆分。" },
  { icon: <Activity size={18} />, title: "可扩展", desc: "保留配置与更新入口，便于接入业务模块。" }
];

export function HomePage() {
  return (
    <div className="page">
      <Panel className="hero-panel">
        <span className="eyebrow">Tauri 2 · React 19 · TypeScript</span>
        <h2>一个干净的桌面应用起点</h2>
        <p>把业务无关的桌面框架抽出来，用来快速启动新的 Windows 优先 Tauri 项目。</p>
      </Panel>

      <SectionTitle icon={<Boxes size={16} />} title="框架能力" action="开箱即用" />
      <div className="feature-grid">
        {features.map((feature) => (
          <Panel className="feature-card" key={feature.title}>
            <div className="feature-icon">{feature.icon}</div>
            <b>{feature.title}</b>
            <span>{feature.desc}</span>
          </Panel>
        ))}
      </div>
    </div>
  );
}
