export type RouteKey = "home" | "settings" | "about";

export interface AppSettings {
  theme: "dark" | "light";
  launchOnBoot: boolean;
  notificationsEnabled: boolean;
  apiBaseUrl: string;
  apiKey: string;
  aiProfile: AiProfile;
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

export interface BookMaterialsRequest {
  epubPath: string;
  targetMinChars: number;
  targetMaxChars: number;
  channelName: string;
  language: "zh-CN";
  extraDirection: string;
}

export interface EpubChapterSummary {
  title: string;
  chars: number;
}

export interface EpubOverview {
  title: string;
  creator: string;
  publisher: string;
  language: string;
  totalChars: number;
  chapters: EpubChapterSummary[];
}

export interface BookMaterials {
  videoTitle: string;
  description: string;
  tags: string[];
  narration: string;
  subtitles: string[];
  prompt: string;
  model: string;
  overview: EpubOverview;
}

export interface ExportBookMaterialsRequest {
  outputDir: string;
  materials: BookMaterials;
}

export interface ExportBookMaterialsResult {
  outputDir: string;
  files: string[];
}
