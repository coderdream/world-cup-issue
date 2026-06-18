import { callCommand } from "@/lib/tauriApi";
import type {
  AppSettings,
  AppStatePayload,
  AiEvaluationRequest,
  AiGenerationResult,
  AiModelConfig,
  ConnectivityTestResult,
  Match,
  Prediction,
  RefreshMatchesResult,
  Team,
  UpdateInfo
} from "@/types";

export const cupwatchApi = {
  getAppState() {
    return callCommand<AppStatePayload>("get_app_state");
  },
  refreshMatches() {
    return callCommand<RefreshMatchesResult>("refresh_matches");
  },
  getMatches() {
    return callCommand<Match[]>("get_matches");
  },
  getTeams() {
    return callCommand<Team[]>("get_teams");
  },
  toggleFavoriteTeam(teamId: string) {
    return callCommand<string[]>("toggle_favorite_team", { teamId });
  },
  savePrediction(prediction: Prediction) {
    return callCommand<Prediction[]>("save_prediction", { prediction });
  },
  getPredictions() {
    return callCommand<Prediction[]>("get_predictions");
  },
  getSettings() {
    return callCommand<AppSettings>("get_settings");
  },
  setSettings(settings: AppSettings) {
    return callCommand<AppSettings>("set_settings", { settings });
  },
  testFootballDataToken(token: string) {
    return callCommand<ConnectivityTestResult>("test_football_data_token", { token });
  },
  testAiModelConfig(config: AiModelConfig) {
    return callCommand<ConnectivityTestResult>("test_ai_model_config", { config });
  },
  generateAiEvaluation(request: AiEvaluationRequest) {
    return callCommand<AiGenerationResult>("generate_ai_evaluation", { config: request.config, context: request.context });
  },
  toggleSpoilerMode() {
    return callCommand<AppSettings>("toggle_spoiler_mode");
  },
  openFloatingScorebar() {
    return callCommand<AppSettings>("open_floating_scorebar");
  },
  closeFloatingScorebar() {
    return callCommand<AppSettings>("close_floating_scorebar");
  },
  checkUpdateMock() {
    return callCommand<UpdateInfo>("check_update_mock");
  }
};
