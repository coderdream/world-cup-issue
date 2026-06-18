import { defaultLicense, defaultSettings, matches, teams } from "@/data/worldCupData";
import type {
  AiModelConfig,
  AiEvaluationRequest,
  AiGenerationResult,
  AppSettings,
  AppStatePayload,
  ConnectivityTestResult,
  Prediction,
  RefreshMatchesResult,
  UpdateInfo
} from "@/types";

const STORAGE_KEY = "worldcupissue-state";

interface PersistedState {
  settings: AppSettings;
  predictions: Prediction[];
  favorites: string[];
  lastUpdated: string | null;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

async function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

export function isTauriRuntime() {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

export async function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauriRuntime()) return invokeTauri<T>(command, args);
  return localCommand<T>(command, args);
}

function readPersisted(): PersistedState {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) {
    return { settings: defaultSettings, predictions: [], favorites: [], lastUpdated: null };
  }
  try {
    const parsed = JSON.parse(raw) as Partial<PersistedState>;
    return {
      settings: { ...defaultSettings, ...parsed.settings },
      predictions: parsed.predictions ?? [],
      favorites: parsed.favorites ?? [],
      lastUpdated: parsed.lastUpdated ?? null
    };
  } catch {
    return { settings: defaultSettings, predictions: [], favorites: [], lastUpdated: null };
  }
}

function writePersisted(next: PersistedState) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
}

async function localCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const persisted = readPersisted();
  switch (command) {
    case "get_app_state":
      return {
        teams,
        matches,
        settings: persisted.settings,
        predictions: persisted.predictions,
        license: defaultLicense,
        lastUpdated: persisted.lastUpdated
      } satisfies AppStatePayload as T;
    case "refresh_matches":
      {
        const lastUpdated = formatBeijingMinute();
        writePersisted({ ...persisted, lastUpdated });
        return {
          matches,
          lastUpdated
        } satisfies RefreshMatchesResult as T;
      }
    case "get_matches":
      return matches as T;
    case "get_standings":
      return [] as T;
    case "get_bracket":
      return [] as T;
    case "get_teams":
      return teams as T;
    case "get_settings":
      return persisted.settings as T;
    case "set_settings": {
      const settings = { ...persisted.settings, ...(args?.settings as Partial<AppSettings>) };
      writePersisted({ ...persisted, settings });
      return settings as T;
    }
    case "test_football_data_token":
      return (await testFootballDataToken(String(args?.token ?? ""))) as T;
    case "test_ai_model_config":
      return (await testAiModelConfig(args?.config as AiModelConfig)) as T;
    case "generate_ai_evaluation":
      return (await generateAiEvaluation(args as unknown as AiEvaluationRequest)) as T;
    case "toggle_spoiler_mode": {
      const settings = { ...persisted.settings, spoilerMode: !persisted.settings.spoilerMode };
      writePersisted({ ...persisted, settings });
      return settings as T;
    }
    case "toggle_favorite_team": {
      const teamId = String(args?.teamId);
      const favorites = persisted.favorites.includes(teamId)
        ? persisted.favorites.filter((id) => id !== teamId)
        : [...persisted.favorites, teamId];
      writePersisted({ ...persisted, favorites });
      return favorites as T;
    }
    case "save_prediction": {
      const prediction = args?.prediction as Prediction;
      const predictions = [
        ...persisted.predictions.filter((item) => item.matchId !== prediction.matchId),
        prediction
      ];
      writePersisted({ ...persisted, predictions });
      return predictions as T;
    }
    case "get_predictions":
      return persisted.predictions as T;
    case "open_floating_scorebar":
      return { ...persisted.settings, scorebarEnabled: true } as T;
    case "close_floating_scorebar":
      return { ...persisted.settings, scorebarEnabled: false } as T;
    case "check_update_mock":
      return {
        currentVersion: "0.1.11",
        latestVersion: "0.1.11",
        available: false,
        notes: "当前已是 WorldCupIssue（世界杯组手）本地复刻版最新版本。"
      } satisfies UpdateInfo as T;
    default:
      throw new Error(`Unsupported local command: ${command}`);
  }
}

