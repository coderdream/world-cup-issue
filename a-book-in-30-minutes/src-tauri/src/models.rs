use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub launch_on_boot: bool,
    pub notifications_enabled: bool,
    pub api_base_url: String,
    pub api_key: String,
    pub active_ai_provider: String,
    pub ai_profile: AiProfile,
    pub gemini_profile: GeminiProfile,
    pub feishu_profile: FeishuProfile,
    pub material_profile: MaterialProfile,
    pub speech_profile: SpeechProfile,
    pub tool_profile: ToolProfile,
    pub ui_profile: UiProfile,
    pub pipeline_profile: PipelineProfile,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            launch_on_boot: false,
            notifications_enabled: true,
            api_base_url: "https://api.example.com".to_string(),
            api_key: String::new(),
            active_ai_provider: "gpt".to_string(),
            ai_profile: AiProfile::default(),
            gemini_profile: GeminiProfile::default(),
            feishu_profile: FeishuProfile::default(),
            material_profile: MaterialProfile::default(),
            speech_profile: SpeechProfile::default(),
            tool_profile: ToolProfile::default(),
            ui_profile: UiProfile::default(),
            pipeline_profile: PipelineProfile::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct UiProfile {
    pub menu_font_family: String,
    pub menu_font_size: u16,
    pub content_font_family: String,
    pub content_font_size: u16,
}

