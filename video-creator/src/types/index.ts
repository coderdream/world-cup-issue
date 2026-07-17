export type RouteKey = "home" | "steps" | "logs" | "history" | "quark" | "skills" | "settings" | "about";

export interface AppSettings {
  theme: "dark" | "light";
  launchOnBoot: boolean;
  notificationsEnabled: boolean;
  apiBaseUrl: string;
  apiKey: string;
  javaProjectDir: string;
  javaRuntimeDir: string;
  outputDir: string;
  jianyingDraftDir: string;
  defaultEpisode: string;
  quarkYears: string;
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

export type TaskStatus = "SUCCESS" | "FAILED" | "RUNNING" | "PENDING" | string;

export interface VideoCreatorDashboard {
  currentTask: string;
  latestEpisode: string;
  latestStatus: TaskStatus;
  latestDurationMs: number;
  latestStepCount: number;
  totalSteps: number;
  successfulSteps: number;
  failedSteps: number;
  runningSteps: number;
  summary: string;
  vpnStatus: string;
  runtimeLogPath: string;
  recentHistory: OperationHistoryEntry[];
  steps: OperationStepEntry[];
  skills: SkillConfigEntry[];
  eventLogs: OperationEventEntry[];
  runtimeLogs: string[];
  quark: QuarkStatus;
}

export interface OperationHistoryEntry {
  id: number;
  ability: string;
  episodeCode: string;
  status: TaskStatus;
  currentStage: string;
  summary: string;
  startedAt: string;
  finishedAt: string;
  durationMs: number;
}

export interface OperationStepEntry {
  seq: number;
  code: string;
  name: string;
  status: TaskStatus;
  startedAt: string;
  finishedAt: string;
  durationMs: number;
  description: string;
}

export interface OperationEventEntry {
  createdAt: string;
  level: string;
  stage: string;
  message: string;
}

export interface SkillConfigEntry {
  key: string;
  title: string;
  command: string;
  enabled: boolean;
  sortOrder: number;
  description: string;
}

export interface QuarkStatus {
  tokenValid: string;
  cookieFile: string;
  cookieUpdatedAt: string;
  rootItemCount: number;
  latestResult: string;
  logs: string[];
}

export interface RunWorkflowRequest {
  command: string;
  episode?: string;
  outputDir?: string;
  preparePublishMaterials?: boolean;
  previewType?: string;
  years?: string;
}

export interface RunWorkflowResult {
  ok: boolean;
  message: string;
  exitCode?: number;
  stdout: string;
  stderr: string;
}
