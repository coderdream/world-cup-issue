import { APP_VERSION } from "@/config/app";
import { defaultSettings } from "@/store/defaults";
import type {
  AiGenerateResult,
  AiTestResult,
  AppSettings,
  AppStatePayload,
  FeishuSendResult,
  GetOperationLogsResult,
  RunWorkflowResult,
  SkillConfigEntry,
  UpdateInfo,
  VideoCreatorDashboard
} from "@/types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const STORAGE_KEY = "video-creator-state";

export function isTauriRuntime() {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

export async function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(command, args);
  }
  return localCommand<T>(command, args);
}

function readSettings(): AppSettings {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return defaultSettings;
  try {
    const parsed = JSON.parse(raw) as Partial<AppSettings>;
    return {
      ...defaultSettings,
      ...parsed,
      aiProfile: {
        ...defaultSettings.aiProfile,
        ...(parsed.aiProfile ?? {})
      },
      feishuProfile: {
        ...defaultSettings.feishuProfile,
        ...(parsed.feishuProfile ?? {})
      }
    };
  } catch {
    return defaultSettings;
  }
}

function writeSettings(settings: AppSettings) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}

async function localCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const settings = readSettings();
  switch (command) {
    case "get_app_state":
      return { settings, version: APP_VERSION } satisfies AppStatePayload as T;
    case "get_settings":
      return settings as T;
    case "set_settings": {
      const next = {
        ...settings,
        ...(args?.settings as AppSettings),
        aiProfile: {
          ...settings.aiProfile,
          ...((args?.settings as Partial<AppSettings> | undefined)?.aiProfile ?? {})
        },
        feishuProfile: {
          ...settings.feishuProfile,
          ...((args?.settings as Partial<AppSettings> | undefined)?.feishuProfile ?? {})
        }
      };
      writeSettings(next);
      return next as T;
    }
    case "check_update_mock":
      return {
        currentVersion: APP_VERSION,
        latestVersion: APP_VERSION,
        available: false,
        notes: "当前已经是视频工坊最新版本。"
      } satisfies UpdateInfo as T;
    case "get_video_creator_dashboard":
      return buildLocalDashboard(settings) as T;
    case "save_skill_configs":
      return (args?.skills ?? []) as T;
    case "run_video_workflow": {
      const request = args?.request as { command?: string } | undefined;
      return {
        ok: true,
        message: `本地预览模式已模拟执行 ${request?.command ?? "unknown"}。`,
        exitCode: 0,
        stdout: "preview ok",
        stderr: ""
      } satisfies RunWorkflowResult as T;
    }
    case "open_video_creator_path":
      return undefined as T;
    case "test_ai_profile":
      return {
        ok: Boolean(settings.aiProfile.apiKey),
        message: settings.aiProfile.apiKey ? "AI 配置可用，本地预览模式已通过。" : "请先填写 AI API Key。",
        content: settings.aiProfile.apiKey ? "ok" : undefined
      } satisfies AiTestResult as T;
    case "generate_ai_text": {
      const request = args?.request as { prompt?: string } | undefined;
      return {
        content: `本地预览结果：\n\n${request?.prompt || "请填写 Prompt 后再生成。"}`,
        model: settings.aiProfile.model
      } satisfies AiGenerateResult as T;
    }
    case "test_feishu_profile":
    case "send_feishu_message":
      return {
        ok: Boolean(settings.feishuProfile.webhookUrl),
        message: settings.feishuProfile.webhookUrl ? "飞书配置可用，本地预览模式已通过。" : "请先填写飞书 Webhook 地址。"
      } satisfies FeishuSendResult as T;
    case "get_operation_logs":
      return {
        entries: []
      } satisfies GetOperationLogsResult as T;
    default:
      throw new Error(`Unsupported local command: ${command}`);
  }
}

function buildLocalDashboard(settings: AppSettings): VideoCreatorDashboard {
  const skills: SkillConfigEntry[] = [
    { key: "one-click", title: "一键执行", command: "one-click", enabled: true, sortOrder: 10, description: "写入 todo、下载资源、生成 script 并执行后续流程" },
    { key: "bbc-prefetch", title: "BBC 前置下载", command: "bbc-prefetch", enabled: true, sortOrder: 20, description: "下载图片、音频和 PDF" },
    { key: "script-text", title: "生成 Script", command: "script-text", enabled: true, sortOrder: 30, description: "从 PDF 生成原始脚本文本" },
    { key: "question-title", title: "疑问句标题", command: "question-title", enabled: true, sortOrder: 35, description: "生成中文疑问句标题" },
    { key: "six-minutes-codex", title: "Codex 工作流", command: "six-minutes-codex", enabled: true, sortOrder: 40, description: "执行 BBC 六分钟英语创作流程" },
    { key: "prepare-sixminutes", title: "发布素材整理", command: "prepare-sixminutes", enabled: true, sortOrder: 60, description: "整理发布所需素材" },
    { key: "daily-sync", title: "Daily 同步", command: "daily-sync", enabled: true, sortOrder: 70, description: "按年份同步 Daily 文件" }
  ];
  const steps = Array.from({ length: 22 }, (_, index) => ({
    seq: index + 1,
    code: index === 0 ? "TODO" : index === 1 ? "DOWNLOAD" : `STEP${String(index).padStart(2, "0")}`,
    name: ["写入 todo", "下载 BBC 资源", "生成 Script", "生成疑问句标题"][index] ?? `流程步骤 ${index + 1}`,
    status: "PENDING",
    startedAt: "-",
    finishedAt: "-",
    durationMs: 0,
    description: "等待手动执行"
  }));
  return {
    currentTask: "-",
    latestEpisode: settings.defaultEpisode,
    latestStatus: "PENDING",
    latestDurationMs: 0,
    latestStepCount: steps.length,
    totalSteps: steps.length,
    successfulSteps: 0,
    failedSteps: 0,
    runningSteps: 0,
    summary: "本地预览模式，等待手动执行任务。",
    vpnStatus: "预览模式",
    runtimeLogPath: "D:\\04_GitHub\\video-easy-creator\\logs\\app\\runtime.log",
    recentHistory: [],
    steps,
    skills,
    eventLogs: [],
    runtimeLogs: [],
    quark: {
      tokenValid: "待校验",
      cookieFile: "D:\\04_GitHub\\video-easy-creator\\auth\\cookie\\quark\\cookies.txt",
      cookieUpdatedAt: "-",
      rootItemCount: 0,
      latestResult: "启动后不会自动续跑任务，请手动校验。",
      logs: ["等待启动检查..."]
    }
  };
}
