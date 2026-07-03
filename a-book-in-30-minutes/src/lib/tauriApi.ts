import { APP_VERSION } from "@/config/app";
import { defaultSettings } from "@/store/defaults";
import type {
  AiGenerateResult,
  AiTestResult,
  AppSettings,
  AppStatePayload,
  BookMaterials,
  ExportBookMaterialsResult,
  FeishuSendResult,
  GetMaterialTaskStepsResult,
  GetSpeechVoicesResult,
  GetOperationLogsResult,
  ScanMaterialFilesResult,
  SpeechRegionKeyResult,
  UpdateInfo
} from "@/types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const STORAGE_KEY = "a-book-in-30-minutes-state";
const MICROSOFT_TTS_LANGUAGE_SUPPORT_URL = "https://learn.microsoft.com/zh-cn/azure/ai-services/speech-service/language-support?tabs=tts";

const localeLanguage: Record<string, string> = {
  "zh-CN": "中文（普通话，简体）",
  "en-US": "英语（美国）",
  "en-GB": "英语（英国）"
};

const localSpeechVoices = [
  ["zh-CN", "Neural", "zh-CN-XiaoxiaoNeural", "Female", "assistant, chat, customerservice, newscast", "Girl, YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-YunxiNeural", "Male", "assistant, chat, narration-relaxed", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-YunjianNeural", "Male", "narration-relaxed, sports-commentary", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaoyiNeural", "Female", "affectionate, cheerful, gentle", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-YunyangNeural", "Male", "customerservice, narration-professional", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaochenNeural", "Female", "general", "YoungAdult"],
  ["zh-CN", "MultilingualNeural", "zh-CN-XiaochenMultilingualNeural", "Female", "multilingual", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaohanNeural", "Female", "calm, cheerful, serious", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaomengNeural", "Female", "general", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaomoNeural", "Female", "affectionate, calm, cheerful", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaoqiuNeural", "Female", "general", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaorouNeural", "Female", "general", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaoruiNeural", "Female", "calm, sad", "Senior"],
  ["zh-CN", "Neural", "zh-CN-XiaoshuangNeural", "Female", "chat", "Child"],
  ["zh-CN", "Neural", "zh-CN-XiaoxiaoDialectsNeural", "Female", "dialect", "YoungAdult"],
  ["zh-CN", "MultilingualNeural", "zh-CN-XiaoxiaoMultilingualNeural", "Female", "multilingual", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaoyanNeural", "Female", "general", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaoyouNeural", "Female", "general", "Child"],
  ["zh-CN", "MultilingualNeural", "zh-CN-XiaoyuMultilingualNeural", "Female", "multilingual", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-XiaozhenNeural", "Female", "cheerful, serious", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-YunfengNeural", "Male", "cheerful, serious", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-YunhaoNeural", "Male", "advertisement-upbeat", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-YunjieNeural", "Male", "documentary-narration, serious", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-YunxiaNeural", "Male", "cheerful, calm", "Child"],
  ["zh-CN", "Neural", "zh-CN-YunyeNeural", "Male", "general", "YoungAdult"],
  ["zh-CN", "MultilingualNeural", "zh-CN-YunyiMultilingualNeural", "Male", "multilingual", "YoungAdult"],
  ["zh-CN", "Neural", "zh-CN-YunzeNeural", "Male", "calm, documentary-narration", "OlderAdult"],
  ["zh-CN", "MultilingualNeural", "zh-CN-YunfanMultilingualNeural", "Male", "multilingual", "YoungAdult"],
  ["zh-CN", "MultilingualNeural", "zh-CN-YunxiaoMultilingualNeural", "Male", "multilingual", "YoungAdult"],
  ["en-US", "Neural", "en-US-JennyNeural", "Female", "assistant, chat, customerservice, newscast", "YoungAdult"],
  ["en-US", "Neural", "en-US-GuyNeural", "Male", "newscast", "YoungAdult"],
  ["en-US", "Neural", "en-US-AriaNeural", "Female", "chat, customerservice, newscast", "YoungAdult"],
  ["en-US", "Neural", "en-US-DavisNeural", "Male", "chat", "YoungAdult"],
  ["en-US", "Neural", "en-US-JaneNeural", "Female", "general", "YoungAdult"],
  ["en-US", "Neural", "en-US-JasonNeural", "Male", "general", "YoungAdult"],
  ["en-US", "Neural", "en-US-NancyNeural", "Female", "general", "YoungAdult"],
  ["en-US", "Neural", "en-US-TonyNeural", "Male", "general", "YoungAdult"],
  ["en-GB", "Neural", "en-GB-SoniaNeural", "Female", "general", "YoungAdult"],
  ["en-GB", "Neural", "en-GB-RyanNeural", "Male", "general", "YoungAdult"],
  ["en-GB", "Neural", "en-GB-LibbyNeural", "Female", "general", "YoungAdult"]
].map(([locale, voiceType, voiceName, gender, styles, roles]) => ({
  locale,
  language: localeLanguage[locale] ?? locale,
  voiceType,
  voiceName,
  gender,
  styles,
  roles,
  sourceUrl: MICROSOFT_TTS_LANGUAGE_SUPPORT_URL
}));

export function isTauriRuntime() {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

export async function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) {
    const { invoke } = await import("@tauri-apps/api/core");
    try {
      return await invoke<T>(command, args);
    } catch (caught) {
      throw new Error(formatCommandError(caught));
    }
  }
  return localCommand<T>(command, args);
}

