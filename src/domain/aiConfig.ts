import { defaultSettings } from "@/data/worldCupData";
import type { AiModelConfig, AiProfileShare, AppSettings } from "@/types";

export interface AiProviderPreset {
  value: string;
  label: string;
  baseUrl: string;
  model: string;
}

export const aiProviderPresets: AiProviderPreset[] = [
  {
    value: "OpenAI Compatible",
    label: "自定义（OpenAI 兼容）",
    baseUrl: "http://81.68.73.15:3000/openai/v1",
    model: "gpt-5.5"
  },
  {
    value: "OpenAI",
    label: "OpenAI",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-5.5"
  },
  {
    value: "OpenRouter",
    label: "OpenRouter（聚合多模型）",
    baseUrl: "https://openrouter.ai/api/v1",
    model: "openai/gpt-5.5"
  },
  {
    value: "Groq",
    label: "Groq（超快推理）",
    baseUrl: "https://api.groq.com/openai/v1",
    model: "llama-3.3-70b-versatile"
  },
  {
    value: "Google Gemini",
    label: "Google Gemini",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai",
    model: "gemini-2.0-flash"
  },
  {
    value: "xAI Grok",
    label: "xAI Grok",
    baseUrl: "https://api.x.ai/v1",
    model: "grok-3"
  },
  {
    value: "Anthropic Claude",
    label: "Anthropic Claude",
    baseUrl: "https://api.anthropic.com/v1",
    model: "claude-sonnet-4"
  },
  {
    value: "Tencent Hunyuan",
    label: "腾讯混元 Hunyuan",
    baseUrl: "https://api.hunyuan.cloud.tencent.com/v1",
    model: "hunyuan-lite"
  }
];

export function getAiProviderPreset(value?: string | null) {
  return aiProviderPresets.find((item) => item.value === value) ?? aiProviderPresets[0];
}

export function getAiModelConfig(settings?: Partial<AppSettings> | Partial<AiModelConfig> | null): AiModelConfig {
  if (settings && "baseUrl" in settings) {
    const config = settings as Partial<AiModelConfig>;
    return {
      provider: config.provider ?? defaultSettings.aiProvider,
      apiKey: config.apiKey ?? defaultSettings.aiApiKey,
      baseUrl: config.baseUrl ?? defaultSettings.aiBaseUrl,
      model: config.model ?? defaultSettings.aiModel,
      name: config.name ?? defaultSettings.aiProfileName
    };
  }
  const appSettings = settings as Partial<AppSettings> | null | undefined;
  const preset = getAiProviderPreset(appSettings?.aiProvider);
  return {
    provider: appSettings?.aiProvider ?? defaultSettings.aiProvider,
    apiKey: appSettings?.aiApiKey ?? defaultSettings.aiApiKey,
    baseUrl: appSettings?.aiBaseUrl ?? preset.baseUrl ?? defaultSettings.aiBaseUrl,
    model: appSettings?.aiModel ?? preset.model ?? defaultSettings.aiModel,
    name: appSettings?.aiProfileName ?? defaultSettings.aiProfileName
  };
}

export function buildAiProfileShare(settings?: Partial<AppSettings> | Partial<AiModelConfig> | null): AiProfileShare {
  const config = getAiModelConfig(settings);
  return {
    data: {
      apiKey: config.apiKey,
      baseURL: config.baseUrl,
      model: config.model,
      name: config.name,
      provider: "openai_compatible"
    },
    kind: "ai.profile",
    v: 1
  };
}

export function stringifyAiProfileShare(settings?: Partial<AppSettings> | Partial<AiModelConfig> | null) {
  return JSON.stringify(buildAiProfileShare(settings), null, 2);
}

export function parseAiProfileShare(text: string): Partial<AppSettings> | null {
  try {
    const parsed = JSON.parse(text) as Partial<AiProfileShare> & {
      data?: Partial<AiProfileShare["data"]>;
    };
    if (parsed.kind !== "ai.profile") return null;
    const data = parsed.data;
    if (!data) return null;
    return {
      aiApiKey: data.apiKey ?? "",
      aiBaseUrl: data.baseURL ?? defaultSettings.aiBaseUrl,
      aiModel: data.model ?? defaultSettings.aiModel,
      aiProfileName: data.name ?? defaultSettings.aiProfileName,
      aiProvider: defaultSettings.aiProvider
    };
  } catch {
    return null;
  }
}

export function resolveAiChatEndpoint(baseUrl: string) {
  const trimmed = baseUrl.trim().replace(/\/+$/, "");
  if (trimmed.endsWith("/openai/v1")) {
    return `${trimmed.replace(/\/openai\/v1$/, "")}/api/v1/chat/completions`;
  }
  if (trimmed.endsWith("/chat/completions")) {
    return trimmed;
  }
  return `${trimmed}/chat/completions`;
}