impl Default for UiProfile {
    fn default() -> Self {
        let family = "\"Microsoft YaHei UI\", \"Microsoft YaHei\", \"PingFang SC\", \"Noto Sans SC\", \"Segoe UI\", Arial, sans-serif".to_string();
        Self {
            menu_font_family: family.clone(),
            menu_font_size: 13,
            content_font_family: family,
            content_font_size: 12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PipelineProfile {
    pub image_backend: String,
    pub skip_existing_materials: bool,
    pub skip_existing_text: bool,
    pub skip_existing_images: bool,
    pub skip_existing_audio: bool,
    pub skip_existing_subtitles: bool,
    pub skip_existing_video: bool,
    pub skip_existing_publish: bool,
}

impl Default for PipelineProfile {
    fn default() -> Self {
        Self {
            image_backend: "xiaohei-production".to_string(),
            skip_existing_materials: true,
            skip_existing_text: true,
            skip_existing_images: true,
            skip_existing_audio: true,
            skip_existing_subtitles: true,
            skip_existing_video: true,
            skip_existing_publish: true,
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
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AiProfile {
    pub provider: String,
    pub name: String,
    #[serde(rename = "baseURL")]
    pub base_url: String,
    pub model: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub proxy_enabled: bool,
    pub proxy_url: String,
}

impl Default for AiProfile {
    fn default() -> Self {
        Self {
            provider: "openai_compatible".to_string(),
            name: "A Book in 30 Minutes".to_string(),
            base_url: "http://81.68.73.15:3000/openai/v1".to_string(),
            model: "gpt-5.5".to_string(),
            api_key: String::new(),
            proxy_enabled: false,
            proxy_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct GeminiProfile {
    pub provider: String,
    pub name: String,
    #[serde(rename = "baseURL")]
    pub base_url: String,
    pub model: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub proxy_enabled: bool,
    pub proxy_url: String,
}

impl Default for GeminiProfile {
    fn default() -> Self {
        Self {
            provider: "gemini".to_string(),
            name: "Gemini".to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            model: "gemini-flash-latest".to_string(),
            api_key: String::new(),
            proxy_enabled: true,
            proxy_url: "http://127.0.0.1:1080".to_string(),
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
#[serde(default)]
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
            title: "A Book in 30 Minutes".to_string(),
            test_message: "听书素材生成工具飞书连通性测试成功。".to_string(),
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
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct MaterialProfile {
    pub channel_name: String,
    pub category_name: String,
    pub categories: Vec<String>,
    pub language: String,
    pub target_min_chars: usize,
    pub target_max_chars: usize,
    pub extra_direction: String,
}

impl Default for MaterialProfile {
    fn default() -> Self {
        Self {
            channel_name: "半小时听完一本书".to_string(),
            category_name: "半小时听完一本书".to_string(),
            categories: vec![
                "半小时听完一本书".to_string(),
                "睡前听完一本书".to_string(),
                "A Book in 30 Minutes".to_string(),
            ],
            language: "zh-CN".to_string(),
            target_min_chars: 7500,
            target_max_chars: 7800,
            extra_direction: "睡前听书风格，温柔、克制、有陪伴感。旁白目标为 30-35 分钟语音，最佳落在 7500~7800 个中文字；标题和简介服务于 YouTube 中文频道。".to_string(),
        }
    }
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
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialFile {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: u64,
    pub category: String,
    pub status: String,
    pub progress: i64,
    pub narration_chars: Option<i64>,
    pub material_output_dir: Option<String>,
    pub message: String,
    pub audio_status: String,
    pub audio_progress: i64,
    pub audio_output_dir: Option<String>,
    pub audio_file: Option<String>,
    pub audio_duration_ms: Option<i64>,
    pub audio_chunks: Option<i64>,
    pub audio_message: String,
    pub image_status: String,
    pub image_progress: i64,
    pub image_output_dir: Option<String>,
    pub image_message: String,
    pub subtitle_status: String,
    pub subtitle_progress: i64,
    pub subtitle_file: Option<String>,
    pub subtitle_message: String,
    pub video_status: String,
    pub video_progress: i64,
    pub video_file: Option<String>,
    pub video_duration_ms: Option<i64>,
    pub video_file_size: Option<i64>,
    pub video_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanMaterialFilesRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanMaterialFilesResult {
    pub directory: String,
    pub files: Vec<MaterialFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMaterialTasksRequest {
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMaterialTaskStatusRequest {
    pub path: String,
    pub category: Option<String>,
    pub status: String,
    pub progress: i64,
    pub narration_chars: Option<i64>,
    pub material_output_dir: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMaterialTaskStageStatusRequest {
    pub path: String,
    pub stage: String,
    pub status: String,
    pub progress: i64,
    pub output_path: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialOutputDirRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialTaskPathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateMaterialTaskAudioRequest {
    pub path: String,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateBookVideoRequest {
    pub epub_path: String,
    pub trace_id: Option<String>,
    pub pipeline_stage: Option<String>,
    pub allow_placeholder_visuals: Option<bool>,
    pub controlled_programmatic_visuals: Option<bool>,
    pub ignore_existing_visual_assets: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateBookVideoResult {
    pub material_dir: String,
    pub pipeline_manifest: String,
    pub cover: Option<String>,
    pub visual_story_plan: Option<String>,
    pub visual_timeline: Option<String>,
    pub no_subtitle_video: Option<String>,
    pub hard_subtitle_video: Option<String>,
    pub hard_subtitle_manifest: Option<String>,
    pub elapsed_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePublishMaterialsRequest {
    pub epub_path: String,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePublishMaterialsResult {
    pub output_dir: String,
    pub markdown_file: String,
    pub title: String,
    pub chapters: usize,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialTaskProgressEvent {
    pub trace_id: String,
    pub path: String,
    pub status: String,
    pub progress: i64,
    pub step: usize,
    pub total_steps: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetMaterialTasksRequest {
    pub path: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBookMaterialsRequest {
    pub output_dir: String,
    pub materials: BookMaterials,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBookMaterialsResult {
    pub output_dir: String,
    pub files: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMaterialTaskStepsRequest {
    pub trace_id: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialTaskStep {
    pub trace_id: String,
    pub path: String,
    pub step_code: String,
    pub step_name: String,
    pub status: String,
    pub progress: i64,
    pub detail: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMaterialTaskStepsResult {
    pub steps: Vec<MaterialTaskStep>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiBookMaterialsPayload {
    pub video_title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub narration: String,
    #[serde(default)]
    pub subtitles: Vec<String>,
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

#[derive(Debug, Serialize)]
pub struct GeminiGenerateRequest {
    pub contents: Vec<GeminiContent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct GeminiGenerateResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiCandidate {
    pub content: Option<GeminiContent>,
}

#[derive(Debug, Deserialize)]
pub struct FeishuWebhookResponse {
    pub code: Option<i32>,
    pub msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct SpeechProfile {
    pub provider: String,
    pub speech_key: String,
    #[serde(default, skip_serializing)]
    pub region_keys: BTreeMap<String, String>,
    #[serde(default = "default_speech_locale")]
    pub locale: String,
    pub region: String,
    pub voice_name: String,
    pub output_format: String,
    pub rate: String,
    pub pitch: String,
    pub proxy_enabled: bool,
    pub proxy_url: String,
}

impl Default for SpeechProfile {
    fn default() -> Self {
        Self {
            provider: "azure_microsoft".to_string(),
            speech_key: String::new(),
            region_keys: BTreeMap::new(),
            locale: default_speech_locale(),
            region: "eastasia".to_string(),
            voice_name: "zh-CN-YunxiNeural".to_string(),
            output_format: "audio-24khz-160kbitrate-mono-mp3".to_string(),
            rate: "0%".to_string(),
            pitch: "+0Hz".to_string(),
            proxy_enabled: true,
            proxy_url: "http://127.0.0.1:1080".to_string(),
        }
    }
}

fn default_speech_locale() -> String {
    "zh-CN".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ToolProfile {
    pub ffmpeg_path: String,
    pub background_music_mode: String,
    pub background_music_path: String,
}

impl Default for ToolProfile {
    fn default() -> Self {
        Self {
            ffmpeg_path: r"D:\03_Dev\ffmpeg\bin\ffmpeg.exe".to_string(),
            background_music_mode: "single".to_string(),
            background_music_path: default_background_music_path(),
        }
    }
}

fn default_background_music_path() -> String {
    "D:\\04_GitHub\\world-cup-issue\\a-book-in-30-minutes\\music\\bf.mp3".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateAudioRequest {
    pub text: String,
    pub output_dir: String,
    pub file_name: String,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateAudioResult {
    pub output_dir: String,
    pub audio_file: String,
    pub ssml_file: String,
    pub manifest_file: String,
    pub part_files: Vec<String>,
    pub chars: usize,
    pub chunks: usize,
    pub duration_ms: Option<u64>,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioManifestPart {
    pub index: usize,
    pub sentence_start: usize,
    pub sentence_end: usize,
    pub chars: usize,
    pub estimated_duration_ms: u64,
    pub status: String,
    pub text_file: String,
    pub ssml_file: String,
    pub audio_file: String,
    pub error: Option<String>,
    pub elapsed_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioManifest {
    pub trace_id: String,
    pub source: String,
    pub status: String,
    pub chars: usize,
    pub chunks: usize,
    pub final_audio_file: String,
    pub duration_ms: Option<u64>,
    pub created_at: String,
    pub updated_at: String,
    pub parts: Vec<AudioManifestPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolTestResult {
    pub ok: bool,
    pub message: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechTestResult {
    pub ok: bool,
    pub message: String,
    pub audio_file: Option<String>,
    pub audio_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechPreviewRequest {
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechRegionKeyRequest {
    pub region: String,
    pub speech_key: String,
    pub voice_name: Option<String>,
    pub output_format: Option<String>,
    pub rate: Option<String>,
    pub pitch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechRegionKeyResult {
    pub region: String,
    pub speech_key: String,
    pub voice_name: String,
    pub output_format: String,
    pub rate: String,
    pub pitch: String,
    pub has_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechVoice {
    pub locale: String,
    pub language: String,
    pub voice_type: String,
    pub voice_name: String,
    pub gender: String,
    pub styles: String,
    pub roles: String,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSpeechVoicesResult {
    pub source_url: String,
    pub voices: Vec<SpeechVoice>,
}
