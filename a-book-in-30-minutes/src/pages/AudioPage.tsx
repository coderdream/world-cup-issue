import { ListChecks } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Panel, SectionTitle } from "@/pages/primitives";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import type { MaterialFile, MaterialTaskStep } from "@/types";

type StepStatus = "PENDING" | "RUNNING" | "SUCCESS" | "FAILED";
type StageKey = "text" | "image" | "audio" | "subtitle" | "video" | "publish";
type TaskStatus = MaterialFile["status"];

interface StepRow {
  order: number;
  taskName: string;
  code: string;
  name: string;
  status: StepStatus;
  progress: number;
  duration: string;
  detail: string;
}

interface StepSpec {
  code: string;
  name: string;
  stage: StageKey;
  threshold: number;
  detail: (file: MaterialFile) => string;
}

const text = (value: string) => value;

const STEP_SPECS: StepSpec[] = [
  { code: "A-01", name: text("文本：解析书籍"), stage: "text", threshold: 10, detail: sourceDetail },
  { code: "A-02", name: text("文本：标题简介标签"), stage: "text", threshold: 35, detail: aiMaterialDetail },
  { code: "A-03", name: text("文本：旁白文稿"), stage: "text", threshold: 70, detail: narrationDetail },
  { code: "A-04", name: text("文本：保存素材包"), stage: "text", threshold: 100, detail: materialDirDetail },
  { code: "B-01", name: text("图片：生成封面"), stage: "image", threshold: 35, detail: coverDetail },
  { code: "B-02", name: text("图片：生成分镜图"), stage: "image", threshold: 45, detail: imageDetail },
  { code: "C-01", name: text("音频：读取旁白"), stage: "audio", threshold: 10, detail: audioTextDetail },
  { code: "C-02", name: text("音频：拆分片段"), stage: "audio", threshold: 35, detail: audioChunkDetail },
  { code: "C-03", name: text("音频：生成语音"), stage: "audio", threshold: 85, detail: audioChunkDetail },
  { code: "C-04", name: text("音频：合成音频"), stage: "audio", threshold: 100, detail: audioFileDetail },
  { code: "D-01", name: text("字幕：生成中文字幕"), stage: "subtitle", threshold: 55, detail: subtitleMaterialDetail },
  { code: "D-02", name: text("字幕：生成双语字幕"), stage: "subtitle", threshold: 65, detail: subtitleDetail },
  { code: "E-01", name: text("视频：准备流水线"), stage: "video", threshold: 20, detail: videoMessageDetail },
  { code: "E-02", name: text("视频：生成无字幕母版"), stage: "video", threshold: 75, detail: noSubtitleVideoDetail },
  { code: "E-03", name: text("视频：生成硬字幕版"), stage: "video", threshold: 90, detail: hardSubtitleVideoDetail },
  { code: "E-04", name: text("视频：登记视频产物"), stage: "video", threshold: 100, detail: videoFileDetail },
  { code: "F-01", name: text("发布：生成发布资料"), stage: "publish", threshold: 100, detail: publishDetail }
];

