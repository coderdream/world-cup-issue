import {
  BookOpenText,
  Clipboard,
  Download,
  FilePlus2,
  FileText,
  FolderOpen,
  Hash,
  ListChecks,
  ListVideo,
  Loader2,
  MessageSquareText,
  Send,
  RefreshCw,
  Sparkles,
  Tags,
  Video,
  Volume2
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Panel, SectionTitle } from "@/pages/primitives";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import type { MaterialFile, MaterialTaskProgressEvent, MaterialsOutputTab } from "@/types";

const outputTabs: Array<{ key: MaterialsOutputTab; label: string; icon: React.ReactNode }> = [
  { key: "title", label: "标题", icon: <ListVideo size={15} /> },
  { key: "description", label: "简介", icon: <MessageSquareText size={15} /> },
  { key: "tags", label: "标签", icon: <Tags size={15} /> },
  { key: "narration", label: "旁白", icon: <FileText size={15} /> },
  { key: "subtitles", label: "字幕", icon: <Hash size={15} /> },
  { key: "prompt", label: "提示词", icon: <BookOpenText size={15} /> }
];

export function HomePage() {
  const settings = useAppStore((state) => state.settings);
  const workbench = useAppStore((state) => state.materialsWorkbench);
  const updateWorkbench = useAppStore((state) => state.updateMaterialsWorkbench);
  const updateRequest = useAppStore((state) => state.updateBookMaterialsRequest);
  const { request, materials, scanResult, fileStatuses, selectedTaskPath, outputDir, error, copyState, exportState, activeTab, currentTraceId, busy, scanning, exporting } = workbench;
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; file: MaterialFile } | null>(null);
  const [selectedTaskPaths, setSelectedTaskPaths] = useState<string[]>([]);
  const [activePipelineStage, setActivePipelineStage] = useState<"materials" | "audio" | "video" | "publish" | null>(null);

  useEffect(() => {
    void loadStoredTasks(settings.materialProfile.categoryName);
  }, [settings.materialProfile.categoryName]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    import("@tauri-apps/api/event")
      .then(({ listen }) =>
        listen<MaterialTaskProgressEvent>("material-task-progress", (event) => {
          const current = useAppStore.getState().materialsWorkbench;
          if (event.payload.traceId !== current.currentTraceId) return;
          if (event.payload.path !== current.request.epubPath) return;
          const status = event.payload.progress >= 100 ? "success" : event.payload.status;
          void updateTaskStatus(event.payload.path, {
            status,
            progress: event.payload.progress,
            message: event.payload.message
          });
        })
      )
      .then((dispose) => {
        if (cancelled) {
          dispose();
        } else {
          unlisten = dispose;
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

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
  const allTaskPaths = scanResult?.files.map((file) => file.path) ?? [];
  const selectedTaskSet = new Set(selectedTaskPaths);
  const allTasksSelected = allTaskPaths.length > 0 && allTaskPaths.every((path) => selectedTaskSet.has(path));

  return (
    <div className="page material-page" onClick={() => setContextMenu(null)}>
      <Panel className="material-analyze-panel">
        <SectionTitle
          action={renderPipelineActions()}
          icon={<BookOpenText size={16} />}
          title="流水线分析面板"
          inline
        />
        <div className="material-analyze-row">
          <label className="field">
            <span>素材路径</span>
            <div className="path-input-row">
              <input
                placeholder="C:\Users\Administrator\Downloads\book.epub 或文件夹"
                value={request.epubPath}
                onChange={(event) => updateRequest({ epubPath: event.target.value })}
              />
              <button className="icon-btn" disabled={scanning} type="button" title="选择文件" onClick={() => void chooseFile()}>
                <FilePlus2 size={16} />
              </button>
              <button className="icon-btn" disabled={scanning} type="button" title="通过素材文件定位文件夹" onClick={() => void chooseFolder()}>
                <FolderOpen size={16} />
              </button>
              <button className="icon-btn" disabled={scanning || !request.epubPath.trim()} type="button" title="扫描素材文件" onClick={() => void scanPath()}>
                {scanning ? <Loader2 className="spin" size={16} /> : <RefreshCw size={16} />}
              </button>
            </div>
          </label>
        </div>
        <div className="material-analyze-summary">
          <span>分类：{settings.materialProfile.categoryName}</span>
          <span>频道：{settings.materialProfile.channelName}</span>
          <span>目标：{settings.materialProfile.targetMinChars}-{settings.materialProfile.targetMaxChars} 字</span>
          <span>状态：{scanResult ? `${scanResult.files.length} 个任务` : "等待扫描"}</span>
        </div>
          {error && <p className="status error">{error}</p>}
          {copyState && <p className="status success">{copyState}</p>}
          {exportState && <p className="status success">{exportState}</p>}
      </Panel>

      <Panel className="task-list-panel">
        <SectionTitle icon={<ListChecks size={16} />} title="流水线任务列表" inline />
        {scanResult && scanResult.files.length > 0 ? (
          <div className="material-file-table task-table">
            <div className="material-file-row header">
              <label className="task-check-cell" title="全选">
                <input checked={allTasksSelected} type="checkbox" onChange={(event) => toggleAllTasks(event.target.checked)} />
              </label>
              <span>任务</span>
              <span>格式</span>
              <span>素材</span>
              <span>素材进度</span>
              <span>成稿字数</span>
              <span>音频</span>
              <span>音频进度</span>
              <span>音频时长</span>
              <span>视频</span>
              <span>视频进度</span>
              <span>视频时长</span>
              <span>视频大小</span>
            </div>
            {scanResult.files.map((file) => renderTaskRow(file))}
          </div>
        ) : (
          <div className="empty-result task-empty">
            <Sparkles size={26} />
            <b>暂无任务</b>
            <span>选择文件或文件夹后，任务会显示在这里。</span>
          </div>
        )}
      </Panel>

      {materials && (
        <Panel className="result-panel compact-result-panel">
          <SectionTitle icon={<FileText size={16} />} title="最近生成结果" inline />
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
                onClick={() => updateWorkbench({ activeTab: tab.key })}
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
                onChange={(event) => updateWorkbench({ outputDir: event.target.value })}
              />
            </label>
            <button className="primary-btn wide-btn" disabled={exporting} type="button" onClick={() => void exportMaterials()}>
              {exporting ? <Loader2 className="spin" size={16} /> : <Download size={16} />}
              {exporting ? "导出中..." : "导出素材包"}
            </button>
          </div>
        </Panel>
      )}

      {contextMenu && renderContextMenu(contextMenu.file, contextMenu.x, contextMenu.y)}
    </div>
  );

  function renderTaskRow(file: MaterialFile) {
    const supported = isSupportedMaterial(file);
    const parsable = canGenerate(file);
    const generation = fileStatuses[file.path] ?? statusFromFile(file);
    return (
      <div
        className={selectedTaskPath === file.path ? "material-file-row active" : "material-file-row"}
        key={file.path}
        role="button"
        tabIndex={0}
        title={supported ? "选择这个文件" : "可选择，生成时会提示不支持的格式"}
        onClick={() => selectMaterialFile(file)}
        onContextMenu={(event) => openTaskMenu(event, file)}
      >
        <label className="task-check-cell" onClick={(event) => event.stopPropagation()}>
          <input checked={selectedTaskSet.has(file.path)} type="checkbox" onChange={(event) => toggleTask(file.path, event.target.checked)} />
        </label>
        <span className="file-name-cell" title={file.name}>
          <FileText size={15} />
          <span>{formatTaskName(file.name)}</span>
        </span>
        <span>{formatExtension(file)}</span>
        <span className={getGenerationStatusClass(generation, parsable, supported)}>{formatGenerationStatus(generation, parsable, supported)}</span>
        <span>{formatProgress(generation, parsable)}</span>
        <small className={supported ? undefined : "unsupported"}>{formatNarrationChars(generation)}</small>
        <span className={getGenerationStatusClass({ status: file.audioStatus }, true, true)}>
          {formatStageStatus(file.audioStatus)}
        </span>
        <span>{formatRawProgress(file.audioProgress)}</span>
        <span>{formatAudioDuration(file.audioDurationMs)}</span>
        <span className={getVideoStatusClass(file)}>
          {formatVideoStageStatus(file)}
        </span>
        <span>{formatVideoProgress(file)}</span>
        <span>{formatAudioDuration(file.videoDurationMs)}</span>
        <span>{formatOptionalBytes(file.videoFileSize)}</span>
      </div>
    );
  }

  function renderPipelineActions() {
    return (
      <div className="pipeline-actions">
        <button className={getPipelineStageClass("materials")} disabled={busy} type="button" onClick={() => void generateSelectedMaterials()}>
          {busy && activePipelineStage === "materials" ? <Loader2 className="spin" size={16} /> : <BookOpenText size={16} />}
          素材
        </button>
        <button className={getPipelineStageClass("audio")} disabled={busy} type="button" onClick={() => void runAudioPipeline()}>
          {busy && activePipelineStage === "audio" ? <Loader2 className="spin" size={16} /> : <Volume2 size={16} />}
          音频
        </button>
        <button className={getPipelineStageClass("video")} disabled={busy || !hasVideoPipelineTarget()} type="button" title="一键生成视频" onClick={() => void runVideoPipeline()}>
          {busy && activePipelineStage === "video" ? <Loader2 className="spin" size={16} /> : <Video size={16} />}
          视频
        </button>
        <button className={getPipelineStageClass("publish")} disabled={busy || !hasVideoPipelineTarget()} type="button" title="生成 YouTube 发布资料" onClick={() => void generatePublishMaterials()}>
          {busy && activePipelineStage === "publish" ? <Loader2 className="spin" size={16} /> : <Send size={16} />}
          发布
        </button>
      </div>
    );
  }

  function getPipelineStageClass(stage: "materials" | "audio" | "video" | "publish") {
    return `pipeline-stage-btn${activePipelineStage === stage ? " active" : ""}`;
  }

  function toggleTask(path: string, checked: boolean) {
    setSelectedTaskPaths((current) => checked ? Array.from(new Set([...current, path])) : current.filter((value) => value !== path));
  }

  function toggleAllTasks(checked: boolean) {
    setSelectedTaskPaths(checked ? allTaskPaths : []);
  }

  async function chooseFile() {
    clearTransientState();
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: false,
        filters: [materialFileFilter],
        multiple: false,
        title: "选择素材文件"
      });
      if (typeof selected !== "string" || !selected) return;
      activateMaterialPath(selected);
      await scanPath(selected);
    } catch (caught) {
      updateWorkbench({ error: caught instanceof Error ? caught.message : "当前环境无法打开文件选择器。" });
    }
  }

  async function chooseFolder() {
    clearTransientState();
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: false,
        filters: [materialFileFilter],
        multiple: false,
        title: "选择一个素材文件来定位所在文件夹"
      });
      if (typeof selected !== "string" || !selected) return;
      activateMaterialPath(selected);
      await scanPath(selected);
    } catch (caught) {
      updateWorkbench({ error: caught instanceof Error ? caught.message : "当前环境无法打开素材文件选择器。" });
    }
  }

  async function scanPath(path = request.epubPath) {
    updateWorkbench({ scanning: true, error: "", copyState: "", exportState: "" });
    try {
      const result = await frameworkApi.scanMaterialFiles({ path });
      updateWorkbench({ scanResult: result, fileStatuses: statusesFromFiles(result.files) });
      setSelectedTaskPaths((current) => current.filter((taskPath) => result.files.some((file) => file.path === taskPath)));
      if (result.files.some((file) => file.path === path)) {
        updateWorkbench({ selectedTaskPath: path });
      }
      const preferred = result.files.find(canGenerate);
      if (preferred && path === result.directory) {
        activateMaterialPath(preferred.path);
      }
    } catch (caught) {
      updateWorkbench({ error: caught instanceof Error ? caught.message : String(caught) });
    } finally {
      updateWorkbench({ scanning: false });
    }
  }

  function selectMaterialFile(file: MaterialFile) {
    activateMaterialPath(file.path, true);
    clearTransientState();
  }

  function openTaskMenu(event: React.MouseEvent, file: MaterialFile) {
    event.preventDefault();
    event.stopPropagation();
    updateRequest({ epubPath: file.path });
    updateWorkbench({ selectedTaskPath: file.path });
    setContextMenu({ x: event.clientX, y: event.clientY, file });
  }

  function renderContextMenu(file: MaterialFile, x: number, y: number) {
    const parsable = canGenerate(file);
    return (
      <div className="task-context-menu" style={{ left: x, top: y }} onClick={(event) => event.stopPropagation()}>
        <button disabled={!parsable || busy} type="button" onClick={() => void runMenuAction(() => generateMaterials(file.path))}>继续生成</button>
        <button disabled={busy || !scanResult?.files.some(canGenerate)} type="button" onClick={() => void runMenuAction(() => generateMaterials(firstParsablePath()))}>批量继续生成</button>
        <button type="button" onClick={() => void runMenuAction(() => openFile(file.path))}>打开文件</button>
        <button type="button" onClick={() => void runMenuAction(() => openFolder(file.path))}>打开文件夹</button>
        <button type="button" onClick={() => void runMenuAction(() => openMaterialFolder(file))}>打开素材文件夹</button>
        <button disabled type="button">生成双语字幕</button>
        <button disabled type="button">批量生成双语字幕</button>
        <button type="button" onClick={() => runMenuAction(() => clearTaskStatus(file.path))}>取消所选任务</button>
        <button type="button" onClick={() => runMenuAction(() => removeTask(file.path))}>从列表中移除</button>
        <button type="button" onClick={() => runMenuAction(() => clearTaskStatus(file.path))}>删除所选任务并清理文件</button>
        <button type="button" onClick={() => runMenuAction(clearAllTaskStatuses)}>批量取消任务</button>
        <button type="button" onClick={() => runMenuAction(clearAllTaskStatuses)}>批量删除任务并清理文件</button>
        <hr />
        <button disabled type="button">一键同步</button>
        <button disabled type="button">一键同步并删除原始文件</button>
      </div>
    );
  }

  async function runMenuAction(action: () => void | Promise<void>) {
    setContextMenu(null);
    try {
      await action();
    } catch (caught) {
      updateWorkbench({ error: caught instanceof Error ? caught.message : String(caught), copyState: "", exportState: "" });
    }
  }

  async function generateSelectedMaterials() {
    setActivePipelineStage("materials");
    const candidates = getMaterialPipelineCandidates();
    const target = candidates.find((file) => shouldGenerateMaterial(file));
    if (!target) {
      updateWorkbench({
        copyState: "",
        error: "",
        exportState: candidates.length > 0 ? "素材已存在，已按配置跳过生成。" : "没有可生成素材的任务。"
      });
      return;
    }
    await generateMaterials(target.path);
  }

  async function runAudioPipeline() {
    setActivePipelineStage("audio");
    const candidates = getAudioPipelineCandidates();
    if (candidates.length === 0) {
      updateWorkbench({ copyState: "", error: "请先勾选已生成素材的任务。", exportState: "" });
      return;
    }
    updateWorkbench({ busy: true, error: "", copyState: "", exportState: `开始生成 ${candidates.length} 个任务的音频。` });
    try {
      for (const file of candidates) {
        await setTaskAudioState(file.path, { audioStatus: "generating", audioProgress: 10, audioMessage: "正在准备音频" });
        try {
          const result = await frameworkApi.generateMaterialTaskAudio({ path: file.path, traceId: `${createTraceId()}-audio` });
          await setTaskAudioState(file.path, {
            audioStatus: "success",
            audioProgress: 100,
            audioOutputDir: result.outputDir,
            audioFile: result.audioFile,
            audioDurationMs: result.durationMs ?? null,
            audioChunks: result.chunks,
            audioMessage: "音频已生成"
          });
        } catch (caught) {
          const message = caught instanceof Error ? caught.message : String(caught);
          await setTaskAudioState(file.path, { audioStatus: "failed", audioProgress: 0, audioMessage: message });
          throw caught;
        }
      }
      updateWorkbench({ exportState: `音频生成完成：${candidates.length} 个任务。`, error: "" });
    } catch (caught) {
      updateWorkbench({ error: caught instanceof Error ? caught.message : String(caught), exportState: "" });
    } finally {
      updateWorkbench({ busy: false });
      await loadStoredTasks(settings.materialProfile.categoryName);
    }
  }

  async function runVideoPipeline() {
    setActivePipelineStage("video");
    const target = getVideoPipelineTarget();
    if (!target) {
      updateWorkbench({ copyState: "", error: "请先勾选或选择 EPUB 任务。", exportState: "" });
      return;
    }
    const path = target.path;
    const traceId = `${createTraceId()}-video`;
    updateWorkbench({ busy: true, error: "", copyState: "", exportState: "正在执行一键视频流水线。", currentTraceId: traceId });
    try {
      let current = target;
      if (shouldGenerateMaterial(current) || !current.materialOutputDir) {
        updateWorkbench({ exportState: "视频流水线：正在补生成素材。", error: "" });
        await generateMaterialsForVideo(path, traceId);
        await loadStoredTasks(settings.materialProfile.categoryName);
        current = findTaskByPath(path) ?? current;
      }
      if (shouldGenerateAudio(current) || !current.audioFile) {
        updateWorkbench({ exportState: "视频流水线：正在补生成音频。", error: "" });
        await setTaskAudioState(path, { audioStatus: "generating", audioProgress: 10, audioMessage: "正在准备音频" });
        const audio = await frameworkApi.generateMaterialTaskAudio({ path, traceId: `${traceId}-audio` });
        await setTaskAudioState(path, {
          audioStatus: "success",
          audioProgress: 100,
          audioOutputDir: audio.outputDir,
          audioFile: audio.audioFile,
          audioDurationMs: audio.durationMs ?? null,
          audioChunks: audio.chunks,
          audioMessage: "音频已生成"
        });
      }
      updateWorkbench({ exportState: "视频流水线：正在启动视频后台任务。", error: "" });
      await updateVideoState(path, { status: "generating", progress: 30, message: "视频任务启动中" });
      await frameworkApi.generateBookVideoPipeline({
        epubPath: path,
        traceId,
        allowPlaceholderVisuals: false,
        controlledProgrammaticVisuals: true,
        ignoreExistingVisualAssets: true
      });
      await updateVideoState(path, { status: "generating", progress: 40, message: "视频后台生成中，请到操作日志查看实际进度" });
      updateWorkbench({
        exportState: "视频后台任务已启动。实际进度请到“操作日志”菜单查看。",
        error: ""
      });
      await loadStoredTasks(settings.materialProfile.categoryName);
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      await updateVideoState(path, { status: "failed", progress: 0, message });
      updateWorkbench({ error: message, exportState: "" });
    } finally {
      updateWorkbench({ busy: false });
    }
  }

  async function generatePublishMaterials() {
    setActivePipelineStage("publish");
    const target = getVideoPipelineTarget();
    if (!target) {
      updateWorkbench({ copyState: "", error: "请先勾选或选择 EPUB 任务。", exportState: "" });
      return;
    }
    const traceId = `${createTraceId()}-publish`;
    updateWorkbench({ busy: true, error: "", copyState: "", exportState: "正在生成发布资料 Markdown。", currentTraceId: traceId });
    try {
      const result = await frameworkApi.generatePublishMaterials({ epubPath: target.path, traceId });
      updateWorkbench({
        exportState: `发布资料已生成：${result.markdownFile}`,
        error: ""
      });
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      updateWorkbench({ error: message, exportState: "" });
    } finally {
      updateWorkbench({ busy: false });
    }
  }

  async function generateMaterials(path = request.epubPath) {
    const traceId = createTraceId();
    const requestWithTrace = {
      ...request,
      epubPath: path,
      channelName: settings.materialProfile.channelName,
      language: settings.materialProfile.language,
      targetMinChars: settings.materialProfile.targetMinChars,
      targetMaxChars: settings.materialProfile.targetMaxChars,
      extraDirection: settings.materialProfile.extraDirection,
      traceId
    };
    updateWorkbench({ busy: true, error: "", copyState: "", exportState: "", currentTraceId: traceId });
    await updateTaskStatus(path, { status: "generating", progress: 0, message: "等待后端开始处理" });
    updateRequest({ traceId });
    try {
      const result = await frameworkApi.generateBookMaterials(requestWithTrace);
      const narrationChars = countHanChars(result.narration);
      updateWorkbench({
        materials: result,
        activeTab: "title"
      });
      await updateTaskStatus(path, { status: "success", progress: 100, narrationChars, message: "已完成" });
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      updateWorkbench({ error: message });
      await updateTaskStatus(path, { status: "failed", progress: 0, message });
    } finally {
      updateWorkbench({ busy: false });
    }
  }

  async function copyText(value: string) {
    await navigator.clipboard.writeText(value);
    updateWorkbench({ copyState: "已复制当前素材。" });
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
    updateWorkbench({ copyState: "已复制全部素材。" });
  }

  async function exportMaterials() {
    if (!materials) return;
    updateWorkbench({ exporting: true, error: "", exportState: "" });
    try {
      const result = await frameworkApi.exportBookMaterials({ outputDir, materials, traceId: currentTraceId || request.traceId });
      updateWorkbench({ exportState: `已导出 ${result.files.length} 个文件：${result.outputDir}` });
    } catch (caught) {
      updateWorkbench({ error: caught instanceof Error ? caught.message : String(caught) });
    } finally {
      updateWorkbench({ exporting: false });
    }
  }

  function clearTransientState() {
    updateWorkbench({ error: "", copyState: "", exportState: "" });
  }

  function activateMaterialPath(path: string, keepChecked = false) {
    updateRequest({ epubPath: path });
    updateWorkbench({ selectedTaskPath: path });
    if (!keepChecked) {
      setSelectedTaskPaths([]);
    }
  }

  function firstParsablePath() {
    return scanResult?.files.find(canGenerate)?.path ?? request.epubPath;
  }

  function getMaterialPipelineCandidates() {
    const files = scanResult?.files ?? [];
    const selected = selectedTaskPaths.length > 0
      ? files.filter((file) => selectedTaskSet.has(file.path))
      : files.filter((file) => file.path === request.epubPath || file.path === selectedTaskPath);
    const candidates = (selected.length > 0 ? selected : files).filter(canGenerate);
    return candidates;
  }

  function getAudioPipelineCandidates() {
    const files = scanResult?.files ?? [];
    const selected = selectedTaskPaths.length > 0
      ? files.filter((file) => selectedTaskSet.has(file.path))
      : files.filter((file) => file.path === request.epubPath || file.path === selectedTaskPath);
    return (selected.length > 0 ? selected : files)
      .filter((file) => file.status === "success" && Boolean(file.materialOutputDir))
      .filter(shouldGenerateAudio);
  }

  function hasVideoPipelineTarget() {
    return Boolean(getVideoPipelineTarget());
  }

  function getVideoPipelineTarget() {
    const files = scanResult?.files ?? [];
    if (selectedTaskPaths.length > 0) {
      return files.find((file) => selectedTaskSet.has(file.path) && canGenerate(file) && shouldGenerateVideo(file));
    }
    const requestPath = request.epubPath.trim();
    return files.find((file) => file.path === requestPath && canGenerate(file) && shouldGenerateVideo(file))
      ?? files.find((file) => file.path === selectedTaskPath && canGenerate(file) && shouldGenerateVideo(file))
      ?? (request.epubPath.trim() ? materialFileFromPath(request.epubPath.trim()) : undefined);
  }

  function findTaskByPath(path: string) {
    return useAppStore.getState().materialsWorkbench.scanResult?.files.find((file) => file.path === path);
  }

  async function generateMaterialsForVideo(path: string, traceId: string) {
    const requestWithTrace = {
      ...request,
      epubPath: path,
      channelName: settings.materialProfile.channelName,
      language: settings.materialProfile.language,
      targetMinChars: settings.materialProfile.targetMinChars,
      targetMaxChars: settings.materialProfile.targetMaxChars,
      extraDirection: settings.materialProfile.extraDirection,
      traceId: `${traceId}-materials`
    };
    await updateTaskStatus(path, { status: "generating", progress: 0, message: "等待后端开始处理" });
    const result = await frameworkApi.generateBookMaterials(requestWithTrace);
    const narrationChars = countHanChars(result.narration);
    updateWorkbench({ materials: result, activeTab: "title" });
    await updateTaskStatus(path, { status: "success", progress: 100, narrationChars, message: "已完成" });
  }

  async function setTaskAudioState(path: string, patch: Partial<Pick<MaterialFile, "audioStatus" | "audioProgress" | "audioOutputDir" | "audioFile" | "audioDurationMs" | "audioChunks" | "audioMessage">>) {
    const current = useAppStore.getState().materialsWorkbench;
    updateWorkbench({
      scanResult: current.scanResult
        ? {
            ...current.scanResult,
            files: current.scanResult.files.map((file) => (file.path === path ? { ...file, ...patch } : file))
          }
        : current.scanResult
    });
  }

  async function updateVideoState(path: string, state: { status: "pending" | "generating" | "success" | "failed"; progress: number; message: string }) {
    const current = useAppStore.getState().materialsWorkbench;
    updateWorkbench({
      scanResult: current.scanResult
        ? {
            ...current.scanResult,
            files: current.scanResult.files.map((file) =>
              file.path === path
                ? {
                    ...file,
                    status: state.status,
                    progress: state.progress,
                    message: state.message,
                    videoStatus: state.status,
                    videoProgress: state.progress,
                    videoMessage: state.message
                  }
                : file
            )
          }
        : current.scanResult
    });
    await updateTaskStatus(path, { status: state.status, progress: state.progress, message: state.message });
  }

  async function openFile(path: string) {
    const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
    await revealItemInDir(path);
  }

  async function openFolder(path: string) {
    const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
    await revealItemInDir(path);
  }

  async function openMaterialFolder(file: MaterialFile) {
    await frameworkApi.openMaterialOutputDir({ path: file.path });
  }

  async function loadStoredTasks(category: string) {
    try {
      const result = await frameworkApi.getMaterialTasks({ category });
      const requestPath = useAppStore.getState().materialsWorkbench.request.epubPath.trim();
      const selectedPatch = requestPath && result.files.some((file) => file.path === requestPath)
        ? { selectedTaskPath: requestPath }
        : {};
      updateWorkbench({ scanResult: result, fileStatuses: statusesFromFiles(result.files), ...selectedPatch });
      setSelectedTaskPaths((current) => current.filter((taskPath) => result.files.some((file) => file.path === taskPath)));
    } catch (caught) {
      updateWorkbench({ error: caught instanceof Error ? caught.message : String(caught) });
    }
  }

  async function updateTaskStatus(path: string, status: { status: "pending" | "generating" | "success" | "failed"; progress: number; narrationChars?: number | null; message?: string }) {
    const normalized = normalizeGenerationStatus(status);
    const current = useAppStore.getState().materialsWorkbench;
    updateWorkbench({
      fileStatuses: {
        ...current.fileStatuses,
        [path]: normalized
      },
      scanResult: current.scanResult
        ? {
            ...current.scanResult,
            files: current.scanResult.files.map((file) =>
              file.path === path
                ? {
                    ...file,
                    status: normalized.status,
                    progress: normalized.progress,
                    narrationChars: normalized.narrationChars,
                    message: normalized.message ?? ""
                  }
                : file
            )
          }
        : current.scanResult
    });
    const saved = await frameworkApi.updateMaterialTaskStatus({
      path,
      category: settings.materialProfile.categoryName,
      status: normalized.status,
      progress: normalized.progress,
      narrationChars: normalized.narrationChars ?? null,
      message: normalized.message
    });
    if (saved && typeof saved === "object") {
      const currentAfterSave = useAppStore.getState().materialsWorkbench;
      updateWorkbench({
        scanResult: currentAfterSave.scanResult
          ? {
              ...currentAfterSave.scanResult,
              files: currentAfterSave.scanResult.files.map((file) => (file.path === path ? { ...file, ...(saved as MaterialFile) } : file))
            }
          : currentAfterSave.scanResult
      });
    }
  }

  async function clearTaskStatus(path: string) {
    await frameworkApi.resetMaterialTasks({ path });
    const current = useAppStore.getState().materialsWorkbench;
    const resetFile = (file: MaterialFile): MaterialFile =>
      file.path === path
        ? {
            ...file,
            status: "pending",
            progress: 0,
            narrationChars: null,
            materialOutputDir: null,
            message: "",
            audioStatus: "pending",
            audioProgress: 0,
            audioOutputDir: null,
            audioFile: null,
            audioDurationMs: null,
            audioChunks: null,
            audioMessage: "",
            videoStatus: "pending",
            videoProgress: 0,
            videoFile: null,
            videoDurationMs: null,
            videoFileSize: null,
            videoMessage: ""
          }
        : file;
    const files = current.scanResult?.files.map(resetFile) ?? [];
    updateWorkbench({ fileStatuses: statusesFromFiles(files), scanResult: current.scanResult ? { ...current.scanResult, files } : current.scanResult });
  }

  async function clearAllTaskStatuses() {
    await frameworkApi.resetMaterialTasks({});
    const current = useAppStore.getState().materialsWorkbench;
    const files = current.scanResult?.files.map((file) => ({
      ...file,
      status: "pending" as const,
      progress: 0,
      narrationChars: null,
      materialOutputDir: null,
      message: "",
      audioStatus: "pending" as const,
      audioProgress: 0,
      audioOutputDir: null,
      audioFile: null,
      audioDurationMs: null,
      audioChunks: null,
      audioMessage: "",
      videoStatus: "pending" as const,
      videoProgress: 0,
      videoFile: null,
      videoDurationMs: null,
      videoFileSize: null,
      videoMessage: ""
    })) ?? [];
    updateWorkbench({ fileStatuses: statusesFromFiles(files), scanResult: current.scanResult ? { ...current.scanResult, files } : current.scanResult });
  }

  async function removeTask(path: string) {
    if (!scanResult) return;
    await frameworkApi.removeMaterialTask({ path });
    const current = useAppStore.getState().materialsWorkbench;
    const nextStatuses = { ...current.fileStatuses };
    delete nextStatuses[path];
    updateWorkbench({
      fileStatuses: nextStatuses,
      scanResult: {
        ...current.scanResult!,
        files: current.scanResult!.files.filter((file) => file.path !== path)
      },
      selectedTaskPath: selectedTaskPath === path ? "" : selectedTaskPath
    });
    setSelectedTaskPaths((currentPaths) => currentPaths.filter((taskPath) => taskPath !== path));
  }
}

