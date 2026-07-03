export type RouteKey = "home" | "logs" | "settings" | "about";

export interface AppSettings {
  theme: "dark" | "light";
  launchOnBoot: boolean;
  notificationsEnabled: boolean;
  apiBaseUrl: string;
  apiKey: string;
  aiProfile: AiProfile;
  feishuProfile: FeishuProfile;
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  available: boolean;
  notes: string;
}

export interface AppStatePayload {
  settings: AppSettings;
  version: string;
}

export interface AiProfile {
  provider: "openai_compatible";
  name: string;
  baseURL: string;
  model: string;
  apiKey: string;
}

export interface AiProfileShare {
  data: AiProfile;
  kind: "ai.profile";
  v: 1;
}

export interface AiTestResult {
  ok: boolean;
  message: string;
  content?: string;
}

export interface AiGenerateRequest {
  prompt: string;
  systemPrompt?: string;
}

export interface AiGenerateResult {
  content: string;
  model: string;
}

export interface FeishuProfile {
  webhookUrl: string;
  title: string;
  testMessage: string;
}

export interface FeishuSendRequest {
  text: string;
}

export interface FeishuSendResult {
  ok: boolean;
  message: string;
}

export interface GetOperationLogsRequest {
  limit: number;
  traceId?: string;
}

export interface OperationLogEntry {
  id: number;
  createdAt: string;
  level: "DEBUG" | "INFO" | "WARN" | "ERROR" | string;
  module: string;
  action: string;
  message: string;
  detail?: string;
  traceId?: string;
}

export interface GetOperationLogsResult {
  entries: OperationLogEntry[];
}
