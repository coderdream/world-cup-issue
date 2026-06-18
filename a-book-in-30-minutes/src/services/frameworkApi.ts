import { callCommand } from "@/lib/tauriApi";
import type {
  AiGenerateRequest,
  AiGenerateResult,
  AiTestResult,
  AppSettings,
  AppStatePayload,
  BookMaterials,
  BookMaterialsRequest,
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
  generateBookMaterials(request: BookMaterialsRequest) {
    return callCommand<BookMaterials>("generate_book_materials", { request });
  }
};