function shouldGenerateMaterial(file: MaterialFile) {
  if (!useAppStore.getState().settings.pipelineProfile.skipExistingMaterials) return true;
  return needsMaterialGeneration(file);
}

function shouldGenerateAudio(file: MaterialFile) {
  if (!useAppStore.getState().settings.pipelineProfile.skipExistingAudio) return true;
  return file.audioStatus !== "success" || !file.audioFile || file.audioProgress < 100;
}

function shouldGenerateVideo(file: MaterialFile) {
  if (!useAppStore.getState().settings.pipelineProfile.skipExistingVideo) return true;
  return file.videoStatus !== "success" || !file.videoFile || file.videoProgress < 100;
}

function needsMaterialGeneration(file: MaterialFile) {
  return file.status !== "success" || typeof file.narrationChars !== "number" || file.narrationChars <= 0 || file.progress < 100 || !file.materialOutputDir;
}

const materialFileFilter = {
  name: "支持的素材文件",
  extensions: ["epub", "pdf", "txt", "docx"]
};

function statusFromFile(file: MaterialFile) {
  return normalizeGenerationStatus({
    status: file.status,
    progress: file.progress,
    narrationChars: file.narrationChars,
    message: file.message
  });
}

function statusesFromFiles(files: MaterialFile[]) {
  return Object.fromEntries(files.map((file) => [file.path, statusFromFile(file)]));
}