function formatCommandError(caught: unknown) {
  if (caught instanceof Error) return caught.message;
  if (typeof caught === "string") return caught;
  if (caught && typeof caught === "object") {
    const record = caught as Record<string, unknown>;
    if (typeof record.message === "string") return record.message;
    if (typeof record.error === "string") return record.error;
    try {
      return JSON.stringify(caught);
    } catch {
      return "操作失败，但错误对象无法序列化。";
    }
  }
  return String(caught);
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
      },
      materialProfile: {
        ...defaultSettings.materialProfile,
        ...(parsed.materialProfile ?? {}),
        categories: Array.from(
          new Set([
            ...defaultSettings.materialProfile.categories,
            (parsed.materialProfile ?? {}).categoryName,
            ...((parsed.materialProfile ?? {}).categories ?? [])
          ].filter((value): value is string => typeof value === "string" && value.trim().length > 0))
        )
      },
      speechProfile: {
        ...defaultSettings.speechProfile,
        ...(parsed.speechProfile ?? {}),
        regionKeys: {
          ...defaultSettings.speechProfile.regionKeys,
          ...((parsed.speechProfile ?? {}).regionKeys ?? {})
        }
      },
      toolProfile: {
        ...defaultSettings.toolProfile,
        ...(parsed.toolProfile ?? {})
      },
      pipelineProfile: {
        ...defaultSettings.pipelineProfile,
        ...(parsed.pipelineProfile ?? {})
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
        },
        materialProfile: {
          ...settings.materialProfile,
          ...((args?.settings as Partial<AppSettings> | undefined)?.materialProfile ?? {}),
          categories: Array.from(
            new Set([
              ...settings.materialProfile.categories,
              ((args?.settings as Partial<AppSettings> | undefined)?.materialProfile ?? {}).categoryName,
              ...(((args?.settings as Partial<AppSettings> | undefined)?.materialProfile ?? {}).categories ?? [])
            ].filter((value): value is string => typeof value === "string" && value.trim().length > 0))
          )
        },
        speechProfile: {
          ...settings.speechProfile,
          ...((args?.settings as Partial<AppSettings> | undefined)?.speechProfile ?? {}),
          regionKeys: {
            ...settings.speechProfile.regionKeys,
            ...(((args?.settings as Partial<AppSettings> | undefined)?.speechProfile ?? {}).regionKeys ?? {})
          }
        },
        toolProfile: {
          ...settings.toolProfile,
          ...((args?.settings as Partial<AppSettings> | undefined)?.toolProfile ?? {})
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
    case "test_feishu_profile":
    case "send_feishu_message":
      return {
        ok: Boolean(settings.feishuProfile.webhookUrl),
        message: settings.feishuProfile.webhookUrl ? "飞书配置可用，本地预览模式已通过。" : "请先填写飞书 Webhook 地址。"
      } satisfies FeishuSendResult as T;
    case "test_ffmpeg_path":
      return {
        ok: Boolean(settings.toolProfile.ffmpegPath),
        message: settings.toolProfile.ffmpegPath ? "ffmpeg.exe 路径可用，本地预览模式已通过。" : "请先填写 ffmpeg.exe 路径。",
        version: settings.toolProfile.ffmpegPath ? "ffmpeg preview" : undefined
      } as T;
    case "test_speech_profile":
      return {
        ok: Boolean(settings.speechProfile.speechKey),
        message: settings.speechProfile.speechKey ? "微软语音配置可用，本地预览模式已通过。" : "请先填写微软语音 Speech Key。",
        audioFile: settings.speechProfile.speechKey ? "浏览器预览模式不会写入测试音频" : undefined
      } as T;
    case "preview_speech":
      return {
        ok: Boolean(settings.speechProfile.speechKey),
        message: settings.speechProfile.speechKey ? "试听音频已生成并准备播放。" : "请先填写微软语音 Speech Key。",
        audioFile: settings.speechProfile.speechKey ? "浏览器预览模式不会写入测试音频" : undefined
      } as T;
    case "save_speech_region_key": {
      const request = args?.request as { region?: string; speechKey?: string; voiceName?: string; outputFormat?: string; rate?: string; pitch?: string } | undefined;
      const region = request?.region || settings.speechProfile.region;
      const speechKey = request?.speechKey || "";
      const voiceName = request?.voiceName || settings.speechProfile.voiceName;
      const outputFormat = request?.outputFormat || settings.speechProfile.outputFormat;
      const rate = request?.rate || settings.speechProfile.rate;
      const pitch = request?.pitch || settings.speechProfile.pitch;
      const next = {
        ...settings,
        speechProfile: {
          ...settings.speechProfile,
          region,
          speechKey,
          voiceName,
          outputFormat,
          rate,
          pitch,
          regionKeys: {
            ...settings.speechProfile.regionKeys,
            [region]: speechKey
          }
        }
      };
      writeSettings(next);
      return { region, speechKey, voiceName, outputFormat, rate, pitch, hasKey: Boolean(speechKey) } satisfies SpeechRegionKeyResult as T;
    }
    case "get_speech_region_key": {
      const region = typeof args?.region === "string" ? args.region : settings.speechProfile.region;
      const speechKey = settings.speechProfile.regionKeys?.[region] ?? "";
      return {
        region,
        speechKey,
        voiceName: settings.speechProfile.voiceName,
        outputFormat: settings.speechProfile.outputFormat,
        rate: settings.speechProfile.rate,
        pitch: settings.speechProfile.pitch,
        hasKey: Boolean(speechKey)
      } satisfies SpeechRegionKeyResult as T;
    }
    case "get_speech_voices": {
      const locale = typeof args?.locale === "string" ? args.locale : "";
      return {
        sourceUrl: MICROSOFT_TTS_LANGUAGE_SUPPORT_URL,
        voices: locale ? localSpeechVoices.filter((voice) => voice.locale === locale) : localSpeechVoices
      } satisfies GetSpeechVoicesResult as T;
    }
    case "generate_audio": {
      const request = args?.request as { text?: string; outputDir?: string; fileName?: string } | undefined;
      return {
        outputDir: request?.outputDir || "浏览器预览模式不会写入本地文件",
        audioFile: `${request?.fileName || "narration"}.mp3`,
        ssmlFile: "narration.ssml",
        manifestFile: "audio_manifest.json",
        partFiles: [],
        chars: request?.text?.length || 0,
        chunks: 1,
        durationMs: 30000,
        elapsedMs: 1200
      } as T;
    }
    case "generate_material_task_audio": {
      return {
        outputDir: "浏览器预览模式不会写入本地文件",
        audioFile: "narration.mp3",
        ssmlFile: "narration.ssml",
        manifestFile: "audio_manifest.json",
        partFiles: [],
        chars: 0,
        chunks: 1,
        durationMs: 30000,
        elapsedMs: 1200
      } as T;
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
    case "scan_material_files": {
      const request = args?.request as { path?: string } | undefined;
      return {
        directory: request?.path || "",
        files: []
      } satisfies ScanMaterialFilesResult as T;
    }
    case "get_material_tasks":
      return {
        directory: "",
        files: []
      } satisfies ScanMaterialFilesResult as T;
    case "update_material_task_status": {
      const request = args?.request as { path?: string; status?: "pending" | "generating" | "success" | "failed"; progress?: number; narrationChars?: number; message?: string } | undefined;
      return {
        path: request?.path || "",
        name: request?.path?.split(/[\\/]/).pop() || "",
        extension: request?.path?.split(".").pop()?.toLowerCase() || "",
        size: 0,
        category: settings.materialProfile.categoryName,
        status: request?.status || "pending",
        progress: request?.progress || 0,
        narrationChars: request?.narrationChars,
        materialOutputDir: undefined,
        message: request?.message || "",
        audioStatus: "pending",
        audioProgress: 0,
        audioOutputDir: undefined,
        audioFile: undefined,
        audioDurationMs: undefined,
        audioChunks: undefined,
        audioMessage: "",
        videoStatus: "pending",
        videoProgress: 0,
        videoFile: undefined,
        videoDurationMs: undefined,
        videoFileSize: undefined,
        videoMessage: ""
      } as T;
    }
    case "remove_material_task":
    case "reset_material_tasks":
    case "open_material_output_dir":
      return true as T;
    case "export_book_materials":
      return {
        outputDir: "浏览器预览模式不会写入本地文件",
        files: []
      } satisfies ExportBookMaterialsResult as T;
    case "get_operation_logs":
      {
        const request = args?.request as { traceId?: string } | undefined;
        const traceId = request?.traceId || "materials-preview-current";
        return {
          entries: [
            {
              id: 1,
              createdAt: new Date().toISOString().slice(0, 19).replace("T", " "),
              level: "INFO",
              module: "materials",
              action: "generate.start",
              message: "开始生成本次 YouTube 听书素材",
              detail: "浏览器预览模式不会读取真实 EPUB。",
              traceId
            },
            {
              id: 2,
              createdAt: new Date().toISOString().slice(0, 19).replace("T", " "),
              level: "DEBUG",
              module: "materials",
              action: "settings.snapshot",
              message: "读取本次生成使用的 AI 配置",
              detail: `model=${settings.aiProfile.model} base_url=${settings.aiProfile.baseURL} api_key_present=${Boolean(settings.aiProfile.apiKey)}`,
              traceId
            },
            {
              id: 3,
              createdAt: new Date().toISOString().slice(0, 19).replace("T", " "),
              level: "INFO",
              module: "materials",
              action: "generate.done",
              message: "本次 YouTube 听书素材生成成功",
              detail: "安装版会显示真实解析、AI 请求、字幕切分和导出日志。",
              traceId
            }
          ]
        } satisfies GetOperationLogsResult as T;
      }
    case "get_material_task_steps":
      return { steps: [] } satisfies GetMaterialTaskStepsResult as T;
    default:
      throw new Error(`Unsupported local command: ${command}`);
  }
}
