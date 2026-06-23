import { ListChecks } from "lucide-react";
import { useEffect, useMemo } from "react";
import { Panel, SectionTitle } from "@/pages/primitives";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import type { MaterialFile } from "@/types";

type StepStatus = "PENDING" | "RUNNING" | "SUCCESS" | "FAILED";
type StageKey = "material" | "audio" | "video";
type TaskStatus = MaterialFile["status"];

interface StepRow {
  order: number;
  code: string;
  name: string;
  status: StepStatus;
  progress: number;
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
  { code: "MAT_A", name: text("\u751f\u6210 A\uff1a\u89e3\u6790\u4e66\u7c4d"), stage: "material", threshold: 10, detail: sourceDetail },
  { code: "MAT_B", name: text("\u751f\u6210 B\uff1a\u6807\u9898\u7b80\u4ecb\u6807\u7b7e"), stage: "material", threshold: 35, detail: aiMaterialDetail },
  { code: "MAT_C", name: text("\u751f\u6210 C\uff1a\u65c1\u767d\u6587\u672c"), stage: "material", threshold: 60, detail: narrationDetail },
  { code: "MAT_D", name: text("\u751f\u6210 D\uff1a\u5b57\u5e55\u6587\u672c"), stage: "material", threshold: 75, detail: subtitleMaterialDetail },
  { code: "MAT_SAVE", name: text("\u4fdd\u5b58\u7d20\u6750\u5305"), stage: "material", threshold: 100, detail: materialDirDetail },
  { code: "AUD_TEXT", name: text("\u8bfb\u53d6\u65c1\u767d\u6587\u672c"), stage: "audio", threshold: 10, detail: audioTextDetail },
  { code: "AUD_SPLIT", name: text("\u62c6\u5206\u97f3\u9891\u7247\u6bb5"), stage: "audio", threshold: 35, detail: audioChunkDetail },
  { code: "AUD_TTS", name: text("\u751f\u6210\u8bed\u97f3\u7247\u6bb5"), stage: "audio", threshold: 85, detail: audioChunkDetail },
  { code: "AUD_MERGE", name: text("\u5408\u6210\u6700\u7ec8\u97f3\u9891"), stage: "audio", threshold: 100, detail: audioFileDetail },
  { code: "VID_PREP", name: text("\u51c6\u5907\u89c6\u9891\u6d41\u6c34\u7ebf"), stage: "video", threshold: 20, detail: videoMessageDetail },
  { code: "VID_COVER", name: text("\u751f\u6210\u5c01\u9762"), stage: "video", threshold: 35, detail: coverDetail },
  { code: "VID_IMAGES", name: text("\u751f\u6210\u56fe\u7247"), stage: "video", threshold: 45, detail: imageDetail },
  { code: "VID_SUBTITLE", name: text("\u751f\u6210\u5b57\u5e55"), stage: "video", threshold: 60, detail: subtitleDetail },
  { code: "VID_NO_SUB", name: text("\u751f\u6210\u65e0\u5b57\u5e55\u89c6\u9891"), stage: "video", threshold: 75, detail: noSubtitleVideoDetail },
  { code: "VID_HARD_SUB", name: text("\u751f\u6210\u786c\u5b57\u5e55\u89c6\u9891"), stage: "video", threshold: 90, detail: hardSubtitleVideoDetail },
  { code: "VID_REGISTER", name: text("\u767b\u8bb0\u89c6\u9891\u4ea7\u7269"), stage: "video", threshold: 100, detail: videoFileDetail }
];