function normalizeGenerationStatus(status: { status: "pending" | "generating" | "success" | "failed"; progress: number; narrationChars?: number | null; message?: string }) {
  return {
    status: status.status,
    progress: clampProgress(status.progress),
    narrationChars: typeof status.narrationChars === "number" ? status.narrationChars : undefined,
    message: status.message ?? ""
  };
}

function clampProgress(value: number) {
  if (value >= 100) return 100;
  if (value >= 75) return 75;
  if (value >= 50) return 50;
  if (value >= 25) return 25;
  return 0;
}

function countHanChars(value: string) {
  return Array.from(value).filter((char) => /[\u4e00-\u9fff]/.test(char)).length;
}

function canGenerate(file: MaterialFile) {
  return file.extension === "epub" || file.extension === "txt";
}

function materialFileFromPath(path: string): MaterialFile {
  const normalized = path.trim();
  const parts = normalized.split(/[\\/]/);
  const name = parts[parts.length - 1] || normalized;
  const extension = name.includes(".") ? name.split(".").pop()?.toLowerCase() ?? "" : "";
  return {
    path: normalized,
    name,
    extension,
    size: 0,
    category: "",
    status: "pending",
    progress: 0,
    narrationChars: null,
    materialOutputDir: null,
    message: "",
    audioStatus: "pending",
    audioProgress: 0,
    audioOutputDir: null,
    audioFile: null,
    audioDurationMs: null,
    audioChunks: null,
    audioMessage: "",
    videoStatus: "pending",
    videoProgress: 0,
    videoFile: null,
    videoDurationMs: null,
    videoFileSize: null,
    videoMessage: ""
  };
}

