import { callCommand } from "@/lib/tauriApi";
import type {
  AiGenerateRequest,
  AiGenerateResult,
  AiTestResult,
  AppSettings,
  AppStatePayload,
  FeishuSendRequest,
  FeishuSendResult,
  GetOperationLogsRequest,
  GetOperationLogsResult,
  RunWorkflowRequest,
  RunWorkflowResult,
  SkillConfigEntry,
  UpdateInfo,
  VideoCreatorDashboard
} from "@/types";

export const frameworkApi = {
  getAppState() {
    return callCommand<AppStatePayload>("get_app_state");
  },
  getSettings() {
    return callCommand<AppSettings>("get_settings");
  },
  setSettings(settings: AppSettings) {
    return callCommand<AppSettings>("set_settings", { settings });
  },
  checkUpdateMock() {
    return callCommand<UpdateInfo>("check_update_mock");
  },
  getVideoCreatorDashboard() {
    return callCommand<VideoCreatorDashboard>("get_video_creator_dashboard");
  },
  runVideoWorkflow(request: RunWorkflowRequest) {
    return callCommand<RunWorkflowResult>("run_video_workflow", { request });
  },
  saveSkillConfigs(skills: SkillConfigEntry[]) {
    return callCommand<SkillConfigEntry[]>("save_skill_configs", { skills });
  },
  openVideoCreatorPath(target: string) {
    return callCommand<void>("open_video_creator_path", { target });
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
  getOperationLogs(request: GetOperationLogsRequest) {
    return callCommand<GetOperationLogsResult>("get_operation_logs", { request });
  }
};