async function generateAiEvaluation(request: AiEvaluationRequest | undefined): Promise<AiGenerationResult> {
  const config = request?.config;
  const context = request?.context;
  const baseUrl = config?.baseUrl?.trim() ?? "";
  const apiKey = config?.apiKey?.trim() ?? "";
  const model = config?.model?.trim() ?? "";
  if (!baseUrl || !apiKey || !model) {
    return { ok: false, content: "", message: "请先完成 AI 模型配置" };
  }
  if (!context) {
    return { ok: false, content: "", message: "缺少比赛上下文" };
  }
  const endpoint = aiChatEndpoint(baseUrl);
  const payload = {
    model,
    messages: [
      {
        role: "system",
        content: "你是世界杯赛事资讯解读助手。只做公开赛事信息分析，不提供投注、赔率、下注或博彩建议。"
      },
      {
        role: "user",
        content: buildAiEvaluationPrompt(context)
      }
    ],
    stream: false
  };
  try {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
        Accept: "*/*"
      },
      body: JSON.stringify(payload)
    });
    const text = await readText(response);
    if (!response.ok) {
      return { ok: false, content: "", message: `AI 评估生成失败：HTTP ${response.status} ${text}` };
    }
    const parsed = JSON.parse(text) as { choices?: Array<{ message?: { content?: string } }> };
    const content = parsed.choices?.[0]?.message?.content?.trim() ?? "";
    return content
      ? { ok: true, content }
      : { ok: false, content: "", message: "AI 已响应，但没有返回分析内容" };
  } catch (error) {
    return { ok: false, content: "", message: error instanceof Error ? error.message : String(error) };
  }
}

function buildAiEvaluationPrompt(context: AiEvaluationRequest["context"]) {
  return [
    `请基于公开赛事信息，生成一段中文赛前/赛中资讯解读。`,
    `比赛：${context.homeTeam} vs ${context.awayTeam}`,
    `时间：${context.kickoff}`,
    `地点：${context.venue}`,
    `状态：${context.status}`,
    `比分：${context.score}`,
    `Elo 概率估算：主胜 ${context.oddsHome}%，平局 ${context.oddsDraw}%，客胜 ${context.oddsAway}%。`,
    `要求：3-5 句，语气克制，不要给投注建议，不要出现赔率、下注、盘口相关表达。`
  ].join("\n");
}

function formatBeijingMinute(now = new Date()) {
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
    timeZone: "Asia/Shanghai"
  }).format(now);
}

async function testFootballDataToken(token: string): Promise<ConnectivityTestResult> {
  const trimmed = token.trim();
  if (!trimmed) {
    return { ok: false, message: "请先输入 API Token" };
  }
  try {
    const response = await fetch("https://api.football-data.org/v4/competitions/WC/matches", {
      headers: { "X-Auth-Token": trimmed }
    });
    if (!response.ok) {
      return { ok: false, message: `Token 测试失败：HTTP ${response.status}`, details: await readText(response) };
    }
    return { ok: true, message: "Token 可用，football-data.org 已返回 WC 赛事数据" };
  } catch (error) {
    return { ok: false, message: "Token 测试失败：无法连接 football-data.org", details: error instanceof Error ? error.message : String(error) };
  }
}

async function testAiModelConfig(config: AiModelConfig | undefined): Promise<ConnectivityTestResult> {
  const baseUrl = config?.baseUrl?.trim() ?? "";
  const apiKey = config?.apiKey?.trim() ?? "";
  const model = config?.model?.trim() ?? "";
  if (!baseUrl) return { ok: false, message: "请先填写接口地址 baseURL" };
  if (!apiKey) return { ok: false, message: "请先填写 API Key" };
  if (!model) return { ok: false, message: "请先填写模型名" };
  const endpoint = aiChatEndpoint(baseUrl);
  const payload = {
    model,
    messages: [{ role: "user", content: "你好，请只回复 ok" }],
    stream: false
  };
  try {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
        Accept: "*/*"
      },
      body: JSON.stringify(payload)
    });
    const text = await readText(response);
    if (!response.ok) {
      return { ok: false, message: `模型连接失败：HTTP ${response.status}`, details: text };
    }
    const parsed = JSON.parse(text) as { choices?: Array<{ message?: { content?: string } }> };
    const content = parsed.choices?.[0]?.message?.content?.trim() ?? "";
    const ok = content.toLowerCase() === "ok";
    return {
      ok,
      message: ok ? "模型连接成功，返回 ok" : "模型已响应，但未按测试提示返回 ok",
      details: content || text
    };
  } catch (error) {
    return {
      ok: false,
      message: "模型连接失败：无法请求接口地址",
      details: error instanceof Error ? error.message : String(error)
    };
  }
}

function aiChatEndpoint(baseUrl: string) {
  const trimmed = baseUrl.trim().replace(/\/+$/, "");
  if (trimmed.endsWith("/openai/v1")) {
    return `${trimmed.replace(/\/openai\/v1$/, "")}/api/v1/chat/completions`;
  }
  if (trimmed.endsWith("/chat/completions")) {
    return trimmed;
  }
  return `${trimmed}/chat/completions`;
}

async function readText(response: Response) {
  try {
    return await response.text();
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

