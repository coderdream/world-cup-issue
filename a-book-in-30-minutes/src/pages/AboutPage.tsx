import { Info, Shield } from "lucide-react";
import { APP_IDENTIFIER, APP_NAME } from "@/config/app";
import { Panel, SectionTitle } from "@/pages/primitives";
import { useAppStore } from "@/store/useAppStore";

export function AboutPage() {
  const version = useAppStore((state) => state.version);

  return (
    <div className="page about-page">
      <Panel>
        <SectionTitle icon={<Info size={16} />} title="关于工具" inline />
        <p>{APP_NAME} 用来把 EPUB 转成 YouTube 听书视频的文本素材包，当前阶段覆盖视频标题、简介、标签、旁白稿和字幕行。</p>
      </Panel>
      <Panel className="info-list">
        <div><span>版本</span><b>{version}</b></div>
        <div><span>应用标识</span><b>{APP_IDENTIFIER}</b></div>
        <div><span>技术栈</span><b>Tauri 2 / React 19 / TypeScript / Vite</b></div>
      </Panel>
      <Panel className="notice-card">
        <Shield size={18} />
        <span>请优先处理自有版权、公版或已获授权内容。对受版权保护的作品，建议生成评论、摘要和解读型素材。</span>
      </Panel>
    </div>
  );
}