export function AudioPage() {
  const settings = useAppStore((state) => state.settings);
  const currentTraceId = useAppStore((state) => state.materialsWorkbench.currentTraceId);
  const requestPath = useAppStore((state) => state.materialsWorkbench.request.epubPath);
  const selectedTaskPath = useAppStore((state) => state.materialsWorkbench.selectedTaskPath);
  const selectedTaskPaths = useAppStore((state) => state.materialsWorkbench.selectedTaskPaths);
  const scanResult = useAppStore((state) => state.materialsWorkbench.scanResult);
  const updateWorkbench = useAppStore((state) => state.updateMaterialsWorkbench);
  const [persistedStepsByPath, setPersistedStepsByPath] = useState<Record<string, MaterialTaskStep[]>>({});
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    let canceled = false;
    async function load() {
      try {
        const categoryResult = await frameworkApi.getMaterialTasks({ category: settings.materialProfile.categoryName });
        const normalizedRequestPath = useAppStore.getState().materialsWorkbench.request.epubPath.trim();
        const requestTask = normalizedRequestPath && !categoryResult.files.some((file) => file.path === normalizedRequestPath)
          ? await frameworkApi.getMaterialTask({ path: normalizedRequestPath })
          : null;
        const fallbackTask = normalizedRequestPath ? materialFileFromPath(normalizedRequestPath) : null;
        const result = requestTask || fallbackTask
          ? { ...categoryResult, files: [requestTask ?? fallbackTask!, ...categoryResult.files] }
          : categoryResult;
        if (!canceled) {
          updateWorkbench({ scanResult: result });
        }
      } catch {
        // The page is read-only; keep the current view if polling fails.
      }
    }

    void load();
    const timer = window.setInterval(() => void load(), 2000);
    return () => {
      canceled = true;
      window.clearInterval(timer);
    };
  }, [settings.materialProfile.categoryName, updateWorkbench]);

  const trackedTasks = useMemo(
    () => pickTrackedTasks(scanResult?.files ?? [], requestPath, selectedTaskPath, selectedTaskPaths),
    [scanResult?.files, requestPath, selectedTaskPath, selectedTaskPaths]
  );

  useEffect(() => {
    const timer = window.setInterval(() => setNowMs(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    let canceled = false;
    async function loadSteps() {
      if (trackedTasks.length === 0) {
        setPersistedStepsByPath({});
        return;
      }
      try {
        const pairs = await Promise.all(
          trackedTasks.map(async (task) => {
            const result = await frameworkApi.getMaterialTaskSteps({ path: task.path });
            return [task.path, result.steps] as const;
          })
        );
        if (!canceled) setPersistedStepsByPath(Object.fromEntries(pairs));
      } catch {
        if (!canceled) setPersistedStepsByPath({});
      }
    }

    void loadSteps();
    const timer = window.setInterval(() => void loadSteps(), 2000);
    return () => {
      canceled = true;
      window.clearInterval(timer);
    };
  }, [trackedTasks]);

  const steps = useMemo(
    () => trackedTasks.flatMap((task, taskIndex) => buildStepRows(task, persistedStepsByPath[task.path] ?? [], nowMs, taskIndex)),
    [nowMs, persistedStepsByPath, trackedTasks]
  );
  const successCount = steps.filter((step) => step.status === "SUCCESS").length;
  const failedCount = steps.filter((step) => step.status === "FAILED").length;
  const runningCount = steps.filter((step) => step.status === "RUNNING").length;
  const summary = trackedTasks.length === 1 ? buildSummary(trackedTasks[0]) : trackedTasks.length > 1 ? text("按勾选任务展开步骤") : text("\u6682\u65e0\u4efb\u52a1");
  const taskLabel = selectedTaskPaths.length > 0 ? `已勾选 ${trackedTasks.length} 个任务` : trackedTasks[0] ? formatTaskTitle(trackedTasks[0].name) : "-";

  return (
    <div className="page steps-page">
      <Panel className="steps-panel">
        <SectionTitle icon={<ListChecks size={16} />} title={text("\u6b65\u9aa4\u7edf\u8ba1")} inline />
        <div className="step-summary-grid">
          <SummaryItem label={text("\u5f53\u524d\u4efb\u52a1")} value={taskLabel} />
          <SummaryItem label={text("\u603b\u6b65\u9aa4")} value={String(steps.length)} />
          <SummaryItem label={text("\u6b65\u9aa4\u8fdb\u5ea6")} value={`${successCount} ${text("\u6210\u529f")} / ${failedCount} ${text("\u5931\u8d25")} / ${runningCount} ${text("\u8fdb\u884c\u4e2d")}`} />
          <SummaryItem label={text("\u4efb\u52a1\u6458\u8981")} value={summary} />
          <SummaryItem label={text("\u4efb\u52a1 ID")} value={currentTraceId || "-"} />
          <SummaryItem label={text("\u6574\u4f53\u8fdb\u5ea6")} value={trackedTasks.length > 0 ? `${estimateOverallProgress(steps)}%` : "-"} />
        </div>
      </Panel>

      <Panel className="steps-panel">
        <SectionTitle icon={<ListChecks size={16} />} title={text("\u6b65\u9aa4\u8ddf\u8e2a")} inline />
        <div className="steps-table">
          <div className="steps-row header">
            <span>{text("\u5e8f\u53f7")}</span>
            <span>{text("任务")}</span>
            <span>{text("\u6b65\u9aa4\u7f16\u7801")}</span>
            <span>{text("\u6b65\u9aa4\u540d\u79f0")}</span>
            <span>{text("\u72b6\u6001")}</span>
            <span>{text("\u8fdb\u5ea6")}</span>
            <span>{text("耗时")}</span>
            <span>{text("\u8bf4\u660e")}</span>
          </div>
          {steps.length > 0 ? (
            steps.map((step) => (
              <div className="steps-row" key={`${step.taskName}-${step.code}-${step.order}`}>
                <span>{step.order}</span>
                <span title={step.taskName}>{formatTaskTitle(step.taskName)}</span>
                <span>{step.code}</span>
                <span>{step.name}</span>
                <span className={`step-status ${step.status.toLowerCase()}`}>{formatStepStatus(step.status)}</span>
                <span>{step.progress}%</span>
                <span>{step.duration}</span>
                <span title={step.detail}>{step.detail || "-"}</span>
              </div>
            ))
          ) : (
            <div className="empty-result compact">
              <ListChecks size={28} />
              <b>{text("\u6682\u65e0\u6b65\u9aa4")}</b>
              <span>{text("\u5728\u201c\u6d41\u6c34\u7ebf\u201d\u4e2d\u9009\u62e9\u6216\u6267\u884c\u4efb\u52a1\u540e\uff0c\u8fd9\u91cc\u4f1a\u663e\u793a\u7d20\u6750\u3001\u97f3\u9891\u3001\u56fe\u7247\u3001\u5c01\u9762\u3001\u5b57\u5e55\u548c\u89c6\u9891\u4ea7\u7269\u6b65\u9aa4\u3002")}</span>
            </div>
          )}
        </div>
      </Panel>
    </div>
  );
}

function SummaryItem({ label, value }: { label: string; value: string }) {
  return (
    <>
      <span>{label}</span>
      <b title={value}>{value}</b>
    </>
  );
}

function pickTrackedTasks(files: MaterialFile[], requestPath: string, selectedTaskPath: string, selectedTaskPaths: string[]) {
  const normalizedRequestPath = requestPath.trim();
  if (selectedTaskPaths.length > 0) {
    return selectedTaskPaths.map((path) => files.find((file) => file.path === path) ?? materialFileFromPath(path));
  }
  const current =
    files.find((file) => file.path === selectedTaskPath) ??
    files.find((file) => file.path === normalizedRequestPath) ??
    files.find(
      (file) =>
        file.status === "generating" ||
        file.imageStatus === "generating" ||
        file.audioStatus === "generating" ||
        file.subtitleStatus === "generating" ||
        file.videoStatus === "generating"
    ) ??
    files[0] ??
    (normalizedRequestPath ? materialFileFromPath(normalizedRequestPath) : null);
  return current ? [current] : [];
}

function buildStepRows(file: MaterialFile, persistedSteps: MaterialTaskStep[], nowMs: number, taskIndex: number): StepRow[] {
  const persistedByCode = new Map(persistedSteps.map((step) => [step.stepCode, step]));
  return STEP_SPECS.map((spec, index) => {
    const persisted = persistedByCode.get(spec.code);
    if (persisted) {
      const status = mapPersistedStepStatus(persisted.status);
      return {
        order: taskIndex * STEP_SPECS.length + index + 1,
        taskName: file.name,
        code: spec.code,
        name: persisted.stepName || spec.name,
        status,
        progress: status === "SUCCESS" ? 100 : status === "PENDING" ? 0 : clampProgress(persisted.progress),
        duration: formatStepDuration(persisted, nowMs),
        detail: persisted.detail || spec.detail(file)
      };
    }
    const stage = getStage(file, spec.stage);
    const status = mapSubStepStatus(stage.status, stage.progress, spec.threshold, spec.stage, file);
    return {
      order: taskIndex * STEP_SPECS.length + index + 1,
      taskName: file.name,
      code: spec.code,
      name: spec.name,
      status,
      progress: status === "SUCCESS" ? 100 : status === "PENDING" ? 0 : clampProgress(stage.progress),
      duration: "-",
      detail: stage.message || spec.detail(file)
    };
  });
}

function mapPersistedStepStatus(status: MaterialTaskStep["status"]): StepStatus {
  if (status === "success") return "SUCCESS";
  if (status === "failed") return "FAILED";
  if (status === "generating") return "RUNNING";
  return "PENDING";
}

function getStage(file: MaterialFile, stage: StageKey) {
  if (stage === "audio") return { status: file.audioStatus, progress: clampProgress(file.audioProgress), message: file.audioMessage };
  if (stage === "image") return { status: file.imageStatus, progress: clampProgress(file.imageProgress), message: file.imageMessage };
  if (stage === "subtitle") return { status: file.subtitleStatus, progress: clampProgress(file.subtitleProgress), message: file.subtitleMessage };
  if (stage === "video") return { status: file.videoStatus, progress: clampProgress(file.videoProgress), message: file.videoMessage };
  if (stage === "publish") return { status: file.videoFile ? "success" : file.videoStatus, progress: file.videoFile ? 100 : clampProgress(file.videoProgress), message: file.videoFile ? "等待生成发布资料" : file.videoMessage };
  if (file.materialOutputDir) return { status: "success" as TaskStatus, progress: 100, message: file.message };
  return { status: file.status, progress: clampProgress(file.progress), message: file.message };
}

function mapSubStepStatus(status: TaskStatus, progress: number, threshold: number, stage: StageKey, file: MaterialFile): StepStatus {
  if (status === "success" || progress >= threshold) return "SUCCESS";
  if (status === "failed") {
    const failedThreshold = failedStageThreshold(stage, file);
    return threshold === failedThreshold ? "FAILED" : threshold < failedThreshold ? "SUCCESS" : "PENDING";
  }
  if (status === "generating") {
    const previous = previousStageThreshold(stage, threshold);
    return progress >= previous || previous === 0 ? "RUNNING" : "PENDING";
  }
  return "PENDING";
}

function failedStageThreshold(stage: StageKey, file: MaterialFile) {
  const progress =
    stage === "audio"
      ? file.audioProgress
      : stage === "image"
        ? file.imageProgress
        : stage === "subtitle"
          ? file.subtitleProgress
          : stage === "video"
            ? file.videoProgress
            : file.progress;
  const thresholds = STEP_SPECS.filter((step) => step.stage === stage).map((step) => step.threshold);
  return thresholds.find((threshold) => threshold >= clampProgress(progress)) ?? thresholds[thresholds.length - 1] ?? 100;
}

function previousStageThreshold(stage: StageKey, threshold: number) {
  const values = STEP_SPECS.filter((step) => step.stage === stage && step.threshold < threshold).map((step) => step.threshold);
  return values.length === 0 ? 0 : Math.max(...values);
}

function clampProgress(value: number | null | undefined) {
  if (typeof value !== "number" || Number.isNaN(value)) return 0;
  return Math.max(0, Math.min(100, Math.round(value)));
}

function estimateOverallProgress(steps: StepRow[]) {
  if (steps.length === 0) return 0;
  return Math.round(steps.reduce((sum, step) => sum + step.progress, 0) / steps.length);
}

function formatStepDuration(step: MaterialTaskStep, nowMs: number) {
  if (typeof step.elapsedMs === "number" && step.elapsedMs > 0) return formatDurationMs(step.elapsedMs);
  if (step.startedAt && step.finishedAt) {
    const started = parseSqliteDateTime(step.startedAt);
    const finished = parseSqliteDateTime(step.finishedAt);
    if (started && finished && finished.getTime() > started.getTime()) {
      return formatDurationMs(finished.getTime() - started.getTime());
    }
  }
  if (step.status !== "generating" || !step.startedAt) return typeof step.elapsedMs === "number" ? formatDurationMs(step.elapsedMs) : "-";
  const started = parseSqliteDateTime(step.startedAt);
  if (!started) return "-";
  return formatDurationMs(Math.max(0, nowMs - started.getTime()));
}

function parseSqliteDateTime(value: string) {
  const parsed = new Date(value.replace(" ", "T"));
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

function formatDurationMs(value: number) {
  const totalMs = Math.max(0, Math.round(value));
  const minutes = Math.floor(totalMs / 60000);
  const seconds = Math.floor((totalMs % 60000) / 1000);
  const millis = totalMs % 1000;
  return `${minutes.toString().padStart(2, "0")}分${seconds.toString().padStart(2, "0")}.${millis.toString().padStart(3, "0")}秒`;
}

function formatStepStatus(status: StepStatus) {
  if (status === "SUCCESS") return "成功";
  if (status === "FAILED") return "失败";
  if (status === "RUNNING") return "进行中";
  return "待处理";
}

function formatTaskTitle(name: string) {
  const chars = Array.from(name);
  return chars.length <= 24 ? name : `${chars.slice(0, 22).join("")}...`;
}

function buildSummary(file: MaterialFile) {
  if (file.videoStatus === "success") return text("\u89c6\u9891\u751f\u6210\u5b8c\u6210");
  if (file.videoStatus === "generating") return text("\u6b63\u5728\u751f\u6210\u89c6\u9891\u4ea7\u7269");
  if (file.subtitleStatus === "generating") return text("\u6b63\u5728\u751f\u6210\u5b57\u5e55");
  if (file.imageStatus === "generating") return text("\u6b63\u5728\u751f\u6210\u56fe\u7247");
  if (file.audioStatus === "generating") return text("\u6b63\u5728\u751f\u6210\u97f3\u9891");
  if (file.status === "generating") return text("\u6b63\u5728\u751f\u6210\u7d20\u6750");
  if (file.status === "failed" || file.imageStatus === "failed" || file.audioStatus === "failed" || file.subtitleStatus === "failed" || file.videoStatus === "failed") return text("\u4efb\u52a1\u5931\u8d25");
  return text("\u7b49\u5f85\u4e0b\u4e00\u6b65");
}

function sourceDetail(file: MaterialFile) {
  return `${text("\u6e90\u6587\u4ef6\uff1a")}${file.name}`;
}

function aiMaterialDetail(file: MaterialFile) {
  if (file.message) return file.message;
  return text("\u8bf7\u6c42 AI \u751f\u6210\u7d20\u6750 JSON");
}

function materialDirDetail(file: MaterialFile) {
  if (file.materialOutputDir) return `${text("\u7d20\u6750\u76ee\u5f55\uff1a")}${file.materialOutputDir}`;
  return text("\u7b49\u5f85\u4fdd\u5b58\u7d20\u6750\u5305");
}

function narrationDetail(file: MaterialFile) {
  if (file.narrationChars) return `${text("\u65c1\u767d\u5b57\u6570\uff1a")}${file.narrationChars}`;
  return text("\u7b49\u5f85\u751f\u6210\u65c1\u767d\u6587\u672c");
}

function subtitleMaterialDetail(file: MaterialFile) {
  if (file.narrationChars) return text("\u5df2\u6839\u636e\u65c1\u767d\u5207\u5206\u5b57\u5e55");
  return text("\u7b49\u5f85\u751f\u6210\u5b57\u5e55\u6587\u672c");
}

function audioTextDetail(file: MaterialFile) {
  return file.materialOutputDir ? text("\u4ece\u7d20\u6750\u5305\u8bfb\u53d6 narration.txt") : text("\u7b49\u5f85\u7d20\u6750\u5305");
}

function audioChunkDetail(file: MaterialFile) {
  if (file.audioChunks) return `${text("\u97f3\u9891\u5206\u6bb5\uff1a")}${file.audioChunks}`;
  return text("\u7b49\u5f85\u8bed\u97f3\u5206\u6bb5");
}

function audioFileDetail(file: MaterialFile) {
  if (file.audioFile) return `${text("\u97f3\u9891\u6587\u4ef6\uff1a")}${file.audioFile}`;
  return text("\u7b49\u5f85\u6700\u7ec8\u97f3\u9891");
}

function videoMessageDetail(file: MaterialFile) {
  return file.videoMessage || text("\u7b49\u5f85\u89c6\u9891\u6d41\u6c34\u7ebf\u542f\u52a8");
}

function coverDetail(file: MaterialFile) {
  return file.materialOutputDir ? text("\u8f93\u51fa\u5c01\u9762\u5230 output \u76ee\u5f55") : text("\u7b49\u5f85\u7d20\u6750\u76ee\u5f55");
}

function imageDetail(file: MaterialFile) {
  return file.materialOutputDir ? text("\u8f93\u51fa\u6b63\u7247\u56fe\u7247\u548c visual timeline") : text("\u7b49\u5f85\u56fe\u7247\u7d20\u6750");
}

function subtitleDetail(file: MaterialFile) {
  return file.materialOutputDir ? text("\u751f\u6210 srt/ass \u5b57\u5e55\u5e76\u6821\u51c6\u65f6\u95f4\u8f74") : text("\u7b49\u5f85\u65c1\u767d\u6587\u672c");
}

function noSubtitleVideoDetail(file: MaterialFile) {
  if (file.videoFile && !file.videoMessage.includes(text("\u786c\u5b57\u5e55"))) return `${text("\u89c6\u9891\u6587\u4ef6\uff1a")}${file.videoFile}`;
  return text("\u7b49\u5f85\u751f\u6210\u65e0\u5b57\u5e55\u89c6\u9891");
}

function hardSubtitleVideoDetail(file: MaterialFile) {
  if (file.videoFile) return `${text("\u6700\u7ec8\u89c6\u9891\uff1a")}${file.videoFile}`;
  return text("\u7b49\u5f85\u70e7\u5f55\u786c\u5b57\u5e55");
}

function videoFileDetail(file: MaterialFile) {
  if (file.videoFile) return `${text("\u89c6\u9891\u6587\u4ef6\uff1a")}${file.videoFile}`;
  if (file.videoFileSize) return `${text("\u89c6\u9891\u5927\u5c0f\uff1a")}${formatBytes(file.videoFileSize)}`;
  return text("\u7b49\u5f85\u767b\u8bb0\u89c6\u9891\u4ea7\u7269");
}

function publishDetail(file: MaterialFile) {
  if (file.videoFile) return text("视频已就绪，可生成发布资料");
  return text("等待最终视频产物");
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MiB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(1)} GiB`;
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
    imageStatus: "pending",
    imageProgress: 0,
    imageOutputDir: null,
    imageMessage: "",
    subtitleStatus: "pending",
    subtitleProgress: 0,
    subtitleFile: null,
    subtitleMessage: "",
    videoStatus: "pending",
    videoProgress: 0,
    videoFile: null,
    videoDurationMs: null,
    videoFileSize: null,
    videoMessage: ""
  };
}