function isSupportedMaterial(file: MaterialFile) {
  return file.extension === "epub" || file.extension === "txt" || file.extension === "docx" || file.extension === "pdf";
}

function formatExtension(file: MaterialFile) {
  return file.extension ? file.extension.toUpperCase() : "无扩展名";
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(2)} MiB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(2)} GiB`;
}

function formatGenerationStatus(
  generation: { status: "pending" | "generating" | "success" | "failed"; progress: number; narrationChars?: number; message?: string } | undefined,
  parsable: boolean,
  supported: boolean
) {
  if (!supported) return "不支持";
  if (!parsable) return "不可解析";
  if (!generation) return "待处理";
  if (generation.status === "generating") return "生成中";
  if (generation.status === "success") return "已完成";
  if (generation.status === "failed") return "失败";
  return "待处理";
}

function formatProgress(
  generation: { status: "pending" | "generating" | "success" | "failed"; progress: number } | undefined,
  parsable: boolean
) {
  if (!parsable) return "-";
  return `${Math.max(0, Math.min(100, generation?.progress ?? 0))}%`;
}

function formatNarrationChars(generation: { narrationChars?: number } | undefined) {
  return typeof generation?.narrationChars === "number" ? generation.narrationChars.toLocaleString() : "-";
}

function formatTaskName(name: string) {
  const chars = Array.from(name);
  if (chars.length <= 20) return name;
  return `${chars.slice(0, 18).join("")}...`;
}

function formatStageStatus(status: "pending" | "generating" | "success" | "failed") {
  if (status === "generating") return "生成中";
  if (status === "success") return "已完成";
  if (status === "failed") return "失败";
  return "待生成";
}

function formatVideoStageStatus(file: MaterialFile) {
  if (hasVideoDurationMismatch(file)) return "异常";
  return formatStageStatus(file.videoStatus);
}

function formatRawProgress(progress: number) {
  return `${Math.max(0, Math.min(100, progress || 0))}%`;
}

function formatVideoProgress(file: MaterialFile) {
  if (hasVideoDurationMismatch(file)) return "-";
  return formatRawProgress(file.videoProgress);
}

function formatAudioDuration(value?: number | null) {
  if (!value || value <= 0) return "-";
  const totalSeconds = Math.round(value / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}时${minutes}分${seconds}秒`;
  if (minutes > 0) return `${minutes}分${seconds}秒`;
  return `${seconds}秒`;
}

function formatOptionalBytes(value?: number | null) {
  return typeof value === "number" && value > 0 ? formatBytes(value) : "-";
}

function getGenerationStatusClass(
  generation: { status: "pending" | "generating" | "success" | "failed" } | undefined,
  parsable: boolean,
  supported: boolean
) {
  if (!supported || !parsable) return "generation-status muted";
  if (!generation) return "generation-status pending";
  return `generation-status ${generation.status}`;
}

function getVideoStatusClass(file: MaterialFile) {
  if (hasVideoDurationMismatch(file)) return "generation-status failed";
  return getGenerationStatusClass({ status: file.videoStatus }, true, true);
}

function hasVideoDurationMismatch(file: MaterialFile) {
  if (file.videoStatus !== "success") return false;
  if (!file.audioDurationMs || !file.videoDurationMs) return false;
  return file.audioDurationMs > 60_000 && file.videoDurationMs < file.audioDurationMs * 0.8;
}

function createTraceId() {
  const stamp = new Date()
    .toISOString()
    .replace(/[-:]/g, "")
    .replace(/\.\d{3}Z$/, "")
    .replace("T", "-");
  const random = Math.random().toString(16).slice(2, 8);
  return `materials-${stamp}-${random}`;
}
