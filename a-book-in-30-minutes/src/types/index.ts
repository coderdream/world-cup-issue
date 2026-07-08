export type RouteKey = "home" | "audio" | "logs" | "settings" | "about";

export interface AppSettings {
  theme: "dark" | "light";
  launchOnBoot: boolean;
  notificationsEnabled: boolean;
  apiBaseUrl: string;
  apiKey: string;
  activeAiProvider: AiProvider;
  aiProfile: AiProfile;
  geminiProfile: GeminiProfile;
  feishuProfile: FeishuProfile;
  materialProfile: MaterialProfile;
  speechProfile: SpeechProfile;
  toolProfile: ToolProfile;
  uiProfile: UiProfile;
  pipelineProfile: PipelineProfile;
}

export interface UiProfile {
  menuFontFamily: string;
  menuFontSize: number;
  contentFontFamily: string;
  contentFontSize: number;
}

export interface PipelineProfile {
  imageBackend: "xiaohei-production" | "xiaohei-sequence" | "xiaohei-ai-y9000p" | "qwen-image-2512" | "whiteboard-skill";
  skipExistingMaterials: boolean;
  skipExistingText: boolean;
  skipExistingImages: boolean;
  skipExistingAudio: boolean;
  skipExistingSubtitles: boolean;
  skipExistingVideo: boolean;
  skipExistingPublish: boolean;
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
  proxyEnabled: boolean;
  proxyUrl: string;
}

export type AiProvider = "gpt" | "gemini";

export interface GeminiProfile {
  provider: "gemini";
  name: string;
  baseURL: string;
  model: string;
  apiKey: string;
  proxyEnabled: boolean;
  proxyUrl: string;
}

