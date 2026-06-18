import { APP_VERSION } from "@/config/app";
import { defaultSettings } from "@/store/defaults";
import type { AiGenerateResult, AiTestResult, AppSettings, AppStatePayload, FeishuSendResult, UpdateInfo } from "@/types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const STORAGE_KEY = "tauri-framework-state";

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
        ...(args?.settings as Partial<AppSettings>),
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
        notes: "当前已经是最新框架版本。"
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
    case "test_feishu_profile":
    case "send_feishu_message":
      return {
        ok: Boolean(settings.feishuProfile.webhookUrl),
        message: settings.feishuProfile.webhookUrl ? "飞书配置可用，本地预览模式已通过。" : "请先填写飞书 Webhook 地址。"
      } satisfies FeishuSendResult as T;
    default:
      throw new Error(`Unsupported local command: ${command}`);
  }
}
