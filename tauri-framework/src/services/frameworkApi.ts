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
  UpdateInfo
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
  getOperationLogs(request: GetOperationLogsRequest) {
    return callCommand<GetOperationLogsResult>("get_operation_logs", { request });
  }
};
