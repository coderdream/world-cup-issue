import { Info, Shield } from "lucide-react";
import { APP_IDENTIFIER, APP_NAME } from "@/config/app";
import { Panel, SectionTitle } from "@/pages/primitives";
import { useAppStore } from "@/store/useAppStore";

export function AboutPage() {
  const version = useAppStore((state) => state.version);

  return (
    <div className="page about-page">
      <Panel>
        <SectionTitle icon={<Info size={16} />} title="关于框架" inline />
        <p>{APP_NAME} 是一个可复用的 Tauri 桌面应用基础框架，适合继续扩展业务页面、后台命令和本地存储。</p>
      </Panel>
      <Panel className="info-list">
        <div><span>版本</span><b>{version}</b></div>
        <div><span>应用标识</span><b>{APP_IDENTIFIER}</b></div>
        <div><span>技术栈</span><b>Tauri 2 / React 19 / TypeScript / Vite</b></div>
      </Panel>
      <Panel className="notice-card">
        <Shield size={18} />
        <span>模板默认不包含业务数据源、不包含授权服务，也不绑定任何第三方品牌。</span>
      </Panel>
    </div>
  );
}