export function AudioPage() {
  const settings = useAppStore((state) => state.settings);
  const currentTraceId = useAppStore((state) => state.materialsWorkbench.currentTraceId);
  const selectedTaskPath = useAppStore((state) => state.materialsWorkbench.selectedTaskPath);
  const scanResult = useAppStore((state) => state.materialsWorkbench.scanResult);
  const updateWorkbench = useAppStore((state) => state.updateMaterialsWorkbench);

  useEffect(() => {
    let canceled = false;
    async function load() {
      try {
        const result = await frameworkApi.getMaterialTasks({ category: settings.materialProfile.categoryName });
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

  const currentTask = useMemo(() => pickCurrentTask(scanResult?.files ?? [], selectedTaskPath), [scanResult?.files, selectedTaskPath]);
  const steps = useMemo(() => buildStepRows(currentTask), [currentTask]);
  const successCount = steps.filter((step) => step.status === "SUCCESS").length;
  const failedCount = steps.filter((step) => step.status === "FAILED").length;
  const runningCount = steps.filter((step) => step.status === "RUNNING").length;
  const summary = currentTask ? buildSummary(currentTask) : text("\u6682\u65e0\u4efb\u52a1");

  return (
    <div className="page steps-page">
      <Panel className="steps-panel">
        <SectionTitle icon={<ListChecks size={16} />} title={text("\u6b65\u9aa4\u7edf\u8ba1")} inline />
        <div className="step-summary-grid">
          <SummaryItem label={text("\u5f53\u524d\u4efb\u52a1")} value={currentTask ? formatTaskTitle(currentTask.name) : "-"} />
          <SummaryItem label={text("\u603b\u6b65\u9aa4")} value={String(steps.length)} />
          <SummaryItem label={text("\u6b65\u9aa4\u8fdb\u5ea6")} value={`${successCount} ${text("\u6210\u529f")} / ${failedCount} ${text("\u5931\u8d25")} / ${runningCount} ${text("\u8fdb\u884c\u4e2d")}`} />
          <SummaryItem label={text("\u4efb\u52a1\u6458\u8981")} value={summary} />
          <SummaryItem label={text("\u4efb\u52a1 ID")} value={currentTraceId || "-"} />
          <SummaryItem label={text("\u6574\u4f53\u8fdb\u5ea6")} value={currentTask ? `${estimateOverallProgress(steps)}%` : "-"} />
        </div>
      </Panel>

      <Panel className="steps-panel">
        <SectionTitle icon={<ListChecks size={16} />} title={text("\u6b65\u9aa4\u8ddf\u8e2a")} inline />
        <div className="steps-table">
          <div className="steps-row header">
            <span>{text("\u5e8f\u53f7")}</span>
            <span>{text("\u6b65\u9aa4\u7f16\u7801")}</span>
            <span>{text("\u6b65\u9aa4\u540d\u79f0")}</span>
            <span>{text("\u72b6\u6001")}</span>
            <span>{text("\u8fdb\u5ea6")}</span>
            <span>{text("\u8bf4\u660e")}</span>
          </div>
          {steps.length > 0 ? (
            steps.map((step) => (
              <div className="steps-row" key={step.code}>
                <span>{step.order}</span>
                <span>{step.code}</span>
                <span>{step.name}</span>
                <span className={`step-status ${step.status.toLowerCase()}`}>{formatStepStatus(step.status)}</span>
                <span>{step.progress}%</span>
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

function pickCurrentTask(files: MaterialFile[], selectedTaskPath: string) {
  return (
    files.find((file) => file.path === selectedTaskPath) ??
    files.find((file) => file.status === "generating" || file.audioStatus === "generating" || file.videoStatus === "generating") ??
    files[0] ??
    null
  );
}

function buildStepRows(file: MaterialFile | null): StepRow[] {
  if (!file) return [];
  return STEP_SPECS.map((spec, index) => {
    const stage = getStage(file, spec.stage);
    const status = mapSubStepStatus(stage.status, stage.progress, spec.threshold, spec.stage, file);
    return {
      order: index + 1,
      code: spec.code,
      name: spec.name,
      status,
      progress: status === "SUCCESS" ? 100 : status === "PENDING" ? 0 : clampProgress(stage.progress),
      detail: stage.message || spec.detail(file)
    };
  });
}

function getStage(file: MaterialFile, stage: StageKey) {
  if (stage === "audio") return { status: file.audioStatus, progress: clampProgress(file.audioProgress), message: file.audioMessage };
  if (stage === "video") return { status: file.videoStatus, progress: clampProgress(file.videoProgress), message: file.videoMessage };
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
    return progress >= previous ? "RUNNING" : "PENDING";
  }
  return "PENDING";
}

function failedStageThreshold(stage: StageKey, file: MaterialFile) {
  const progress = stage === "audio" ? file.audioProgress : stage === "video" ? file.videoProgress : file.progress;
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

function formatStepStatus(status: StepStatus) {
  if (status === "SUCCESS") return "SUCCESS";
  if (status === "FAILED") return "FAILED";
  if (status === "RUNNING") return "RUNNING";
  return "PENDING";
}

function formatTaskTitle(name: string) {
  const chars = Array.from(name);
  return chars.length <= 24 ? name : `${chars.slice(0, 22).join("")}...`;
}

function buildSummary(file: MaterialFile) {
  if (file.videoStatus === "success") return text("\u89c6\u9891\u751f\u6210\u5b8c\u6210");
  if (file.videoStatus === "generating") return text("\u6b63\u5728\u751f\u6210\u89c6\u9891\u4ea7\u7269");
  if (file.audioStatus === "generating") return text("\u6b63\u5728\u751f\u6210\u97f3\u9891");
  if (file.status === "generating") return text("\u6b63\u5728\u751f\u6210\u7d20\u6750");
  if (file.status === "failed" || file.audioStatus === "failed" || file.videoStatus === "failed") return text("\u4efb\u52a1\u5931\u8d25");
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

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MiB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(1)} GiB`;
}
