use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    pub code: String,
    pub name_zh: String,
    pub name_en: String,
    pub group: String,
    pub flag: String,
    pub elo: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    pub home: Option<i64>,
    pub away: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    pub id: String,
    pub group: String,
    pub stage: String,
    pub date: String,
    pub time: String,
    pub utc_offset: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub score: Score,
    pub status: String,
    pub venue: String,
    pub city: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prediction {
    pub match_id: String,
    pub pick: String,
    pub locked_at: String,
    pub resolved: bool,
    pub correct: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub spoiler_mode: bool,
    pub scorebar_enabled: bool,
    pub launch_on_boot: bool,
    pub notifications_enabled: bool,
    pub reminder_minutes: i64,
    pub football_data_token: String,
    #[serde(default = "default_ai_provider")]
    pub ai_provider: String,
    pub ai_api_key: String,
    #[serde(default = "default_ai_base_url")]
    pub ai_base_url: String,
    #[serde(default = "default_ai_model")]
    pub ai_model: String,
    #[serde(default = "default_ai_profile_name")]
    pub ai_profile_name: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            spoiler_mode: false,
            scorebar_enabled: false,
            launch_on_boot: false,
            notifications_enabled: true,
            reminder_minutes: 15,
            football_data_token: String::new(),
            ai_provider: default_ai_provider(),
            ai_api_key: String::new(),
            ai_base_url: default_ai_base_url(),
            ai_model: default_ai_model(),
            ai_profile_name: default_ai_profile_name(),
        }
    }
}

fn default_ai_provider() -> String {
    "OpenAI Compatible".to_string()
}

fn default_ai_base_url() -> String {
    "http://81.68.73.15:3000/openai/v1".to_string()
}

fn default_ai_model() -> String {
    "gpt-5.5".to_string()
}

fn default_ai_profile_name() -> String {
    "杯况 CupWatch".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseState {
    pub status: String,
    pub remaining_days: i64,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BracketNode {
    pub id: String,
    pub round: String,
    pub slot_a: String,
    pub slot_b: String,
    pub venue: Option<String>,
    pub winner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub teams: Vec<Team>,
    pub matches: Vec<Match>,
    pub settings: AppSettings,
    pub predictions: Vec<Prediction>,
    pub license: LicenseState,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshMatchesResult {
    pub matches: Vec<Match>,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityTestResult {
    pub ok: bool,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiEvaluationContext {
    pub match_id: String,
    pub home_team: String,
    pub away_team: String,
    pub kickoff: String,
    pub venue: String,
    pub status: String,
    pub score: String,
    pub odds_home: i64,
    pub odds_draw: i64,
    pub odds_away: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerationResult {
    pub ok: bool,
    pub content: String,
    pub message: Option<String>,
}
