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
            name: "Tauri Framework".to_string(),
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
            title: "Tauri Framework".to_string(),
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
