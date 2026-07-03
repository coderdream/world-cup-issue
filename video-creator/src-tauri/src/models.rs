use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub launch_on_boot: bool,
    pub notifications_enabled: bool,
    pub api_base_url: String,
    pub api_key: String,
    pub java_project_dir: String,
    pub output_dir: String,
    pub jianying_draft_dir: String,
    pub default_episode: String,
    pub quark_years: String,
    pub ai_profile: AiProfile,
    pub feishu_profile: FeishuProfile,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            launch_on_boot: false,
            notifications_enabled: true,
            api_base_url: "https://api.example.com".to_string(),
            api_key: String::new(),
            java_project_dir: r"D:\04_GitHub\video-easy-creator".to_string(),
            output_dir: r"D:\14_LearnEnglish\6MinuteEnglish".to_string(),
            jianying_draft_dir: r"D:\03_Software\JianyingPro Drafts\六分钟英语_2606".to_string(),
            default_episode: "260625".to_string(),
            quark_years: "2014,2015,2016,2017,2018,2019,2020,2021,2022,2023,2024,2025,2026".to_string(),
            ai_profile: AiProfile::default(),
            feishu_profile: FeishuProfile::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub settings: AppSettings,
    pub version: String,
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
pub struct AiProfile {
    pub provider: String,
    pub name: String,
    #[serde(rename = "baseURL")]
    pub base_url: String,
    pub model: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
}

impl Default for AiProfile {
    fn default() -> Self {
        Self {
            provider: "openai_compatible".to_string(),
            name: "视频工坊".to_string(),
            base_url: "http://81.68.73.15:3000/openai/v1".to_string(),
            model: "gpt-5.5".to_string(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTestResult {
    pub ok: bool,
    pub message: String,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerateRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerateResult {
    pub content: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuProfile {
    pub webhook_url: String,
    pub title: String,
    pub test_message: String,
}

impl Default for FeishuProfile {
    fn default() -> Self {
        Self {
            webhook_url: String::new(),
            title: "视频工坊".to_string(),
            test_message: "飞书连通性测试成功。".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuSendRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuSendResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOperationLogsRequest {
    pub limit: usize,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogEntry {
    pub id: i64,
    pub created_at: String,
    pub level: String,
    pub module: String,
    pub action: String,
    pub message: String,
    pub detail: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOperationLogsResult {
    pub entries: Vec<OperationLogEntry>,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
}

#[derive(Debug, Deserialize)]
pub struct FeishuWebhookResponse {
    pub code: Option<i32>,
    pub msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoCreatorDashboard {
    pub current_task: String,
    pub latest_episode: String,
    pub latest_status: String,
    pub latest_duration_ms: i64,
    pub latest_step_count: usize,
    pub total_steps: usize,
    pub successful_steps: usize,
    pub failed_steps: usize,
    pub running_steps: usize,
    pub summary: String,
    pub vpn_status: String,
    pub runtime_log_path: String,
    pub recent_history: Vec<OperationHistoryEntry>,
    pub steps: Vec<OperationStepEntry>,
    pub skills: Vec<SkillConfigEntry>,
    pub event_logs: Vec<OperationEventEntry>,
    pub runtime_logs: Vec<String>,
    pub quark: QuarkStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationHistoryEntry {
    pub id: i64,
    pub ability: String,
    pub episode_code: String,
    pub status: String,
    pub current_stage: String,
    pub summary: String,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationStepEntry {
    pub seq: usize,
    pub code: String,
    pub name: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEventEntry {
    pub created_at: String,
    pub level: String,
    pub stage: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillConfigEntry {
    pub key: String,
    pub title: String,
    pub command: String,
    pub enabled: bool,
    pub sort_order: i64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarkStatus {
    pub token_valid: String,
    pub cookie_file: String,
    pub cookie_updated_at: String,
    pub root_item_count: i64,
    pub latest_result: String,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWorkflowRequest {
    pub command: String,
    pub episode: Option<String>,
    pub output_dir: Option<String>,
    pub prepare_publish_materials: Option<bool>,
    pub preview_type: Option<String>,
    pub years: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWorkflowResult {
    pub ok: bool,
    pub message: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}
