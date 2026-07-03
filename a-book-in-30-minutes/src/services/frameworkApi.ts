import { callCommand } from "@/lib/tauriApi";
import type {
  AiGenerateRequest,
  AiGenerateResult,
  AiTestResult,
  AppSettings,
  AppStatePayload,
  BookMaterials,
  BookMaterialsRequest,
  ExportBookMaterialsRequest,
  ExportBookMaterialsResult,
  FeishuSendRequest,
  FeishuSendResult,
  GenerateMaterialTaskAudioRequest,
  GenerateBookVideoRequest,
  GenerateBookVideoResult,
  GeneratePublishMaterialsRequest,
  GeneratePublishMaterialsResult,
  GetMaterialTaskStepsRequest,
  GetMaterialTaskStepsResult,
  GetMaterialTasksRequest,
  GetSpeechVoicesResult,
  GetOperationLogsRequest,
  GetOperationLogsResult,
  GenerateAudioRequest,
  GenerateAudioResult,
  MaterialFile,
  MaterialTaskPathRequest,
  ResetMaterialTasksRequest,
  ScanMaterialFilesRequest,
  ScanMaterialFilesResult,
  SpeechPreviewRequest,
  SpeechRegionKeyRequest,
  SpeechRegionKeyResult,
  SpeechTestResult,
  ToolTestResult,
  UpdateInfo,
  UpdateMaterialTaskStatusRequest
} from "@/types";

export const frameworkApi = {
  getAppState() {
    return callCommand<AppStatePayload>("get_app_state");
  },
  getSettings() {
    return callCommand<AppSettings>("get_settings");
  },
  setSettings(settings: Partial<AppSettings>) {
    return callCommand<AppSettings>("set_settings", { settings });
  },
  checkUpdateMock() {
    return callCommand<UpdateInfo>("check_update_mock");
  },
  testAiProfile() {
    return callCommand<AiTestResult>("test_ai_profile");
  },
  generateAiText(request: AiGenerateRequest) {
    return callCommand<AiGenerateResult>("generate_ai_text", { request });
  },
  testFeishuProfile() {
    return callCommand<FeishuSendResult>("test_feishu_profile");
  },
  sendFeishuMessage(request: FeishuSendRequest) {
    return callCommand<FeishuSendResult>("send_feishu_message", { request });
  },
  testSpeechProfile() {
    return callCommand<SpeechTestResult>("test_speech_profile");
  },
  previewSpeech(request: SpeechPreviewRequest) {
    return callCommand<SpeechTestResult>("preview_speech", { request });
  },
  saveSpeechRegionKey(request: SpeechRegionKeyRequest) {
    return callCommand<SpeechRegionKeyResult>("save_speech_region_key", { request });
  },
  getSpeechRegionKey(region: string) {
    return callCommand<SpeechRegionKeyResult>("get_speech_region_key", { region });
  },
  getSpeechVoices(locale?: string) {
    return callCommand<GetSpeechVoicesResult>("get_speech_voices", { locale });
  },
  testFfmpegPath() {
    return callCommand<ToolTestResult>("test_ffmpeg_path");
  },
  generateAudio(request: GenerateAudioRequest) {
    return callCommand<GenerateAudioResult>("generate_audio", { request });
  },
  generateMaterialTaskAudio(request: GenerateMaterialTaskAudioRequest) {
    return callCommand<GenerateAudioResult>("generate_material_task_audio", { request });
  },
  generateBookVideoPipeline(request: GenerateBookVideoRequest) {
    return callCommand<GenerateBookVideoResult>("generate_book_video_pipeline", { request });
  },
  generatePublishMaterials(request: GeneratePublishMaterialsRequest) {
    return callCommand<GeneratePublishMaterialsResult>("generate_publish_materials", { request });
  },
  generateBookMaterials(request: BookMaterialsRequest) {
    return callCommand<BookMaterials>("generate_book_materials", { request });
  },
  scanMaterialFiles(request: ScanMaterialFilesRequest) {
    return callCommand<ScanMaterialFilesResult>("scan_material_files", { request });
  },
  getMaterialTasks(request: GetMaterialTasksRequest = {}) {
    return callCommand<ScanMaterialFilesResult>("get_material_tasks", { request });
  },
  getMaterialTask(request: MaterialTaskPathRequest) {
    return callCommand<MaterialFile | null>("get_material_task", { request });
  },
  updateMaterialTaskStatus(request: UpdateMaterialTaskStatusRequest) {
    return callCommand<MaterialFile>("update_material_task_status", { request });
  },
  removeMaterialTask(request: MaterialTaskPathRequest) {
    return callCommand<boolean>("remove_material_task", { request });
  },
  resetMaterialTasks(request: ResetMaterialTasksRequest = {}) {
    return callCommand<boolean>("reset_material_tasks", { request });
  },
  openMaterialOutputDir(request: MaterialTaskPathRequest) {
    return callCommand<boolean>("open_material_output_dir", { request });
  },
  exportBookMaterials(request: ExportBookMaterialsRequest) {
    return callCommand<ExportBookMaterialsResult>("export_book_materials", { request });
  },
  getOperationLogs(request: GetOperationLogsRequest) {
    return callCommand<GetOperationLogsResult>("get_operation_logs", { request });
  },
  getMaterialTaskSteps(request: GetMaterialTaskStepsRequest) {
    return callCommand<GetMaterialTaskStepsResult>("get_material_task_steps", { request });
  }
};