export interface AiProfileShare {
  data: AiProfile | GeminiProfile;
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

export interface MaterialProfile {
  channelName: string;
  categoryName: string;
  categories: string[];
  language: "zh-CN";
  targetMinChars: number;
  targetMaxChars: number;
  extraDirection: string;
}

export interface SpeechProfile {
  provider: "azure_microsoft";
  speechKey: string;
  regionKeys: Record<string, string>;
  locale: string;
  region: string;
  voiceName: string;
  outputFormat: string;
  rate: string;
  pitch: string;
}

export interface ToolProfile {
  ffmpegPath: string;
  backgroundMusicMode: "single" | "playlist";
  backgroundMusicPath: string;
}

export interface GenerateAudioRequest {
  text: string;
  outputDir: string;
  fileName: string;
  traceId?: string;
}

export interface GenerateAudioResult {
  outputDir: string;
  audioFile: string;
  ssmlFile: string;
  manifestFile: string;
  partFiles: string[];
  chars: number;
  chunks: number;
  durationMs?: number | null;
  elapsedMs: number;
}

export interface GenerateBookVideoRequest {
  epubPath: string;
  traceId?: string;
  pipelineStage?: "image" | "subtitle" | "video";
  allowPlaceholderVisuals?: boolean;
  controlledProgrammaticVisuals?: boolean;
  ignoreExistingVisualAssets?: boolean;
}

export interface GenerateBookVideoResult {
  materialDir: string;
  pipelineManifest: string;
  cover?: string | null;
  visualStoryPlan?: string | null;
  visualTimeline?: string | null;
  noSubtitleVideo?: string | null;
  hardSubtitleVideo?: string | null;
  hardSubtitleManifest?: string | null;
  elapsedSeconds: number;
}

export interface GeneratePublishMaterialsRequest {
  epubPath: string;
  traceId?: string;
}

export interface GeneratePublishMaterialsResult {
  outputDir: string;
  markdownFile: string;
  title: string;
  chapters: number;
  tags: string[];
}

export interface ToolTestResult {
  ok: boolean;
  message: string;
  version?: string;
}

export interface SpeechTestResult {
  ok: boolean;
  message: string;
  audioFile?: string;
  audioDataUrl?: string;
}

export interface SpeechPreviewRequest {
  text?: string;
}

export interface SpeechRegionKeyRequest {
  region: string;
  speechKey: string;
  voiceName?: string;
  outputFormat?: string;
  rate?: string;
  pitch?: string;
}

export interface SpeechRegionKeyResult {
  region: string;
  speechKey: string;
  voiceName: string;
  outputFormat: string;
  rate: string;
  pitch: string;
  hasKey: boolean;
}

export interface SpeechVoice {
  locale: string;
  language: string;
  voiceType: string;
  voiceName: string;
  gender: string;
  styles: string;
  roles: string;
  sourceUrl: string;
}

export interface GetSpeechVoicesResult {
  sourceUrl: string;
  voices: SpeechVoice[];
}

export interface BookMaterialsRequest {
  epubPath: string;
  targetMinChars: number;
  targetMaxChars: number;
  channelName: string;
  language: "zh-CN";
  extraDirection: string;
  traceId?: string;
}

export type MaterialsOutputTab = "title" | "description" | "tags" | "narration" | "subtitles" | "prompt";

export interface MaterialFile {
  path: string;
  name: string;
  extension: string;
  size: number;
  category: string;
  status: "pending" | "generating" | "success" | "failed";
  progress: number;
  narrationChars?: number | null;
  materialOutputDir?: string | null;
  message: string;
  audioStatus: "pending" | "generating" | "success" | "failed";
  audioProgress: number;
  audioOutputDir?: string | null;
  audioFile?: string | null;
  audioDurationMs?: number | null;
  audioChunks?: number | null;
  audioMessage: string;
  imageStatus: "pending" | "generating" | "success" | "failed";
  imageProgress: number;
  imageOutputDir?: string | null;
  imageMessage: string;
  subtitleStatus: "pending" | "generating" | "success" | "failed";
  subtitleProgress: number;
  subtitleFile?: string | null;
  subtitleMessage: string;
  videoStatus: "pending" | "generating" | "success" | "failed";
  videoProgress: number;
  videoFile?: string | null;
  videoDurationMs?: number | null;
  videoFileSize?: number | null;
  videoMessage: string;
}

export interface ScanMaterialFilesRequest {
  path: string;
}

export interface ScanMaterialFilesResult {
  directory: string;
  files: MaterialFile[];
}

export interface GetMaterialTasksRequest {
  category?: string;
}

export interface UpdateMaterialTaskStatusRequest {
  path: string;
  category?: string;
  status: "pending" | "generating" | "success" | "failed";
  progress: number;
  narrationChars?: number | null;
  materialOutputDir?: string | null;
  message?: string;
}

export interface UpdateMaterialTaskStageStatusRequest {
  path: string;
  stage: "audio" | "image" | "subtitle" | "video";
  status: "pending" | "generating" | "success" | "failed";
  progress: number;
  outputPath?: string | null;
  message?: string;
}

export interface MaterialTaskProgressEvent {
  traceId: string;
  path: string;
  status: "pending" | "generating" | "success" | "failed";
  progress: number;
  step: number;
  totalSteps: number;
  message: string;
}

export interface MaterialTaskPathRequest {
  path: string;
}

export interface GenerateMaterialTaskAudioRequest {
  path: string;
  traceId?: string;
}

export interface ResetMaterialTasksRequest {
  path?: string;
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
  traceId?: string;
}

export interface ExportBookMaterialsResult {
  outputDir: string;
  files: string[];
}

export interface GetOperationLogsRequest {
  limit: number;
  traceId?: string;
}

export interface OperationLogEntry {
  id: number;
  createdAt: string;
  level: string;
  module: string;
  action: string;
  message: string;
  detail?: string | null;
  traceId?: string | null;
}

export interface GetOperationLogsResult {
  entries: OperationLogEntry[];
}

export interface GetMaterialTaskStepsRequest {
  traceId?: string;
  path?: string;
}

export interface MaterialTaskStep {
  traceId: string;
  path: string;
  stepCode: string;
  stepName: string;
  status: "pending" | "generating" | "success" | "failed";
  progress: number;
  detail: string;
  startedAt?: string | null;
  finishedAt?: string | null;
  elapsedMs?: number | null;
  updatedAt: string;
}

export interface GetMaterialTaskStepsResult {
  steps: MaterialTaskStep[];
}
