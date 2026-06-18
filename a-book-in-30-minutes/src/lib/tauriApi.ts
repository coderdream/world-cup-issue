import { APP_VERSION } from "@/config/app";
import { defaultSettings } from "@/store/defaults";
import type { AiGenerateResult, AiTestResult, AppSettings, AppStatePayload, BookMaterials, UpdateInfo } from "@/types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const STORAGE_KEY = "a-book-in-30-minutes-state";

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
        ...(args?.settings as Partial<AppSettings>),
        aiProfile: {
          ...settings.aiProfile,
          ...((args?.settings as Partial<AppSettings> | undefined)?.aiProfile ?? {})
        },
      };
      writeSettings(next);
      return next as T;
    }
    case "check_update_mock":
      return {
        currentVersion: APP_VERSION,
        latestVersion: APP_VERSION,
        available: false,
        notes: "当前已经是最新版本。"
      } satisfies UpdateInfo as T;
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
    case "generate_book_materials": {
      const request = args?.request as { epubPath?: string; channelName?: string; targetMinChars?: number; targetMaxChars?: number } | undefined;
      const channelName = request?.channelName || "半小时听完一本书";
      const narration = [
        "睡前听完一本书。",
        "今天这本书会在桌面版里调用 AI 生成完整旁白稿。",
        "浏览器预览模式先展示素材结构，安装版会读取 EPUB 并生成标题、简介、标签、旁白和字幕。"
      ].join("\n");
      return {
        videoTitle: `${channelName}｜示例书名`,
        description: "这里会生成适合 YouTube 的中文简介、时间感描述和版权提示。",
        tags: ["听书", "睡前听书", "半小时听完一本书", "A Book in 30 Minutes"],
        narration,
        subtitles: narration.split(/[。！？\n]+/).filter(Boolean),
        prompt: "安装版会在这里返回实际使用的生成提示词。",
        model: settings.aiProfile.model,
        overview: {
          title: request?.epubPath || "浏览器预览",
          creator: "",
          publisher: "",
          language: "zh-CN",
          totalChars: 0,
          chapters: []
        }
      } satisfies BookMaterials as T;
    }
    default:
      throw new Error(`Unsupported local command: ${command}`);
  }
}
