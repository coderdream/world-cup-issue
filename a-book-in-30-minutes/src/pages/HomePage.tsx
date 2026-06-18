import { BookOpenText, Clipboard, Download, FileText, Hash, ListVideo, Loader2, MessageSquareText, Sparkles, Tags } from "lucide-react";
import { useMemo, useState } from "react";
import { Panel, SectionTitle } from "@/pages/primitives";
import { frameworkApi } from "@/services/frameworkApi";
import type { BookMaterials, BookMaterialsRequest } from "@/types";

type OutputTab = "title" | "description" | "tags" | "narration" | "subtitles" | "prompt";

const defaultRequest: BookMaterialsRequest = {
  epubPath: "",
  targetMinChars: 5005,
  targetMaxChars: 6000,
  channelName: "半小时听完一本书",
  language: "zh-CN",
  extraDirection: "睡前听书风格，温柔、克制、有陪伴感。标题和简介服务于 YouTube 中文频道。"
};

const outputTabs: Array<{ key: OutputTab; label: string; icon: React.ReactNode }> = [
  { key: "title", label: "标题", icon: <ListVideo size={15} /> },
  { key: "description", label: "简介", icon: <MessageSquareText size={15} /> },
  { key: "tags", label: "标签", icon: <Tags size={15} /> },
  { key: "narration", label: "旁白", icon: <FileText size={15} /> },
  { key: "subtitles", label: "字幕", icon: <Hash size={15} /> },
  { key: "prompt", label: "提示词", icon: <BookOpenText size={15} /> }
];

