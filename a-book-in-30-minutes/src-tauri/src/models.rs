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
pub struct BookMaterialsRequest {
    pub epub_path: String,
    pub target_min_chars: usize,
    pub target_max_chars: usize,
    pub channel_name: String,
    pub language: String,
    pub extra_direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpubChapterSummary {
    pub title: String,
    pub chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpubOverview {
    pub title: String,
    pub creator: String,
    pub publisher: String,
    pub language: String,
    pub total_chars: usize,
    pub chapters: Vec<EpubChapterSummary>,
}

#[derive(Debug, Clone)]
pub struct EpubChapter {
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct EpubBook {
    pub overview: EpubOverview,
    pub chapters: Vec<EpubChapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookMaterials {
    pub video_title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub narration: String,
    pub subtitles: Vec<String>,
    pub prompt: String,
    pub model: String,
    pub overview: EpubOverview,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiBookMaterialsPayload {
    pub video_title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub narration: String,
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