export function HomePage() {
  const [request, setRequest] = useState<BookMaterialsRequest>(defaultRequest);
  const [materials, setMaterials] = useState<BookMaterials | null>(null);
  const [activeTab, setActiveTab] = useState<OutputTab>("title");
  const [busy, setBusy] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [outputDir, setOutputDir] = useState("");
  const [error, setError] = useState("");
  const [copyState, setCopyState] = useState("");
  const [exportState, setExportState] = useState("");

  const activeContent = useMemo(() => {
    if (!materials) return "";
    switch (activeTab) {
      case "title":
        return materials.videoTitle;
      case "description":
        return materials.description;
      case "tags":
        return materials.tags.join(", ");
      case "narration":
        return materials.narration;
      case "subtitles":
        return materials.subtitles.join("\n");
      case "prompt":
        return materials.prompt;
    }
  }, [activeTab, materials]);

  const narrationChars = materials ? countHanChars(materials.narration) : 0;

  return (
    <div className="page material-page">
      <Panel className="generator-hero">
        <div>
          <span className="eyebrow">A Book in 30 Minutes</span>
          <h2>半小时听完一本书</h2>
          <p>从 EPUB 生成 YouTube 听书视频的标题、简介、标签、旁白稿和字幕文本。</p>
        </div>
        <div className="hero-metrics">
          <span>5005-6000 字</span>
          <b>第一阶段</b>
          <small>文本素材</small>
        </div>
      </Panel>

      <div className="generator-grid">
        <Panel className="input-panel">
          <SectionTitle icon={<BookOpenText size={16} />} title="EPUB 输入" inline />
          <label className="field">
            <span>EPUB 文件路径</span>
            <input
              placeholder="C:\Users\Administrator\Downloads\book.epub"
              value={request.epubPath}
              onChange={(event) => updateRequest({ epubPath: event.target.value })}
            />
          </label>
          <div className="field-grid">
            <label className="field">
              <span>频道名</span>
              <input value={request.channelName} onChange={(event) => updateRequest({ channelName: event.target.value })} />
            </label>
            <label className="field">
              <span>目标语言</span>
              <input disabled value="中文" />
            </label>
          </div>
          <div className="field-grid">
            <label className="field">
              <span>最少字数</span>
              <input
                min={1000}
                type="number"
                value={request.targetMinChars}
                onChange={(event) => updateRequest({ targetMinChars: Number(event.target.value) })}
              />
            </label>
            <label className="field">
              <span>最多字数</span>
              <input
                min={1001}
                type="number"
                value={request.targetMaxChars}
                onChange={(event) => updateRequest({ targetMaxChars: Number(event.target.value) })}
              />
            </label>
          </div>
          <label className="field">
            <span>生成方向</span>
            <textarea value={request.extraDirection} onChange={(event) => updateRequest({ extraDirection: event.target.value })} rows={5} />
          </label>
          <button className="primary-btn wide-btn" disabled={busy} type="button" onClick={() => void generateMaterials()}>
            {busy ? <Loader2 className="spin" size={16} /> : <Sparkles size={16} />}
            {busy ? "生成中..." : "生成 YouTube 素材"}
          </button>
          {error && <p className="status error">{error}</p>}
          {copyState && <p className="status success">{copyState}</p>}
          {exportState && <p className="status success">{exportState}</p>}
        </Panel>

        <Panel className="result-panel">
          <SectionTitle icon={<FileText size={16} />} title="素材结果" inline />
          {materials ? (
            <>
              <div className="overview-strip">
                <div>
                  <span>书名</span>
                  <b>{materials.overview.title}</b>
                </div>
                <div>
                  <span>原文中文</span>
                  <b>{materials.overview.totalChars.toLocaleString()}</b>
                </div>
                <div>
                  <span>旁白中文</span>
                  <b>{narrationChars.toLocaleString()}</b>
                </div>
                <div>
                  <span>字幕行</span>
                  <b>{materials.subtitles.length.toLocaleString()}</b>
                </div>
              </div>

              <div className="tabs-row" role="tablist">
                {outputTabs.map((tab) => (
                  <button
                    aria-selected={activeTab === tab.key}
                    className={activeTab === tab.key ? "tab-btn active" : "tab-btn"}
                    key={tab.key}
                    onClick={() => setActiveTab(tab.key)}
                    role="tab"
                    type="button"
                  >
                    {tab.icon}
                    {tab.label}
                  </button>
                ))}
              </div>

              {activeTab === "tags" ? (
                <div className="tag-output">
                  {materials.tags.map((tag) => (
                    <span key={tag}>{tag}</span>
                  ))}
                </div>
              ) : (
                <pre className="material-output">{activeContent}</pre>
              )}

              <div className="button-row">
                <button className="outline-btn" type="button" onClick={() => void copyText(activeContent)}>
                  <Clipboard size={15} /> 复制当前
                </button>
                <button className="outline-btn" type="button" onClick={() => void copyAll()}>
                  <Clipboard size={15} /> 复制全部素材
                </button>
              </div>

              <div className="export-box">
                <label className="field">
                  <span>导出目录</span>
                  <input
                    placeholder="留空则导出到应用数据目录 exports"
                    value={outputDir}
                    onChange={(event) => setOutputDir(event.target.value)}
                  />
                </label>
                <button className="primary-btn wide-btn" disabled={exporting} type="button" onClick={() => void exportMaterials()}>
                  {exporting ? <Loader2 className="spin" size={16} /> : <Download size={16} />}
                  {exporting ? "导出中..." : "导出素材包"}
                </button>
              </div>
            </>
          ) : (
            <div className="empty-result">
              <Sparkles size={28} />
              <b>等待生成</b>
              <span>填写 EPUB 路径后生成第一批 YouTube 文本素材。</span>
            </div>
          )}
        </Panel>
      </div>
    </div>
  );

  function updateRequest(partial: Partial<BookMaterialsRequest>) {
    setRequest((current) => ({ ...current, ...partial }));
  }

  async function generateMaterials() {
    setBusy(true);
    setError("");
    setCopyState("");
    setExportState("");
    try {
      const result = await frameworkApi.generateBookMaterials(request);
      setMaterials(result);
      setActiveTab("title");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  }

  async function copyText(value: string) {
    await navigator.clipboard.writeText(value);
    setCopyState("已复制当前素材。");
  }

  async function copyAll() {
    if (!materials) return;
    const all = [
      `视频标题\n${materials.videoTitle}`,
      `视频简介\n${materials.description}`,
      `标签\n${materials.tags.join(", ")}`,
      `旁白稿\n${materials.narration}`,
      `字幕\n${materials.subtitles.join("\n")}`,
      `提示词\n${materials.prompt}`
    ].join("\n\n---\n\n");
    await navigator.clipboard.writeText(all);
    setCopyState("已复制全部素材。");
  }

  async function exportMaterials() {
    if (!materials) return;
    setExporting(true);
    setError("");
    setExportState("");
    try {
      const result = await frameworkApi.exportBookMaterials({ outputDir, materials });
      setExportState(`已导出 ${result.files.length} 个文件：${result.outputDir}`);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setExporting(false);
    }
  }
}

function countHanChars(value: string) {
  return Array.from(value).filter((char) => /[\u4e00-\u9fff]/.test(char)).length;
}
