use crate::epub::{count_han_chars, read_epub, truncate_chars};
use crate::models::{
    AiBookMaterialsPayload, AiGenerateRequest, AiGenerateResult, AiTestResult, AppSettings,
    AppStatePayload, AudioManifest, AudioManifestPart, BookMaterials, BookMaterialsRequest,
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, EpubBook, EpubChapter,
    EpubChapterSummary, EpubOverview, ExportBookMaterialsRequest, ExportBookMaterialsResult,
    FeishuSendRequest, FeishuSendResult, FeishuWebhookResponse, GenerateAudioRequest,
    GenerateAudioResult, GenerateBookVideoRequest, GenerateBookVideoResult,
    GenerateMaterialTaskAudioRequest, GetMaterialTasksRequest, GetOperationLogsRequest,
    GetOperationLogsResult, GetSpeechVoicesResult, MaterialFile, MaterialOutputDirRequest,
    MaterialTaskPathRequest, MaterialTaskProgressEvent, OperationLogEntry,
    ResetMaterialTasksRequest, ScanMaterialFilesRequest, ScanMaterialFilesResult,
    SpeechPreviewRequest, SpeechProfile, SpeechRegionKeyRequest, SpeechRegionKeyResult,
    SpeechTestResult, SpeechVoice, ToolTestResult, UpdateInfo, UpdateMaterialTaskStatusRequest,
};
use crate::operation_log::OperationLogger;
use base64::Engine;
use regex::Regex;
use rusqlite::params;
use rusqlite::Connection;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};

const SOURCE_READ_TIMEOUT_SECONDS: u64 = 30;
const AI_REQUEST_TIMEOUT_SECONDS: u64 = 600;
const AI_REQUEST_MAX_ATTEMPTS: usize = 3;
const FEISHU_REQUEST_TIMEOUT_SECONDS: u64 = 20;
const SPEECH_REQUEST_TIMEOUT_SECONDS: u64 = 120;
const SPEECH_CHUNK_MAX_CHARS: usize = 2200;
const SPEECH_CHUNK_MAX_SENTENCES: usize = 100;
const SPEECH_CHUNK_MAX_ESTIMATED_MS: u64 = 8 * 60 * 1000;
const MICROSOFT_TTS_LANGUAGE_SUPPORT_URL: &str =
    "https://learn.microsoft.com/zh-cn/azure/ai-services/speech-service/language-support?tabs=tts";
const DEFAULT_MATERIAL_CATEGORY: &str = "閸楀﹤鐨弮璺烘儔鐎瑰奔绔撮張顑垮姛";
const MATERIAL_PROGRESS_STEPS: usize = 4;

const SPEECH_VOICE_SEEDS: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaoxiaoNeural", "Female", "assistant, chat, customerservice, newscast, affectionate, angry, calm, cheerful, disgruntled, fearful, gentle, lyrical, sad, serious", "Girl, YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunxiNeural", "Male", "assistant, chat, narration-relaxed, angry, cheerful, depressed, disgruntled, embarrassed, fearful, sad, serious", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunjianNeural", "Male", "narration-relaxed, sports-commentary, sports-commentary-excited", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaoyiNeural", "Female", "affectionate, angry, cheerful, disgruntled, embarrassed, fearful, gentle, sad, serious", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunyangNeural", "Male", "customerservice, narration-professional, newscast-casual", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaochenNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "MultilingualNeural", "zh-CN-XiaochenMultilingualNeural", "Female", "multilingual", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaohanNeural", "Female", "calm, fearful, cheerful, disgruntled, serious, angry, sad, gentle, affectionate, embarrassed", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaomengNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaomoNeural", "Female", "affectionate, angry, calm, cheerful, depressed, disgruntled, embarrassed, envious, fearful, gentle, sad, serious", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaoqiuNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaorouNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaoruiNeural", "Female", "angry, calm, fearful, sad", "Senior"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaoshuangNeural", "Female", "chat", "Child"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaoxiaoDialectsNeural", "Female", "dialect", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "MultilingualNeural", "zh-CN-XiaoxiaoMultilingualNeural", "Female", "multilingual", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaoyanNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaoyouNeural", "Female", "general", "Child"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "MultilingualNeural", "zh-CN-XiaoyuMultilingualNeural", "Female", "multilingual", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-XiaozhenNeural", "Female", "angry, cheerful, disgruntled, fearful, sad, serious", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunfengNeural", "Male", "angry, cheerful, depressed, disgruntled, fearful, sad, serious", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunhaoNeural", "Male", "advertisement-upbeat", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunjieNeural", "Male", "angry, cheerful, depressed, disgruntled, documentary-narration, fearful, sad, serious", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunxiaNeural", "Male", "angry, calm, cheerful, fearful, sad", "Child"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunyeNeural", "Male", "general", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "MultilingualNeural", "zh-CN-YunyiMultilingualNeural", "Male", "multilingual", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "Neural", "zh-CN-YunzeNeural", "Male", "calm, cheerful, depressed, disgruntled, documentary-narration, fearful, sad, serious", "OlderAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "MultilingualNeural", "zh-CN-YunfanMultilingualNeural", "Male", "multilingual", "YoungAdult"),
    ("zh-CN", "娑擃厽鏋冮敍鍫熸珮闁俺鐦介敍宀€鐣濇担鎿勭礆", "MultilingualNeural", "zh-CN-YunxiaoMultilingualNeural", "Male", "multilingual", "YoungAdult"),
    ("en-US", "閼昏精顕㈤敍鍫㈢法閸ユ枻绱?", "Neural", "en-US-JennyNeural", "Female", "assistant, chat, customerservice, newscast", "YoungAdult"),
    ("en-US", "閼昏精顕㈤敍鍫㈢法閸ユ枻绱?", "Neural", "en-US-GuyNeural", "Male", "newscast", "YoungAdult"),
    ("en-US", "閼昏精顕㈤敍鍫㈢法閸ユ枻绱?", "Neural", "en-US-AriaNeural", "Female", "chat, customerservice, newscast", "YoungAdult"),
    ("en-US", "閼昏精顕㈤敍鍫㈢法閸ユ枻绱?", "Neural", "en-US-DavisNeural", "Male", "chat", "YoungAdult"),
    ("en-US", "閼昏精顕㈤敍鍫㈢法閸ユ枻绱?", "Neural", "en-US-JaneNeural", "Female", "general", "YoungAdult"),
    ("en-US", "閼昏精顕㈤敍鍫㈢法閸ユ枻绱?", "Neural", "en-US-JasonNeural", "Male", "general", "YoungAdult"),
    ("en-US", "閼昏精顕㈤敍鍫㈢法閸ユ枻绱?", "Neural", "en-US-NancyNeural", "Female", "general", "YoungAdult"),
    ("en-US", "閼昏精顕㈤敍鍫㈢法閸ユ枻绱?", "Neural", "en-US-TonyNeural", "Male", "general", "YoungAdult"),
    ("en-GB", "閼昏精顕㈤敍鍫ｅ閸ユ枻绱?", "Neural", "en-GB-SoniaNeural", "Female", "general", "YoungAdult"),
    ("en-GB", "閼昏精顕㈤敍鍫ｅ閸ユ枻绱?", "Neural", "en-GB-RyanNeural", "Male", "general", "YoungAdult"),
    ("en-GB", "閼昏精顕㈤敍鍫ｅ閸ユ枻绱?", "Neural", "en-GB-LibbyNeural", "Female", "general", "YoungAdult"),
];

pub struct AppData {
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
    db_path: PathBuf,
    logger: OperationLogger,
    app_started_at: String,
}

#[derive(Clone)]
struct AudioTaskProgress {
    db_path: PathBuf,
    path: String,
}

impl AudioTaskProgress {
    fn update(&self, progress: i64, message: &str) {
        if let Ok(connection) = Connection::open(&self.db_path) {
            let _ = update_material_task_audio_status(
                &connection,
                &self.path,
                "generating",
                progress,
                None,
                None,
                None,
                None,
                Some(message),
            );
        }
    }
}

impl AppData {
    pub fn load(app: &tauri::AppHandle) -> Self {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let app_local_data_dir = app
            .path()
            .app_local_data_dir()
            .unwrap_or_else(|_| app_data_dir.clone());

        let settings_path = app_data_dir.join("settings.json");
        let db_path = app_data_dir.join("app.db");
        let log_dir = app_local_data_dir.join("logs");
        let logger = OperationLogger::new(db_path.clone(), log_dir);

        let settings_file_exists = settings_path.exists();
        let settings_load_result = fs::read_to_string(&settings_path)
            .map_err(|error| error.to_string())
            .and_then(|content| {
                serde_json::from_str::<AppSettings>(content.trim_start_matches('\u{feff}'))
                    .map_err(|error| error.to_string())
            });
        let settings_load_error = settings_load_result.as_ref().err().cloned();
        let settings = settings_load_result.unwrap_or_default();

        let app_started_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        logger.info("app", "startup", "A Book in 30 Minutes started");
        logger.debug(
            "settings",
            "load",
            "閸氼垰濮╅弮鎯邦嚢閸欐牠鍘ょ純?",
            format!(
                "settings_path={} exists={} ai_key_present={} ai_key_length={} load_error={}",
                settings_path.to_string_lossy(),
                settings_file_exists,
                !settings.ai_profile.api_key.trim().is_empty(),
                settings.ai_profile.api_key.trim().chars().count(),
                settings_load_error.unwrap_or_else(|| "none".to_string())
            ),
            "startup",
        );
        init_app_tables(&db_path, &logger);

        Self {
            settings: Mutex::new(settings),
            settings_path,
            db_path,
            logger,
            app_started_at,
        }
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<(), CommandError> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                command_error(format!("Create settings directory failed: {error}"))
            })?;
        }
        let content = serde_json::to_string_pretty(settings)
            .map_err(|error| command_error(format!("Serialize settings failed: {error}")))?;
        fs::write(&self.settings_path, content)
            .map_err(|error| command_error(format!("Write settings failed: {error}")))?;
        self.logger.info("settings", "save", "闁板秶鐤嗗韫箽鐎?");
        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
}

#[tauri::command]
pub fn get_app_state(data: State<'_, AppData>) -> Result<AppStatePayload, CommandError> {
    data.logger
        .info("app", "get_app_state", "鐠囪褰囨惔鏃傛暏閻樿埖鈧?");
    Ok(AppStatePayload {
        settings: data.settings.lock().map_err(lock_error)?.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
pub fn get_settings(data: State<'_, AppData>) -> Result<AppSettings, CommandError> {
    data.logger.info("settings", "get", "鐠囪褰囬柊宥囩枂");
    Ok(data.settings.lock().map_err(lock_error)?.clone())
}

#[tauri::command]
pub fn set_settings(
    data: State<'_, AppData>,
    settings: AppSettings,
) -> Result<AppSettings, CommandError> {
    let mut current = data.settings.lock().map_err(lock_error)?;
    *current = settings.clone();
    data.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn check_update_mock(data: State<'_, AppData>) -> UpdateInfo {
    data.logger.info("update", "check", "Check update mock");
    UpdateInfo {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: env!("CARGO_PKG_VERSION").to_string(),
        available: false,
        notes: "Current dev build is up to date.".to_string(),
    }
}

#[tauri::command]
pub async fn test_ai_profile(data: State<'_, AppData>) -> Result<AiTestResult, CommandError> {
    data.logger.info("ai", "test_profile", "Testing AI profile");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let content = match call_ai(
        &settings,
        vec![ChatMessage {
            role: "user".to_string(),
            content: "Reply ok only.".to_string(),
        }],
    )
    .await
    {
        Ok(content) => content,
        Err(error) => {
            data.logger.error(
                "ai",
                "test_profile",
                "AI profile test failed",
                &error.message,
            );
            return Err(error);
        }
    };

    data.save_settings(&settings)?;
    data.logger
        .info("ai", "test_profile", "AI profile test succeeded");
    Ok(AiTestResult {
        ok: true,
        message: "AI profile test succeeded.".to_string(),
        content: Some(content),
    })
}

#[tauri::command]
pub async fn generate_ai_text(
    data: State<'_, AppData>,
    request: AiGenerateRequest,
) -> Result<AiGenerateResult, CommandError> {
    data.logger
        .info("ai", "generate_text", "瀵偓婵鏁撻幋?AI 閺傚洦婀?");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let mut messages = Vec::new();
    if let Some(system_prompt) = request
        .system_prompt
        .filter(|value| !value.trim().is_empty())
    {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        });
    }
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: request.prompt,
    });

    let content = match call_ai(&settings, messages).await {
        Ok(content) => content,
        Err(error) => {
            data.logger.error(
                "ai",
                "generate_text",
                "AI 閺傚洦婀伴悽鐔稿灇婢惰精瑙?",
                &error.message,
            );
            return Err(error);
        }
    };
    data.logger
        .info("ai", "generate_text", "AI 閺傚洦婀伴悽鐔稿灇閹存劕濮?");
    Ok(AiGenerateResult {
        content,
        model: settings.ai_profile.model,
    })
}

#[tauri::command]
pub async fn test_feishu_profile(
    data: State<'_, AppData>,
) -> Result<FeishuSendResult, CommandError> {
    data.logger
        .info("feishu", "test_profile", "Testing Feishu webhook");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let text = settings.feishu_profile.test_message.trim();
    let message = if text.is_empty() {
        "Feishu test message from A Book in 30 Minutes.".to_string()
    } else {
        text.to_string()
    };
    match call_feishu(&settings, &message).await {
        Ok(result) => {
            data.logger
                .info("feishu", "test_profile", "Feishu webhook test succeeded");
            Ok(result)
        }
        Err(error) => {
            data.logger.error(
                "feishu",
                "test_profile",
                "Feishu webhook test failed",
                &error.message,
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn send_feishu_message(
    data: State<'_, AppData>,
    request: FeishuSendRequest,
) -> Result<FeishuSendResult, CommandError> {
    data.logger
        .info("feishu", "send_message", "瀵偓婵褰傞柅渚€顥ｆ稊锔界Х閹?");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    match call_feishu(&settings, &request.text).await {
        Ok(result) => {
            data.logger.info(
                "feishu",
                "send_message",
                "妞嬬偘鍔熷☉鍫熶紖閸欐垿鈧焦鍨氶崝?",
            );
            Ok(result)
        }
        Err(error) => {
            data.logger.error(
                "feishu",
                "send_message",
                "妞嬬偘鍔熷☉鍫熶紖閸欐垿鈧礁銇戠拹?",
                &error.message,
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn generate_book_materials(
    app: tauri::AppHandle,
    data: State<'_, AppData>,
    request: BookMaterialsRequest,
) -> Result<BookMaterials, CommandError> {
    let started = Instant::now();
    let trace_id = build_trace_id(request.trace_id.as_deref());
    update_material_task_progress_db(
        &data.db_path,
        request.epub_path.trim(),
        "generating",
        10,
        "濮濓絽婀憴锝嗙€藉┃鎰姛濮濓絾鏋?",
    );
    data.logger.trace_info(
        "materials",
        "generate.start",
        "瀵偓婵鏁撻幋鎰拱濞?YouTube 閸氼兛鍔熺槐鐘虫綏",
        format!(
            "trace_id={} source={} target={}..{} channel={} language={} extra_direction_chars={}",
            trace_id,
            request.epub_path.trim(),
            request.target_min_chars,
            request.target_max_chars,
            request.channel_name.trim(),
            request.language.trim(),
            request.extra_direction.chars().count()
        ),
        &trace_id,
    );
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    data.logger.debug(
        "materials",
        "settings.snapshot",
        "鐠囪褰囬張顒侇偧閻㈢喐鍨氭担璺ㄦ暏閻?AI 闁板秶鐤?",
        ai_profile_debug_detail(&settings),
        &trace_id,
    );
    let epub_path = Path::new(request.epub_path.trim());
    if request.epub_path.trim().is_empty() {
        data.logger.trace_error(
            "materials",
            "generate.validate",
            "缁辩姵娼楃捄顖氱窞娑撹櫣鈹?",
            "鐠囧嘲鍘涙繅顐㈠晸缁辩姵娼楅弬鍥︽鐠侯垰绶為妴?",
            &trace_id,
        );
        return Err(command_error(
            "鐠囧嘲鍘涙繅顐㈠晸缁辩姵娼楅弬鍥︽鐠侯垰绶為妴?",
        ));
    }
    if !epub_path.exists() {
        data.logger.trace_error(
            "materials",
            "generate.validate",
            "缁辩姵娼楅弬鍥︽娑撳秴鐡ㄩ崷?",
            request.epub_path.trim(),
            &trace_id,
        );
        return Err(command_error(
            "缁辩姵娼楅弬鍥︽娑撳秴鐡ㄩ崷顭掔礉鐠囬攱顥呴弻銉ㄧ熅瀵板嫨鈧?",
        ));
    }
    data.logger.debug(
        "materials",
        "source.file",
        "濠ф劖鏋冩禒鍫曠崣鐠囦線鈧俺绻?",
        source_file_detail(epub_path),
        &trace_id,
    );

    let read_started = Instant::now();
    emit_material_progress(
        &app,
        &trace_id,
        request.epub_path.trim(),
        1,
        "鐟欙絾鐎藉┃鎰姛濮濓絾鏋?",
    );
    data.logger.trace_info(
        "materials",
        "source.read",
        "瀵偓婵袙閺嬫劖绨稊锔筋劀閺?",
        source_file_detail(epub_path),
        &trace_id,
    );
    let book = match read_source_book_with_timeout(
        epub_path.to_path_buf(),
        Duration::from_secs(SOURCE_READ_TIMEOUT_SECONDS),
    ) {
        SourceReadResult::Ok(book) => {
            data.logger.trace_info(
                "materials",
                "source.read.done",
                "濠ф劒鍔熷锝嗘瀮鐟欙絾鐎界€瑰本鍨?",
                format!(
                    "elapsed_ms={} title={} creator={} publisher={} language={} total_han_chars={} chapters={} first_chapters={}",
                    read_started.elapsed().as_millis(),
                    book.overview.title,
                    book.overview.creator,
                    book.overview.publisher,
                    book.overview.language,
                    book.overview.total_chars,
                    book.chapters.len(),
                    chapter_debug_list(&book)
                ),
                &trace_id,
            );
            update_material_task_progress_db(
                &data.db_path,
                request.epub_path.trim(),
                "generating",
                25,
                "濠ф劒鍔熷锝嗘瀮鐟欙絾鐎界€瑰本鍨氶敍灞绢劀閸︺劍鐎?AI 閹绘劗銇氱拠?",
            );
            book
        }
        SourceReadResult::Err(error) => {
            data.logger.trace_error(
                "materials",
                "source.read.failed",
                "Source read failed",
                &error.message,
                &trace_id,
            );
            return Err(error);
        }
        SourceReadResult::Panic(message) => {
            data.logger.trace_error(
                "materials",
                "source.read.panic",
                "Source reader panicked",
                &message,
                &trace_id,
            );
            return Err(command_error(format!("Source reader panicked: {message}")));
        }
        SourceReadResult::Timeout => {
            let detail = format!(
                "elapsed_ms={} timeout_seconds={} {}",
                read_started.elapsed().as_millis(),
                SOURCE_READ_TIMEOUT_SECONDS,
                source_file_detail(epub_path)
            );
            data.logger.trace_error(
                "materials",
                "source.read.timeout",
                "濠ф劒鍔熷锝嗘瀮鐟欙絾鐎界搾鍛",
                detail,
                &trace_id,
            );
            return Err(command_error(format!(
                "濠ф劒鍔熷锝嗘瀮鐟欙絾鐎界搾鍛扮箖 {} 缁夋帗婀€瑰本鍨氶敍灞藉嚒閸嬫粍顒涚粵澶婄窡閵嗗倽顕Λ鈧弻?EPUB 閺勵垰鎯侀幑鐔锋綎閿涘本鍨ㄩ弻銉ф箙閺堫剚顐兼禒璇插閺冦儱绻旂€规矮缍呴崡鈥茬秶娴ｅ秶鐤嗛妴?",
                SOURCE_READ_TIMEOUT_SECONDS
            )));
        }
    };
    let prompt = build_book_materials_prompt(&book, &request);
    update_material_task_progress_db(
        &data.db_path,
        request.epub_path.trim(),
        "generating",
        35,
        "AI 閹绘劗銇氱拠宥嗙€鍝勭暚閹存劧绱濋崙鍡楊槵鐠囬攱鐪扮槐鐘虫綏 JSON",
    );
    data.logger.debug(
        "materials",
        "prompt.build",
        "閻㈢喐鍨氶幓鎰仛鐠囧秵鐎鍝勭暚閹?",
        format!(
            "prompt_chars={} prompt_han_chars={} prompt_preview={}",
            prompt.chars().count(),
            count_han_chars(&prompt),
            text_preview(&prompt, 360)
        ),
        &trace_id,
    );
    let system_prompt =
        "娴ｇ姵妲告稉鈧稉顏冭厬閺?YouTube 閸氼兛鍔熺憴鍡涱暥缁涙牕鍨濋崪灞炬⒑閻х晫顭堟担婊嗏偓鍛偓鍌欑稑閸欘亣绶崙杞板紬閺?JSON閿涘奔绗夋潏鎾冲毉 Markdown閵?"
            .to_string();
    let ai_started = Instant::now();
    emit_material_progress(
        &app,
        &trace_id,
        request.epub_path.trim(),
        2,
        "鐠囬攱鐪?AI 閻㈢喐鍨氱槐鐘虫綏",
    );
    update_material_task_progress_db(
        &data.db_path,
        request.epub_path.trim(),
        "generating",
        45,
        "濮濓絽婀拠閿嬬湴 AI 閻㈢喐鍨氱槐鐘虫綏 JSON",
    );
    data.logger.trace_info(
        "materials",
        "ai.request",
        "瀵偓婵顕Ч?AI 閻㈢喐鍨氱槐鐘虫綏 JSON",
        format!(
            "model={} messages=2 system_prompt_chars={} user_prompt_chars={} base_url={}",
            settings.ai_profile.model,
            system_prompt.chars().count(),
            prompt.chars().count(),
            settings.ai_profile.base_url
        ),
        &trace_id,
    );
    let content = match call_ai(
        &settings,
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            },
        ],
    )
    .await
    {
        Ok(content) => {
            data.logger.trace_info(
                "materials",
                "ai.response",
                "AI 閸掓繄顭堟潻鏂挎礀閹存劕濮?",
                format!(
                    "elapsed_ms={} response_chars={} response_han_chars={} response_preview={}",
                    ai_started.elapsed().as_millis(),
                    content.chars().count(),
                    count_han_chars(&content),
                    text_preview(&content, 300)
                ),
                &trace_id,
            );
            content
        }
        Err(error) => {
            update_material_task_progress_db(
                &data.db_path,
                request.epub_path.trim(),
                "generating",
                45,
                "AI 閸掓繄顭堟径杈Е閿涘本顒滈崷銊ゅ▏閻劍绨稊锕€鍞寸€硅婀伴崷鎵晸閹存劗绀岄弶?",
            );
            data.logger.trace_error(
                "materials",
                "ai.request.failed",
                "缁辩姵娼楅悽鐔稿灇 AI 鐠囬攱鐪版径杈Е閿涘本鏁奸悽銊︽拱閸︾増绨稊锕€鍘规惔?",
                &error.message,
                &trace_id,
            );
            String::new()
        }
    };

    let mut payload = if content.trim().is_empty() {
        let payload = build_local_book_materials_payload(&book, &request);
        data.logger.warn(
            "materials",
            "ai.local_initial_fallback",
            "AI 閸掓繄顭堟稉宥呭讲閻㈩煉绱濆韫▏閻劍绨稊锔芥拱閸︽壆鏁撻幋鎰灥缁?",
            format!(
                "title={} narration_han_chars={} tags={}",
                payload.video_title,
                count_han_chars(&payload.narration),
                payload.tags.len()
            ),
            &trace_id,
        );
        payload
    } else {
        match parse_book_materials_payload(&content) {
            Ok(payload) => {
                data.logger.debug(
                    "materials",
                    "ai.parse",
                    "AI 閸掓繄顭?JSON 鐟欙絾鐎介幋鎰",
                    format!(
                        "title={} description_chars={} tags={} narration_han_chars={}",
                        payload.video_title,
                        payload.description.chars().count(),
                        payload.tags.len(),
                        count_han_chars(&payload.narration)
                    ),
                    &trace_id,
                );
                payload
            }
            Err(error) => {
                data.logger.trace_error(
                    "materials",
                    "ai.parse.failed",
                    "AI 閸掓繄顭?JSON 鐟欙絾鐎芥径杈Е",
                    &error.message,
                    &trace_id,
                );
                return Err(error);
            }
        }
    };
    let min_chars = request.target_min_chars.max(1000);
    let max_chars = request.target_max_chars.max(min_chars + 1);
    for repair_attempt in 1..=8 {
        let narration_chars = count_han_chars(&payload.narration);
        if narration_chars >= min_chars && narration_chars <= max_chars {
            data.logger.debug(
                "materials",
                "ai.repair.skip",
                "閺冧胶娅х€涙鏆熷鍙夊姬鐡掑磭娲伴弽鍥瘱閸ヨ揪绱濈捄瀹犵箖娣囶喖顦?",
                format!(
                    "narration_han_chars={} target={}..{} repair_attempts_checked={}",
                    narration_chars,
                    min_chars,
                    max_chars,
                    repair_attempt - 1
                ),
                &trace_id,
            );
            break;
        }
        data.logger.warn(
            "materials",
            "ai.repair.required",
            "閺冧胶娅х€涙鏆熸稉宥呮躬閻╊喗鐖ｉ懠鍐ㄦ纯閿涘苯鍣径鍥у絺鐠ц渹鎱ㄦ径?",
            format!(
                "attempt={} current_han_chars={} target={}..{} title={}",
                repair_attempt, narration_chars, min_chars, max_chars, payload.video_title
            ),
            &trace_id,
        );
        let repair_prompt = if narration_chars < min_chars {
            build_narration_extension_prompt(&payload, narration_chars, min_chars, max_chars)
        } else {
            build_repair_prompt(&payload, min_chars, max_chars)
        };
        let repair_started = Instant::now();
        data.logger.debug(
            "materials",
            "ai.repair.request",
            "瀵偓婵顕Ч?AI 娣囶喖顦茬槐鐘虫綏 JSON",
            format!(
                "repair_prompt_chars={} repair_prompt_preview={}",
                repair_prompt.chars().count(),
                text_preview(&repair_prompt, 300)
            ),
            &trace_id,
        );
        if let Ok(repaired) = call_ai(
            &settings,
            vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: repair_prompt,
                },
            ],
        )
        .await
        {
            data.logger.debug(
                "materials",
                "ai.repair.response",
                "AI 娣囶喖顦茬粙鑳箲閸?",
                format!(
                    "elapsed_ms={} response_chars={} response_han_chars={} response_preview={}",
                    repair_started.elapsed().as_millis(),
                    repaired.chars().count(),
                    count_han_chars(&repaired),
                    text_preview(&repaired, 240)
                ),
                &trace_id,
            );
            if narration_chars < min_chars {
                let extension = clean_narration_extension(&repaired);
                if !extension.trim().is_empty() {
                    payload.narration = merge_narration_extension(&payload.narration, &extension);
                    data.logger.trace_info(
                        "materials",
                        "ai.repair.done",
                        "AI 鏉╄棄濮為弮浣烘鐟欙絾鐎介幋鎰閿涘苯鍑￠崥鍫濊嫙閸掓澘鍨电粙?",
                        format!(
                            "extension_han_chars={} narration_han_chars={} title={}",
                            count_han_chars(&extension),
                            count_han_chars(&payload.narration),
                            payload.video_title
                        ),
                        &trace_id,
                    );
                } else {
                    data.logger.warn(
                        "materials",
                        "ai.repair.parse_failed",
                        "AI 鏉╄棄濮為弮浣烘娑撹櫣鈹栭敍宀€鎴风紒顓濆▏閻劎骞囬張澶岊焾娴?",
                        "娣囶喖顦叉潻鏂挎礀濞屸剝婀侀崣顖滄暏閺冧胶娅ч弬鍥ㄦ拱",
                        &trace_id,
                    );
                    break;
                }
            } else if let Ok(next_payload) = parse_book_materials_payload(&repaired) {
                data.logger.trace_info(
                    "materials",
                    "ai.repair.done",
                    "AI 娣囶喖顦茬粙鑳掗弸鎰灇閸旂噦绱濆鍙夋禌閹广垹鍨电粙?",
                    format!(
                        "narration_han_chars={} tags={} title={}",
                        count_han_chars(&next_payload.narration),
                        next_payload.tags.len(),
                        next_payload.video_title
                    ),
                    &trace_id,
                );
                payload = next_payload;
            } else {
                data.logger.warn(
                    "materials",
                    "ai.repair.parse_failed",
                    "AI 娣囶喖顦茬粙鎸庢￥濞夋洝袙閺嬫劧绱濈紒褏鐢绘担璺ㄦ暏閸掓繄顭?",
                    "娣囶喖顦叉潻鏂挎礀娑撳秵妲搁張澶嬫櫏缁辩姵娼?JSON",
                    &trace_id,
                );
                break;
            }
        } else {
            data.logger.warn(
                "materials",
                "ai.repair.failed",
                "AI 娣囶喖顦茬拠閿嬬湴婢惰精瑙﹂敍灞藉櫙婢跺洣濞囬悽銊︽拱閸︽媽藟鐡?",
                "閸氬海鐢绘导姘辨暏濠ф劒鍔熼幗妯虹秿鐞涖儴鍐婚弮浣烘鐎涙鏆?",
                &trace_id,
            );
            break;
        }
    }
    let before_fallback_chars = count_han_chars(&payload.narration);
    if before_fallback_chars < min_chars {
        let fallback =
            build_local_narration_extension(&payload, before_fallback_chars, min_chars, max_chars);
        if !fallback.trim().is_empty() {
            payload.narration = merge_narration_extension(&payload.narration, &fallback);
            data.logger.warn(
                "materials",
                "ai.repair.local_fallback",
                "AI 閺冧胶娅ф禒宥勭瑝鐡掔绱濆韫▏閻劍绨稊锔芥喅瑜版洘婀伴崷鎷屗夌搾?",
                format!(
                    "before_han_chars={} fallback_han_chars={} after_han_chars={} target={}..{}",
                    before_fallback_chars,
                    count_han_chars(&fallback),
                    count_han_chars(&payload.narration),
                    min_chars,
                    max_chars
                ),
                &trace_id,
            );
        }
    }
    let final_narration_chars = count_han_chars(&payload.narration);
    if final_narration_chars < min_chars || final_narration_chars > max_chars {
        data.logger.trace_error(
            "materials",
            "ai.repair.out_of_range",
            "AI 娣囶喖顦查崥搴㈡⒑閻ц棄鐡ч弫棰佺矝娑撳秴婀惄顔界垼閼煎啫娲?",
            &format!(
                "final_han_chars={} target={}..{} title={}",
                final_narration_chars, min_chars, max_chars, payload.video_title
            ),
            &trace_id,
        );
        return Err(command_error(format!(
            "AI 閻㈢喐鍨氶惃鍕⒑閻ф垝鑵戦弬鍥х摟閺侀璐?{}閿涘本婀潏鎯у煂闁板秶鐤嗙憰浣圭湴 {}-{}閵嗗倽顕粙宥呮倵闁插秷鐦敍灞惧灗閸︺劏顔曠純顔昏厬闁倸缍嬮梽宥勭秵閻╊喗鐖ｇ€涙鏆熼妴?",
            final_narration_chars, min_chars, max_chars
        )));
    }
    emit_material_progress(
        &app,
        &trace_id,
        request.epub_path.trim(),
        3,
        "閺佸鎮婄紒鎾寸亯閸滃苯鐡ч獮?",
    );
    let subtitles = split_subtitles(&payload.narration);
    data.logger.trace_info(
        "materials",
        "subtitle.split",
        "鐎涙绠烽弬鍥ㄦ拱閸掑洤鍨庣€瑰本鍨?",
        format!(
            "subtitle_lines={} narration_han_chars={} first_line={} last_line={}",
            subtitles.len(),
            count_han_chars(&payload.narration),
            subtitles.first().cloned().unwrap_or_default(),
            subtitles.last().cloned().unwrap_or_default()
        ),
        &trace_id,
    );
    let materials = BookMaterials {
        video_title: payload.video_title,
        description: payload.description,
        tags: payload.tags,
        narration: payload.narration,
        subtitles,
        prompt,
        model: settings.ai_profile.model.clone(),
        overview: book.overview,
    };
    data.logger.trace_info(
        "materials",
        "generate.done",
        "閺堫剚顐?YouTube 閸氼兛鍔熺槐鐘虫綏閻㈢喐鍨氶幋鎰",
        format!(
            "elapsed_ms={} video_title={} model={} tags={} narration_han_chars={} subtitles={} source={}",
            started.elapsed().as_millis(),
            materials.video_title,
            materials.model,
            materials.tags.len(),
            count_han_chars(&materials.narration),
            materials.subtitles.len(),
            request.epub_path.trim()
        ),
        &trace_id,
    );
    let material_base_dir = source_output_dir(epub_path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    match write_book_materials_package(&data, &material_base_dir, &materials, &trace_id) {
        Ok(result) => {
            data.logger.trace_info(
                "materials",
                "generate.auto_export.done",
                "缁辩姵娼楅悽鐔稿灇鐎瑰本鍨氶崥搴″嚒閼奉亜濮╅崘娆忓弳缁辩姵娼楅弬鍥︽婢?",
                format!(
                    "files={} output_dir={}",
                    result.files.len(),
                    result.output_dir
                ),
                &trace_id,
            );
            if let Ok(connection) = Connection::open(&data.db_path) {
                let _ = ensure_material_tasks_table(&connection).and_then(|_| {
                    update_material_task_output_dir(
                        &connection,
                        request.epub_path.trim(),
                        &result.output_dir,
                    )
                });
            }
        }
        Err(error) => {
            data.logger.trace_error(
                "materials",
                "generate.auto_export.failed",
                "缁辩姵娼楅弬鍥︽婢剁鍤滈崝銊ュ晸閸忋儱銇戠拹?",
                &error.message,
                &trace_id,
            );
        }
    }
    notify_generation_completed(
        &data,
        &settings,
        &materials,
        started.elapsed(),
        request.epub_path.trim(),
        &trace_id,
    )
    .await;
    emit_material_progress(
        &app,
        &trace_id,
        request.epub_path.trim(),
        4,
        "閻㈢喐鍨氱€瑰本鍨?",
    );
    Ok(materials)
}

#[tauri::command]
pub fn scan_material_files(
    data: State<'_, AppData>,
    request: ScanMaterialFilesRequest,
) -> Result<ScanMaterialFilesResult, CommandError> {
    let started = Instant::now();
    data.logger.info(
        "materials",
        "scan",
        format!(
            "瀵偓婵澹傞幓蹇曠閺夋劖鏋冩禒璁圭窗{}",
            request.path.trim()
        ),
    );
    let input = request.path.trim();
    if input.is_empty() {
        return Err(command_error(
            "鐠囧嘲鍘涙繅顐㈠晸閺傚洣娆㈡径瑙勫灗閺傚洣娆㈢捄顖氱窞閵?",
        ));
    }
    let path = PathBuf::from(input);
    if !path.exists() {
        return Err(command_error(
            "鐠侯垰绶炴稉宥呯摠閸︻煉绱濈拠閿嬵梾閺屻儱鎮楅柌宥堢槸閵?",
        ));
    }

    let directory = if path.is_dir() {
        path
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| command_error("閺冪姵纭剁€规矮缍呴弬鍥︽閹碘偓閸︺劍鏋冩禒璺恒仚閵?"))?
    };

    data.logger.info(
        "materials",
        "scan.resolve",
        format!("Scan material directory: {}", directory.to_string_lossy()),
    );
    let connection = Connection::open(&data.db_path)
        .map_err(|error| command_error(format!("Open material task database failed: {error}")))?;
    ensure_material_tasks_table(&connection)?;
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let category = normalize_material_category(&settings.material_profile.category_name);
    let mut files = Vec::new();
    for entry in fs::read_dir(&directory)
        .map_err(|error| command_error(format!("Read material directory failed: {error}")))?
    {
        let entry = entry.map_err(|error| {
            command_error(format!("Read material directory entry failed: {error}"))
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "epub" | "pdf" | "txt" | "docx") {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| {
            command_error(format!("Read material file metadata failed: {error}"))
        })?;
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_string();
        let file = MaterialFile {
            path: path.to_string_lossy().into_owned(),
            name,
            extension,
            size: metadata.len(),
            category: category.clone(),
            status: "pending".to_string(),
            progress: 0,
            narration_chars: None,
            material_output_dir: None,
            message: String::new(),
            audio_status: "pending".to_string(),
            audio_progress: 0,
            audio_output_dir: None,
            audio_file: None,
            audio_duration_ms: None,
            audio_chunks: None,
            audio_message: String::new(),
            video_status: "pending".to_string(),
            video_progress: 0,
            video_file: None,
            video_duration_ms: None,
            video_file_size: None,
            video_message: String::new(),
        };
        upsert_material_task(&connection, &file)?;
        files.push(load_material_task_by_path(&connection, &file.path)?.unwrap_or(file));
    }
    files.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    let supported = files.len();
    let parsable = files
        .iter()
        .filter(|file| matches!(file.extension.as_str(), "epub" | "txt"))
        .count();
    data.logger.info(
        "materials",
        "scan.done",
        format!(
            "缁辩姵娼楅弬鍥︽閹殿偅寮块幋鎰閿涙瓲iles={} supported={} parsable={} elapsed_ms={} directory={}",
            files.len(),
            supported,
            parsable,
            started.elapsed().as_millis(),
            directory.to_string_lossy()
        ),
    );
    Ok(ScanMaterialFilesResult {
        directory: directory.to_string_lossy().into_owned(),
        files,
    })
}

#[tauri::command]
pub fn get_material_tasks(
    data: State<'_, AppData>,
    request: GetMaterialTasksRequest,
) -> Result<ScanMaterialFilesResult, CommandError> {
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "閹垫挸绱戞禒璇插閺佺増宓佹惔鎾炽亼鐠愩儻绱皗{error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    let category = request.category.unwrap_or_default().trim().to_string();
    let mut files = if category.is_empty() {
        let mut statement = connection
            .prepare(
                "SELECT path, name, extension, size, category, status, progress, narration_chars, material_output_dir, message, audio_status, audio_progress, audio_output_dir, audio_file, audio_duration_ms, audio_chunks, audio_message, video_status, video_progress, video_file, video_duration_ms, video_file_size, video_message FROM material_tasks ORDER BY updated_at DESC, name ASC"
            )
            .map_err(|error| command_error(format!("閸戝棗顦拠璇插絿缁辩姵娼楁禒璇插婢惰精瑙﹂敍姝縠{error}")))?;
        let rows = statement
            .query_map([], material_task_from_row)
            .map_err(|error| {
                command_error(format!("鐠囪褰囩槐鐘虫綏娴犺濮熸径杈Е閿涙{error}"))
            })?;
        collect_material_tasks(rows)?
    } else {
        let mut statement = connection
            .prepare(
                "SELECT path, name, extension, size, category, status, progress, narration_chars, material_output_dir, message, audio_status, audio_progress, audio_output_dir, audio_file, audio_duration_ms, audio_chunks, audio_message, video_status, video_progress, video_file, video_duration_ms, video_file_size, video_message FROM material_tasks WHERE category = ?1 ORDER BY updated_at DESC, name ASC"
            )
            .map_err(|error| command_error(format!("閸戝棗顦拠璇插絿缁辩姵娼楁禒璇插婢惰精瑙﹂敍姝縠{error}")))?;
        let rows = statement
            .query_map(params![category], material_task_from_row)
            .map_err(|error| {
                command_error(format!("鐠囪褰囩槐鐘虫綏娴犺濮熸径杈Е閿涙{error}"))
            })?;
        collect_material_tasks(rows)?
    };
    files.retain(|file| Path::new(&file.path).exists());
    for file in &mut files {
        let _ = migrate_task_outputs_to_source_output(&connection, file);
    }
    data.logger.info(
        "materials",
        "tasks.get",
        format!("Loaded material tasks: {}", files.len()),
    );
    Ok(ScanMaterialFilesResult {
        directory: String::new(),
        files,
    })
}

#[tauri::command]
pub fn update_material_task_status(
    data: State<'_, AppData>,
    request: UpdateMaterialTaskStatusRequest,
) -> Result<MaterialFile, CommandError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(command_error("娴犺濮熺捄顖氱窞娑撳秷鍏樻稉铏光敄閵?"));
    }
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "閹垫挸绱戞禒璇插閺佺増宓佹惔鎾炽亼鐠愩儻绱皗{error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    let progress = clamp_task_progress(request.progress);
    let status = normalize_task_status(&request.status);
    let message = request.message.unwrap_or_default();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    connection
        .execute(
            r#"
            UPDATE material_tasks
            SET status = ?2,
                progress = ?3,
                narration_chars = ?4,
                material_output_dir = COALESCE(?5, material_output_dir),
                message = ?6,
                updated_at = ?7
            WHERE path = ?1
            "#,
            params![
                path,
                status,
                progress,
                request.narration_chars,
                request.material_output_dir,
                message,
                now
            ],
        )
        .map_err(|error| command_error(format!("Update material task status failed: {error}")))?;
    if connection.changes() == 0 {
        let category = request
            .category
            .as_deref()
            .unwrap_or(DEFAULT_MATERIAL_CATEGORY);
        upsert_material_task(&connection, &material_file_from_path(path, category)?)?;
        connection
            .execute(
                r#"
                UPDATE material_tasks
                SET status = ?2,
                    progress = ?3,
                    narration_chars = ?4,
                    material_output_dir = COALESCE(?5, material_output_dir),
                    message = ?6,
                    updated_at = ?7
                WHERE path = ?1
                "#,
                params![
                    path,
                    status,
                    progress,
                    request.narration_chars,
                    request.material_output_dir,
                    message,
                    now
                ],
            )
            .map_err(|error| {
                command_error(format!("Update material task status failed: {error}"))
            })?;
    }
    data.logger.info(
        "materials",
        "tasks.status",
        format!("閺囧瓨鏌婄槐鐘虫綏娴犺濮熼悩鑸碘偓渚婄窗path={path} status={status} progress={progress}"),
    );
    load_material_task_by_path(&connection, path)?
        .ok_or_else(|| command_error("閺囧瓨鏌婇崥搴㈡弓閹垫儳鍩岀槐鐘虫綏娴犺濮熼妴?"))
}

#[tauri::command]
pub fn remove_material_task(
    data: State<'_, AppData>,
    request: MaterialTaskPathRequest,
) -> Result<bool, CommandError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(command_error("娴犺濮熺捄顖氱窞娑撳秷鍏樻稉铏光敄閵?"));
    }
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "閹垫挸绱戞禒璇插閺佺増宓佹惔鎾炽亼鐠愩儻绱皗{error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    connection
        .execute("DELETE FROM material_tasks WHERE path = ?1", params![path])
        .map_err(|error| command_error(format!("缁夊娅庣槐鐘虫綏娴犺濮熸径杈Е閿涙{error}")))?;
    data.logger.info(
        "materials",
        "tasks.remove",
        format!("缁夊娅庣槐鐘虫綏娴犺濮熼敍姝盿th={path}"),
    );
    Ok(true)
}

#[tauri::command]
pub fn reset_material_tasks(
    data: State<'_, AppData>,
    request: ResetMaterialTasksRequest,
) -> Result<bool, CommandError> {
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "閹垫挸绱戞禒璇插閺佺増宓佹惔鎾炽亼鐠愩儻绱皗{error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if let Some(path) = request
        .path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        connection
            .execute(
                "UPDATE material_tasks SET status = 'pending', progress = 0, narration_chars = NULL, message = '', updated_at = ?2 WHERE path = ?1",
                params![path, now],
            )
            .map_err(|error| command_error(format!("闁插秶鐤嗙槐鐘虫綏娴犺濮熸径杈Е閿涙{error}")))?;
        data.logger.info(
            "materials",
            "tasks.reset",
            format!("闁插秶鐤嗙槐鐘虫綏娴犺濮熼敍姝盿th={path}"),
        );
    } else {
        connection
            .execute(
                "UPDATE material_tasks SET status = 'pending', progress = 0, narration_chars = NULL, message = '', updated_at = ?1",
                params![now],
            )
            .map_err(|error| command_error(format!("閹靛綊鍣洪柌宥囩枂缁辩姵娼楁禒璇插婢惰精瑙﹂敍姝縠{error}")))?;
        data.logger.info(
            "materials",
            "tasks.reset_all",
            "閹靛綊鍣洪柌宥囩枂缁辩姵娼楁禒璇插閻樿埖鈧?",
        );
    }
    Ok(true)
}

#[tauri::command]
pub fn export_book_materials(
    data: State<'_, AppData>,
    request: ExportBookMaterialsRequest,
) -> Result<ExportBookMaterialsResult, CommandError> {
    let started = Instant::now();
    let trace_id = build_trace_id(request.trace_id.as_deref());
    data.logger.trace_info(
        "materials",
        "export.start",
        "瀵偓婵顕遍崙?YouTube 閸氼兛鍔熺槐鐘虫綏閸?",
        format!(
            "title={} model={} output_dir={}",
            request.materials.video_title,
            request.materials.model,
            request.output_dir.trim()
        ),
        &trace_id,
    );
    let result = write_book_materials_package(
        &data,
        request.output_dir.trim(),
        &request.materials,
        &trace_id,
    )?;

    data.logger.trace_info(
        "materials",
        "export.done",
        "YouTube 閸氼兛鍔熺槐鐘虫綏閸栧懎顕遍崙鐑樺灇閸?",
        format!(
            "files={} elapsed_ms={} output_dir={}",
            result.files.len(),
            started.elapsed().as_millis(),
            result.output_dir
        ),
        &trace_id,
    );
    Ok(result)
}

#[tauri::command]
pub fn open_material_output_dir(
    data: State<'_, AppData>,
    request: MaterialOutputDirRequest,
) -> Result<bool, CommandError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(command_error("娴犺濮熺捄顖氱窞娑撳秷鍏樻稉铏光敄閵?"));
    }
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "閹垫挸绱戞禒璇插閺佺増宓佹惔鎾炽亼鐠愩儻绱皗{error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    let file = load_material_task_by_path(&connection, path)?
        .ok_or_else(|| command_error("閺堫亝澹橀崚鎷岊嚉缁辩姵娼楁禒璇插閵?"))?;
    let output_path = if let Some(output_dir) = file
        .material_output_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let saved = PathBuf::from(output_dir);
        if saved.exists() && material_output_dir_matches(&file, &saved) {
            saved
        } else if let Some(found) = find_existing_material_output_dir(&data, &file)? {
            let output_dir = found.to_string_lossy().into_owned();
            update_material_task_output_dir(&connection, path, &output_dir)?;
            found
        } else {
            clear_material_task_output_dir(&connection, path)?;
            return Err(command_error("閺堫亝澹橀崚棰佺瑢瑜版挸澧犳禒璇插閸栧綊鍘ら惃鍕晸閹存劗绀岄弶鎰瀮娴犺泛銇欓妴鍌濐嚞閻愮懓鍤妴鎰閺夋劑鈧垿鍣搁弬鎵晸閹存劒绔村▎鈽呯礉鐠佲晝閮寸紒鐔峰晸閸忋儲鏌婇惃鍕閺夋劕瀵橀惄顔肩秿閵?"));
        }
    } else if let Some(found) = find_existing_material_output_dir(&data, &file)? {
        let output_dir = found.to_string_lossy().into_owned();
        update_material_task_output_dir(&connection, path, &output_dir)?;
        found
    } else {
        return Err(command_error(
            "鏉╂ɑ鐥呴張澶嬪閸掓壆鏁撻幋鎰倵閻ㄥ嫮绀岄弶鎰瀮娴犺泛銇欓妴鍌濐嚞閸忓牏鏁撻幋鎰閺夋劧绱濋幋鏍櫢閺傛壆鏁撻幋鎰濞嗏€蹭簰閸愭瑥鍙嗙槐鐘虫綏閺傚洣娆㈡径骞库偓?",
        ));
    };
    if !output_path.exists() {
        return Err(command_error(format!(
            "Material output directory does not exist: {}",
            output_path.to_string_lossy()
        )));
    }
    open_directory_in_explorer(&output_path)?;
    data.logger.info(
        "materials",
        "output_dir.open",
        format!(
            "Open material output directory: {}",
            output_path.to_string_lossy()
        ),
    );
    Ok(true)
}

#[tauri::command]
pub fn test_ffmpeg_path(data: State<'_, AppData>) -> Result<ToolTestResult, CommandError> {
    data.logger
        .info("audio", "ffmpeg.test", "Testing ffmpeg path");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let version = run_ffmpeg_version(&settings.tool_profile.ffmpeg_path)?;
    data.save_settings(&settings)?;
    Ok(ToolTestResult {
        ok: true,
        message: "ffmpeg test succeeded.".to_string(),
        version: Some(version),
    })
}

#[tauri::command]
pub async fn test_speech_profile(
    data: State<'_, AppData>,
) -> Result<SpeechTestResult, CommandError> {
    let started = Instant::now();
    data.logger
        .info("audio", "speech.test", "Testing speech profile");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    validate_speech_profile(&settings.speech_profile)?;
    let output_dir = resolve_audio_base_dir(&data, "")?.join("tests");
    fs::create_dir_all(&output_dir)
        .map_err(|error| command_error(format!("Create speech test output dir failed: {error}")))?;
    let audio_file = output_dir.join(format!(
        "speech_test_{}.mp3",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));
    let ssml = build_ssml("This is a speech test.", &settings.speech_profile);
    synthesize_speech_to_file(&settings.speech_profile, &ssml, &audio_file).await?;
    data.save_settings(&settings)?;
    data.logger.info(
        "audio",
        "speech.test.done",
        format!(
            "Speech test done elapsed_ms={}",
            started.elapsed().as_millis()
        ),
    );
    Ok(SpeechTestResult {
        ok: true,
        message: "Speech test succeeded.".to_string(),
        audio_file: Some(audio_file.to_string_lossy().into_owned()),
        audio_data_url: None,
    })
}

#[tauri::command]
pub async fn preview_speech(
    data: State<'_, AppData>,
    request: SpeechPreviewRequest,
) -> Result<SpeechTestResult, CommandError> {
    let started = Instant::now();
    data.logger
        .info("audio", "speech.preview", "Generating speech preview");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    validate_speech_profile(&settings.speech_profile)?;
    let text = request.text.as_deref().unwrap_or("").trim();
    if text.is_empty() {
        return Err(command_error("Preview text cannot be empty."));
    }
    let output_dir = resolve_audio_base_dir(&data, "")?.join("previews");
    fs::create_dir_all(&output_dir).map_err(|error| {
        command_error(format!("Create speech preview output dir failed: {error}"))
    })?;
    let audio_file = output_dir.join(format!(
        "speech_preview_{}_{}.mp3",
        sanitize_file_name(&settings.speech_profile.voice_name),
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ));
    let ssml = build_ssml(text, &settings.speech_profile);
    synthesize_speech_to_file(&settings.speech_profile, &ssml, &audio_file).await?;
    let audio_data_url = build_audio_data_url(&audio_file)?;
    data.logger.info(
        "audio",
        "speech.preview.done",
        format!(
            "Speech preview done chars={} elapsed_ms={}",
            text.chars().count(),
            started.elapsed().as_millis()
        ),
    );
    Ok(SpeechTestResult {
        ok: true,
        message: "Speech preview generated.".to_string(),
        audio_file: Some(audio_file.to_string_lossy().into_owned()),
        audio_data_url: Some(audio_data_url),
    })
}

#[tauri::command]
pub fn save_speech_region_key(
    data: State<'_, AppData>,
    request: SpeechRegionKeyRequest,
) -> Result<SpeechRegionKeyResult, CommandError> {
    let region = request.region.trim();
    if region.is_empty() {
        return Err(command_error("Region cannot be empty."));
    }
    let speech_key = request.speech_key.trim();
    if speech_key.is_empty() {
        return Err(command_error("Speech key cannot be empty."));
    }
    let connection = Connection::open(&data.db_path)
        .map_err(|error| command_error(format!("Open database failed: {error}")))?;
    ensure_speech_region_key_table(&connection)?;
    let current_settings = data.settings.lock().map_err(lock_error)?.clone();
    let voice_name = request
        .voice_name
        .as_deref()
        .unwrap_or(&current_settings.speech_profile.voice_name)
        .trim()
        .to_string();
    let output_format = request
        .output_format
        .as_deref()
        .unwrap_or(&current_settings.speech_profile.output_format)
        .trim()
        .to_string();
    let rate = request
        .rate
        .as_deref()
        .unwrap_or(&current_settings.speech_profile.rate)
        .trim()
        .to_string();
    let pitch = request
        .pitch
        .as_deref()
        .unwrap_or(&current_settings.speech_profile.pitch)
        .trim()
        .to_string();
    connection
        .execute(
            r#"
            INSERT INTO speech_region_keys (region, speech_key, voice_name, output_format, rate, pitch, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(region) DO UPDATE SET
              speech_key = excluded.speech_key,
              voice_name = excluded.voice_name,
              output_format = excluded.output_format,
              rate = excluded.rate,
              pitch = excluded.pitch,
              updated_at = excluded.updated_at
            "#,
            params![region, speech_key, voice_name, output_format, rate, pitch, chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()],
        )
        .map_err(|error| command_error(format!("Save speech region key failed: {error}")))?;
    let mut settings = data.settings.lock().map_err(lock_error)?;
    settings.speech_profile.region = region.to_string();
    settings.speech_profile.speech_key = speech_key.to_string();
    settings.speech_profile.voice_name = voice_name.clone();
    settings.speech_profile.output_format = output_format.clone();
    settings.speech_profile.rate = rate.clone();
    settings.speech_profile.pitch = pitch.clone();
    settings
        .speech_profile
        .region_keys
        .insert(region.to_string(), speech_key.to_string());
    data.save_settings(&settings)?;
    Ok(SpeechRegionKeyResult {
        region: region.to_string(),
        speech_key: speech_key.to_string(),
        voice_name,
        output_format,
        rate,
        pitch,
        has_key: true,
    })
}

#[tauri::command]
pub fn get_speech_region_key(
    data: State<'_, AppData>,
    region: String,
) -> Result<SpeechRegionKeyResult, CommandError> {
    let region = region.trim();
    if region.is_empty() {
        return Err(command_error("Region cannot be empty."));
    }
    let connection = Connection::open(&data.db_path)
        .map_err(|error| command_error(format!("Open database failed: {error}")))?;
    ensure_speech_region_key_table(&connection)?;
    let current_settings = data.settings.lock().map_err(lock_error)?.clone();
    let result = connection.query_row(
        "SELECT speech_key, voice_name, output_format, rate, pitch FROM speech_region_keys WHERE region = ?1",
        params![region],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?)),
    );
    let (speech_key, voice_name, output_format, rate, pitch) = match result {
        Ok(value) => value,
        Err(rusqlite::Error::QueryReturnedNoRows) => (
            String::new(),
            current_settings.speech_profile.voice_name,
            current_settings.speech_profile.output_format,
            current_settings.speech_profile.rate,
            current_settings.speech_profile.pitch,
        ),
        Err(error) => {
            return Err(command_error(format!(
                "Load speech region key failed: {error}"
            )))
        }
    };
    Ok(SpeechRegionKeyResult {
        region: region.to_string(),
        has_key: !speech_key.is_empty(),
        speech_key,
        voice_name,
        output_format,
        rate,
        pitch,
    })
}

#[tauri::command]
pub fn get_speech_voices(
    data: State<'_, AppData>,
    locale: Option<String>,
) -> Result<GetSpeechVoicesResult, CommandError> {
    let connection = Connection::open(&data.db_path)
        .map_err(|error| command_error(format!("Open database failed: {error}")))?;
    ensure_speech_voices_table(&connection)?;
    seed_speech_voices(&connection)?;
    let locale = locale.unwrap_or_default().trim().to_string();
    let mut voices = Vec::new();
    if locale.is_empty() {
        let mut statement = connection
            .prepare("SELECT locale, language, voice_type, voice_name, gender, styles, roles, source_url FROM speech_voices ORDER BY locale, voice_name")
            .map_err(|error| command_error(format!("Prepare speech voices query failed: {error}")))?;
        let rows = statement
            .query_map([], speech_voice_from_row)
            .map_err(|error| command_error(format!("Query speech voices failed: {error}")))?;
        for row in rows {
            voices.push(row.map_err(|error| {
                command_error(format!("Read speech voice row failed: {error}"))
            })?);
        }
    } else {
        let mut statement = connection
            .prepare("SELECT locale, language, voice_type, voice_name, gender, styles, roles, source_url FROM speech_voices WHERE locale = ?1 ORDER BY voice_name")
            .map_err(|error| command_error(format!("Prepare speech voices query failed: {error}")))?;
        let rows = statement
            .query_map(params![locale], speech_voice_from_row)
            .map_err(|error| command_error(format!("Query speech voices failed: {error}")))?;
        for row in rows {
            voices.push(row.map_err(|error| {
                command_error(format!("Read speech voice row failed: {error}"))
            })?);
        }
    }
    Ok(GetSpeechVoicesResult {
        source_url: MICROSOFT_TTS_LANGUAGE_SUPPORT_URL.to_string(),
        voices,
    })
}

#[tauri::command]
pub async fn generate_audio(
    data: State<'_, AppData>,
    request: GenerateAudioRequest,
) -> Result<GenerateAudioResult, CommandError> {
    generate_audio_from_text(
        &data,
        request.text,
        request.output_dir,
        request.file_name,
        request.trace_id,
        "manual".to_string(),
        None,
    )
    .await
}

#[tauri::command]
pub async fn generate_material_task_audio(
    data: State<'_, AppData>,
    request: GenerateMaterialTaskAudioRequest,
) -> Result<GenerateAudioResult, CommandError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(command_error("娴犺濮熺捄顖氱窞娑撳秷鍏樻稉铏光敄閵?"));
    }
    let trace_id = build_audio_trace_id(request.trace_id.as_deref());
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "閹垫挸绱戞禒璇插閺佺増宓佹惔鎾炽亼鐠愩儻绱皗{error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    let mut file = match load_material_task_by_path(&connection, path)? {
        Some(file) => file,
        None => {
            let settings = data.settings.lock().map_err(lock_error)?.clone();
            let file = material_file_from_path(path, &settings.material_profile.category_name)?;
            upsert_material_task(&connection, &file)?;
            file
        }
    };
    let material_dir = if let Some(saved) = file
        .material_output_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let saved = PathBuf::from(saved);
        if saved.exists() && material_output_dir_matches(&file, &saved) {
            saved
        } else if let Some(found) = find_existing_material_output_dir(&data, &file)? {
            update_material_task_output_dir(&connection, path, &found.to_string_lossy())?;
            file.material_output_dir = Some(found.to_string_lossy().into_owned());
            found
        } else {
            return Err(command_error("閺堫亝澹橀崚鎷岊嚉娴犺濮熼惃鍕閺夋劕瀵橀惄顔肩秿閿涘矁顕崗鍫㈡晸閹存劗绀岄弶鎰┾偓?"));
        }
    } else if let Some(found) = find_existing_material_output_dir(&data, &file)? {
        update_material_task_output_dir(&connection, path, &found.to_string_lossy())?;
        file.material_output_dir = Some(found.to_string_lossy().into_owned());
        found
    } else {
        return Err(command_error("閺堫亝澹橀崚鎷岊嚉娴犺濮熼惃鍕閺夋劕瀵橀惄顔肩秿閿涘矁顕崗鍫㈡晸閹存劗绀岄弶鎰┾偓?"));
    };
    let narration_file = material_dir.join("narration.txt");
    if !narration_file.exists() {
        return Err(command_error(
            "缁辩姵娼楅崠鍛厬缂傚搫鐨?narration.txt閿涘矁顕柌宥嗘煀閻㈢喐鍨氱槐鐘虫綏閵?",
        ));
    }
    update_material_task_audio_status(
        &connection,
        path,
        "generating",
        10,
        None,
        None,
        None,
        None,
        Some("瀹歌尪顕伴崣鏍ㄦ⒑閻ф枻绱濋崙鍡楊槵閻㈢喐鍨氶棅鎶筋暥"),
    )?;
    let text = fs::read_to_string(&narration_file)
        .map_err(|error| command_error(format!("鐠囪褰?narration.txt 婢惰精瑙﹂敍姝縠{error}")))?;
    update_material_task_audio_status(
        &connection,
        path,
        "generating",
        20,
        None,
        None,
        None,
        None,
        Some("閺冧胶娅у鑼额嚢閸欐牭绱濆锝呮躬鐟欏嫬鍨濋崚鍡橆唽"),
    )?;
    let file_name = sanitize_file_name(&file.name);
    let audio_base_dir = material_dir.clone();
    update_material_task_audio_status(
        &connection,
        path,
        "generating",
        30,
        None,
        None,
        None,
        None,
        Some("濮濓絽婀拠閿嬬湴瀵邦喛钂嬬拠顓㈢叾"),
    )?;
    let audio_progress = AudioTaskProgress {
        db_path: data.db_path.clone(),
        path: path.to_string(),
    };
    let result = match generate_audio_from_text(
        &data,
        text,
        audio_base_dir.to_string_lossy().into_owned(),
        file_name,
        Some(trace_id.clone()),
        narration_file.to_string_lossy().into_owned(),
        Some(audio_progress),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            let _ = update_material_task_audio_status(
                &connection,
                path,
                "failed",
                0,
                None,
                None,
                None,
                None,
                Some(&error.message),
            );
            data.logger.trace_error(
                "audio",
                "task.failed",
                "缁辩姵娼楁禒璇插闂婃娊顣堕悽鐔稿灇婢惰精瑙?",
                &error.message,
                &trace_id,
            );
            return Err(error);
        }
    };
    update_material_task_audio_status(
        &connection,
        path,
        "success",
        100,
        Some(&result.output_dir),
        Some(&result.audio_file),
        result.duration_ms.map(|value| value as i64),
        Some(result.chunks as i64),
        Some("闂婃娊顣跺鑼晸閹?"),
    )?;
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    notify_audio_generation_completed(
        &data,
        &settings,
        &file.name,
        path,
        &result,
        Duration::from_millis(result.elapsed_ms as u64),
        &trace_id,
    )
    .await;
    data.logger.trace_info(
        "audio",
        "task.update",
        "缁辩姵娼楁禒璇插闂婃娊顣堕悩鑸碘偓浣稿嚒閺囧瓨鏌?",
        format!(
            "path={} audio_file={} duration_ms={:?}",
            path, result.audio_file, result.duration_ms
        ),
        &trace_id,
    );
    Ok(result)
}

#[tauri::command]
pub fn generate_book_video_pipeline(
    app: tauri::AppHandle,
    data: State<'_, AppData>,
    request: GenerateBookVideoRequest,
) -> Result<GenerateBookVideoResult, CommandError> {
    let trace_id = build_trace_id(request.trace_id.as_deref());
    let epub_path = request.epub_path.trim();
    if epub_path.is_empty() {
        return Err(command_error("鐠囧嘲鍘涢柅澶嬪 EPUB 閺傚洣娆㈤妴?"));
    }
    let epub = PathBuf::from(epub_path);
    if !epub.exists() {
        return Err(command_error(format!(
            "EPUB file does not exist: {epub_path}"
        )));
    }
    let (pipeline_root, script) = find_video_pipeline(&app)?;
    let python = find_python_command();
    data.logger.trace_info(
        "video",
        "pipeline.spawn",
        "Video pipeline task spawned",
        format!(
            "epub={} script={} python={}",
            epub.to_string_lossy(),
            script.to_string_lossy(),
            python
        ),
        &trace_id,
    );

    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "閹垫挸绱戞禒璇插閺佺増宓佹惔鎾炽亼鐠愩儻绱皗{error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    upsert_material_task(
        &connection,
        &material_file_from_path(epub_path, DEFAULT_MATERIAL_CATEGORY)?,
    )?;
    let _ = connection.execute(
        "UPDATE material_tasks SET status='generating', progress=20, message='Video pipeline queued', video_status='generating', video_progress=20, video_message='Video pipeline queued', updated_at=?2 WHERE path=?1",
        params![epub_path, chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()],
    );

    let db_path = data.db_path.clone();
    let logger = data.logger.clone();
    let epub_path_owned = epub_path.to_string();
    let allow_placeholder_visuals = request.allow_placeholder_visuals.unwrap_or(true);
    let trace_id_for_task = trace_id.clone();
    std::thread::spawn(move || {
        run_book_video_pipeline_background(
            db_path,
            logger,
            trace_id_for_task,
            epub_path_owned,
            epub,
            pipeline_root,
            script,
            python,
            allow_placeholder_visuals,
        );
    });

    Ok(GenerateBookVideoResult {
        material_dir: String::new(),
        pipeline_manifest: String::new(),
        cover: None,
        visual_timeline: None,
        no_subtitle_video: None,
        hard_subtitle_video: None,
        hard_subtitle_manifest: None,
        elapsed_seconds: 0.0,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_book_video_pipeline_background(
    db_path: PathBuf,
    logger: OperationLogger,
    trace_id: String,
    epub_path: String,
    epub: PathBuf,
    pipeline_root: PathBuf,
    script: PathBuf,
    python: String,
    allow_placeholder_visuals: bool,
) {
    let started = Instant::now();
    logger.trace_info(
        "video",
        "pipeline.start",
        "Video pipeline started",
        format!(
            "epub={} script={} python={}",
            epub.to_string_lossy(),
            script.to_string_lossy(),
            python
        ),
        &trace_id,
    );
    update_video_task_after_background(
        &db_path,
        &epub_path,
        "generating",
        45,
        None,
        None,
        None,
        None,
        "Preparing video pipeline",
    );

    let mut command = Command::new(&python);
    let app_material_dir = resolve_task_material_dir_for_video(&db_path, &epub_path);
    let app_video_dir = app_material_dir.clone();
    command
        .current_dir(&pipeline_root)
        .env("PYTHONIOENCODING", "UTF-8")
        .arg(&script)
        .arg("--epub")
        .arg(&epub)
        .arg("--skip-notify")
        .arg("--audio-language")
        .arg("cmn");
    if let Some(video_dir) = app_video_dir.as_ref() {
        command.arg("--output-dir").arg(video_dir);
    }
    if let Some(music_file) = find_background_music_file() {
        command.arg("--background-music").arg(music_file);
    }
    if allow_placeholder_visuals {
        command.arg("--allow-placeholder-visuals");
    }

    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            let message = format!("Failed to launch video pipeline: {error}");
            logger.trace_error(
                "video",
                "pipeline.failed",
                "Video pipeline failed",
                &message,
                &trace_id,
            );
            update_video_task_after_background(
                &db_path, &epub_path, "failed", 0, None, None, None, None, &message,
            );
            return;
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    update_video_task_after_background(
        &db_path,
        &epub_path,
        "generating",
        75,
        None,
        None,
        None,
        None,
        "Video pipeline finished, parsing result",
    );
    if !output.status.success() {
        let detail = format!(
            "stdout:\n{}\n\nstderr:\n{}",
            text_preview(&stdout, 4000),
            text_preview(&stderr, 4000)
        );
        logger.trace_error(
            "video",
            "pipeline.failed",
            "Video pipeline failed",
            &detail,
            &trace_id,
        );
        update_video_task_after_background(
            &db_path,
            &epub_path,
            "failed",
            0,
            None,
            None,
            None,
            None,
            &text_preview(&(stderr + &stdout), 240),
        );
        return;
    }

    let json = match parse_last_json_object(&stdout) {
        Ok(json) => json,
        Err(error) => {
            logger.trace_error(
                "video",
                "pipeline.parse_failed",
                "Video pipeline JSON parse failed",
                &error.message,
                &trace_id,
            );
            update_video_task_after_background(
                &db_path,
                &epub_path,
                "failed",
                0,
                None,
                None,
                None,
                None,
                &error.message,
            );
            return;
        }
    };
    update_video_task_after_background(
        &db_path,
        &epub_path,
        "generating",
        85,
        None,
        None,
        None,
        None,
        "Video pipeline result parsed",
    );
    let material_dir = match json_string(&json, "materialDir") {
        Some(material_dir) => material_dir,
        None => {
            let message = "Video pipeline result is missing materialDir";
            logger.trace_error(
                "video",
                "pipeline.parse_failed",
                "Video pipeline JSON parse failed",
                message,
                &trace_id,
            );
            update_video_task_after_background(
                &db_path, &epub_path, "failed", 0, None, None, None, None, message,
            );
            return;
        }
    };
    let hard_subtitle_video = json_string(&json, "hardSubtitleVideo").unwrap_or_default();
    let video_file = if hard_subtitle_video.trim().is_empty() {
        json_string(&json, "noSubtitleVideo")
    } else {
        Some(hard_subtitle_video.clone())
    };
    let video_file_size = video_file
        .as_deref()
        .and_then(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len().min(i64::MAX as u64) as i64);
    let video_duration_ms = video_file.as_deref().and_then(probe_video_duration_ms);
    let no_subtitle_video = json_string(&json, "noSubtitleVideo");
    let no_subtitle_duration_ms = no_subtitle_video
        .as_deref()
        .and_then(probe_video_duration_ms)
        .or_else(|| json_i64(&json, "noSubtitleVideoDurationMs"));
    let pipeline_video_duration_ms = json_i64(&json, "videoDurationMs");
    let duration_reference_ms = if hard_subtitle_video.trim().is_empty() {
        pipeline_video_duration_ms.or(no_subtitle_duration_ms)
    } else {
        no_subtitle_duration_ms.or(pipeline_video_duration_ms)
    };
    if video_duration_is_abnormal(duration_reference_ms, video_duration_ms) {
        let material_dir_for_db = app_material_dir
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| material_dir.clone());
        let message = format!(
            "Video duration mismatch: expected {}, video {}. Please regenerate the video.",
            format_duration_ms(duration_reference_ms),
            format_duration_ms(video_duration_ms)
        );
        logger.trace_error(
            "video",
            "pipeline.duration_abnormal",
            "Video duration mismatch",
            &message,
            &trace_id,
        );
        update_video_task_after_background(
            &db_path,
            &epub_path,
            "failed",
            0,
            Some(&material_dir_for_db),
            video_file.as_deref(),
            video_duration_ms,
            video_file_size,
            &message,
        );
        return;
    }
    let material_dir_for_db = app_material_dir
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| material_dir.clone());
    update_video_task_after_background(
        &db_path,
        &epub_path,
        "success",
        100,
        Some(&material_dir_for_db),
        video_file.as_deref(),
        video_duration_ms,
        video_file_size,
        "Video generated",
    );
    logger.trace_info(
        "video",
        "pipeline.done",
        "Video pipeline completed",
        format!(
            "elapsed_seconds={:.1} material_dir={} hard_subtitle_video={}",
            started.elapsed().as_secs_f64(),
            material_dir,
            hard_subtitle_video
        ),
        &trace_id,
    );
}

#[allow(clippy::too_many_arguments)]
fn update_video_task_after_background(
    db_path: &Path,
    epub_path: &str,
    status: &str,
    progress: i64,
    material_dir: Option<&str>,
    video_file: Option<&str>,
    video_duration_ms: Option<i64>,
    video_file_size: Option<i64>,
    message: &str,
) {
    if let Ok(connection) = Connection::open(db_path) {
        let _ = connection.execute(
            "UPDATE material_tasks SET status=?2, progress=?3, material_output_dir=COALESCE(?4, material_output_dir), message=?8, video_status=?2, video_progress=?3, video_file=COALESCE(?5, video_file), video_duration_ms=COALESCE(?6, video_duration_ms), video_file_size=COALESCE(?7, video_file_size), video_message=?8, updated_at=?9 WHERE path=?1",
            params![
                epub_path,
                status,
                progress,
                material_dir,
                video_file,
                video_duration_ms,
                video_file_size,
                message,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
            ],
        );
    }
}

fn probe_video_duration_ms(path: &str) -> Option<i64> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let seconds = stdout.trim().parse::<f64>().ok()?;
    Some((seconds * 1000.0).round() as i64)
}

fn resolve_task_material_dir_for_video(db_path: &Path, epub_path: &str) -> Option<PathBuf> {
    let connection = Connection::open(db_path).ok()?;
    let value = connection
        .query_row(
            "SELECT material_output_dir FROM material_tasks WHERE path=?1",
            params![epub_path],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()?;
    let path = PathBuf::from(value);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn video_duration_is_abnormal(
    audio_duration_ms: Option<i64>,
    video_duration_ms: Option<i64>,
) -> bool {
    let Some(audio) = audio_duration_ms else {
        return false;
    };
    let Some(video) = video_duration_ms else {
        return true;
    };
    audio > 60_000 && (audio - video).abs() > 5_000
}

fn format_duration_ms(value: Option<i64>) -> String {
    let Some(ms) = value else {
        return "unknown".to_string();
    };
    let total_seconds = (ms.max(0) + 500) / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes}m{seconds:02}s")
}

fn find_background_music_file() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for dir_name in ["bg", "music"] {
                let music_dir = parent.join(dir_name);
                if let Ok(entries) = fs::read_dir(&music_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path
                            .extension()
                            .and_then(|value| value.to_str())
                            .is_some_and(|value| value.eq_ignore_ascii_case("mp3"))
                        {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

async fn generate_audio_from_text(
    data: &AppData,
    text: String,
    output_dir: String,
    file_name: String,
    trace_id: Option<String>,
    source: String,
    progress_callback: Option<AudioTaskProgress>,
) -> Result<GenerateAudioResult, CommandError> {
    let started = Instant::now();
    let trace_id = build_audio_trace_id(trace_id.as_deref());
    data.logger.trace_info(
        "audio",
        "generate.start",
        "瀵偓婵鏁撻幋鎰⒑閻т粙鐓舵０?",
        format!(
            "trace_id={} text_chars={} output_dir={} file_name={}",
            trace_id,
            text.chars().count(),
            output_dir.trim(),
            file_name.trim()
        ),
        &trace_id,
    );

    let settings = data.settings.lock().map_err(lock_error)?.clone();
    validate_speech_profile(&settings.speech_profile)?;
    let text = text.trim();
    if text.is_empty() {
        data.logger.trace_error(
            "audio",
            "generate.validate",
            "闂婃娊顣堕悽鐔稿灇閺傚洦婀版稉铏光敄",
            "鐠囧嘲鍘涢悽鐔稿灇閹存牜鐭樼拹瀛樻⒑閻ц姤鏋冮張顑锯偓?",
            &trace_id,
        );
        return Err(command_error(
            "鐠囧嘲鍘涢悽鐔稿灇閹存牜鐭樼拹瀛樻⒑閻ц姤鏋冮張顑锯偓?",
        ));
    }
    let chunks = split_speech_text(text, SPEECH_CHUNK_MAX_CHARS);
    if chunks.is_empty() {
        return Err(command_error(
            "濞屸剝婀侀崣顖滄暏娴滃海鏁撻幋鎰扮叾妫版垹娈戦弬鍥ㄦ拱閵?",
        ));
    }
    if chunks.len() > 1 {
        run_ffmpeg_version(&settings.tool_profile.ffmpeg_path)?;
    }

    let base_dir = resolve_audio_base_dir(data, output_dir.trim())?;
    let file_stem = sanitize_file_name(if file_name.trim().is_empty() {
        "narration"
    } else {
        file_name.trim()
    });
    let file_stem = if file_stem.is_empty() {
        "narration".to_string()
    } else {
        file_stem
    };
    let output_dir = base_dir.join(format!(
        "{}_{}",
        chrono::Local::now().format("%Y%m%d_%H%M%S"),
        file_stem
    ));
    let parts_dir = output_dir.join("parts");
    let ssml_dir = output_dir.join("ssml");
    fs::create_dir_all(&output_dir).map_err(|error| {
        command_error(format!(
            "閸掓稑缂撻棅鎶筋暥鏉堟挸鍤惄顔肩秿婢惰精瑙﹂敍姝縠{error}"
        ))
    })?;
    fs::create_dir_all(&parts_dir).map_err(|error| {
        command_error(format!(
            "閸掓稑缂撻棅鎶筋暥閸掑棙顔岄惄顔肩秿婢惰精瑙﹂敍姝縠{error}"
        ))
    })?;
    fs::create_dir_all(&ssml_dir)
        .map_err(|error| command_error(format!("閸掓稑缂?SSML 閻╊喖缍嶆径杈Е閿涙{error}")))?;

    data.logger.debug(
        "audio",
        "generate.plan",
        "闂婃娊顣堕悽鐔稿灇閸掑棙顔岀拋鈥冲灊鐎瑰本鍨?",
        format!(
            "chunks={} chunk_max_chars={} voice={} output_format={} output_dir={}",
            chunks.len(),
            SPEECH_CHUNK_MAX_CHARS,
            settings.speech_profile.voice_name,
            settings.speech_profile.output_format,
            output_dir.to_string_lossy()
        ),
        &trace_id,
    );
    if let Some(callback) = progress_callback.as_ref() {
        callback.update(35, "闂婃娊顣堕崚鍡橆唽鐠佲€冲灊鐎瑰本鍨?");
    }

    let ssml_file = output_dir.join("narration.ssml");
    let manifest_file = output_dir.join("audio_manifest.json");
    let all_ssml = chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            format!(
                "<!-- part {} -->\n{}",
                index + 1,
                build_ssml(chunk, &settings.speech_profile)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    fs::write(&ssml_file, all_ssml)
        .map_err(|error| command_error(format!("閸愭瑥鍙?SSML 閺傚洣娆㈡径杈Е閿涙{error}")))?;

    let mut part_files = Vec::new();
    let mut manifest = build_audio_manifest(
        &trace_id,
        &source,
        text,
        &chunks,
        &output_dir,
        &parts_dir,
        &ssml_dir,
        &file_stem,
    );
    write_audio_manifest(&manifest_file, &manifest)?;
    for (index, chunk) in chunks.iter().enumerate() {
        let part_file = parts_dir.join(format!("part_{:03}.mp3", index + 1));
        let part_ssml_file = ssml_dir.join(format!("part_{:03}.ssml", index + 1));
        let ssml = build_ssml(chunk, &settings.speech_profile);
        fs::write(&part_ssml_file, &ssml).map_err(|error| {
            command_error(format!("閸愭瑥鍙嗛崚鍡橆唽 SSML 婢惰精瑙﹂敍姝縠{error}"))
        })?;
        data.logger.trace_info(
            "audio",
            "speech.request",
            format!(
                "瀵偓婵顕Ч鍌氫簳鏉烆垵顕㈤棅鍐插瀻濞?{}/{}",
                index + 1,
                chunks.len()
            ),
            format!(
                "chunk_chars={} output={}",
                chunk.chars().count(),
                part_file.to_string_lossy()
            ),
            &trace_id,
        );
        let part_started = Instant::now();
        synthesize_speech_to_file(&settings.speech_profile, &ssml, &part_file)
            .await
            .map_err(|error| {
                data.logger.trace_error(
                    "audio",
                    "speech.request.failed",
                    format!("瀵邦喛钂嬬拠顓㈢叾閸掑棙顔?{} 閻㈢喐鍨氭径杈Е", index + 1),
                    &error.message,
                    &trace_id,
                );
                if let Some(part) = manifest.parts.get_mut(index) {
                    part.status = "failed".to_string();
                    part.error = Some(error.message.clone());
                    part.elapsed_ms = Some(part_started.elapsed().as_millis());
                }
                manifest.status = "failed".to_string();
                manifest.updated_at = chrono::Local::now().to_rfc3339();
                let _ = write_audio_manifest(&manifest_file, &manifest);
                error
            })?;
        if let Some(part) = manifest.parts.get_mut(index) {
            part.status = "success".to_string();
            part.error = None;
            part.elapsed_ms = Some(part_started.elapsed().as_millis());
        }
        manifest.updated_at = chrono::Local::now().to_rfc3339();
        write_audio_manifest(&manifest_file, &manifest)?;
        if let Some(callback) = progress_callback.as_ref() {
            let progress = 40 + (((index + 1) as i64 * 45) / chunks.len().max(1) as i64);
            callback.update(
                progress.min(85),
                &format!(
                    "瀵邦喛钂嬬拠顓㈢叾閸掑棙顔?{}/{} 瀹告彃鐣幋?",
                    index + 1,
                    chunks.len()
                ),
            );
        }
        data.logger.trace_info(
            "audio",
            "speech.response",
            format!(
                "瀵邦喛钂嬬拠顓㈢叾閸掑棙顔?{}/{} 閻㈢喐鍨氶幋鎰",
                index + 1,
                chunks.len()
            ),
            format!(
                "elapsed_ms={} file={}",
                part_started.elapsed().as_millis(),
                part_file.to_string_lossy()
            ),
            &trace_id,
        );
        part_files.push(part_file);
    }

    let final_audio = output_dir.join(format!("{file_stem}.mp3"));
    if part_files.len() == 1 {
        fs::copy(&part_files[0], &final_audio).map_err(|error| {
            command_error(format!("婢跺秴鍩楅棅鎶筋暥閺傚洣娆㈡径杈Е閿涙{error}"))
        })?;
    } else {
        if let Some(callback) = progress_callback.as_ref() {
            callback.update(88, "濮濓絽婀幏鍏煎复闂婃娊顣堕崚鍡橆唽");
        }
        concat_audio_parts(
            &settings.tool_profile.ffmpeg_path,
            &output_dir,
            &part_files,
            &final_audio,
            &data.logger,
            &trace_id,
        )?;
    }
    if let Some(callback) = progress_callback.as_ref() {
        callback.update(92, "濮濓絽婀幒銏＄ゴ闂婃娊顣堕弮鍫曟毐");
    }
    let duration_ms = probe_audio_duration_ms(
        &settings.tool_profile.ffmpeg_path,
        &final_audio,
        &data.logger,
        &trace_id,
    )
    .ok();
    manifest.status = "success".to_string();
    manifest.duration_ms = duration_ms;
    manifest.updated_at = chrono::Local::now().to_rfc3339();
    write_audio_manifest(&manifest_file, &manifest)?;

    data.logger.trace_info(
        "audio",
        "generate.done",
        "閺冧胶娅ч棅鎶筋暥閻㈢喐鍨氱€瑰本鍨?",
        format!(
            "elapsed_ms={} chars={} chunks={} audio_file={} ssml_file={}",
            started.elapsed().as_millis(),
            text.chars().count(),
            part_files.len(),
            final_audio.to_string_lossy(),
            ssml_file.to_string_lossy()
        ),
        &trace_id,
    );

    let result = GenerateAudioResult {
        output_dir: output_dir.to_string_lossy().into_owned(),
        audio_file: final_audio.to_string_lossy().into_owned(),
        ssml_file: ssml_file.to_string_lossy().into_owned(),
        manifest_file: manifest_file.to_string_lossy().into_owned(),
        part_files: part_files
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        chars: text.chars().count(),
        chunks: chunks.len(),
        duration_ms,
        elapsed_ms: started.elapsed().as_millis(),
    };
    if source == "manual" {
        notify_audio_generation_completed(
            data,
            &settings,
            &file_stem,
            &source,
            &result,
            started.elapsed(),
            &trace_id,
        )
        .await;
    }
    Ok(result)
}

#[tauri::command]
pub fn get_operation_logs(
    data: State<'_, AppData>,
    request: GetOperationLogsRequest,
) -> Result<GetOperationLogsResult, CommandError> {
    let limit = request.limit.clamp(1, 1000);
    let trace_id = request
        .trace_id
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "鐠囪褰囬幙宥勭稊閺冦儱绻旈弫鐗堝祦鎼存挸銇戠拹銉窗{error}"
        ))
    })?;
    let entries = if let Some(trace_id) = trace_id {
        query_operation_logs_by_trace(&connection, limit, trace_id)?
    } else {
        query_operation_logs_since(&connection, limit, &data.app_started_at)?
    };
    Ok(GetOperationLogsResult { entries })
}

fn query_operation_logs_since(
    connection: &Connection,
    limit: usize,
    since: &str,
) -> Result<Vec<OperationLogEntry>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, created_at, level, module, action, message, detail, trace_id
            FROM operate_log
            WHERE created_at >= ?1
            ORDER BY id DESC
            LIMIT ?2
            "#,
        )
        .map_err(|error| command_error(format!("Prepare operation log query failed: {error}")))?;
    let rows = statement
        .query_map((since, limit as i64), operation_log_from_row)
        .map_err(|error| command_error(format!("Query operation logs failed: {error}")))?;
    collect_operation_logs(rows)
}

fn query_operation_logs_by_trace(
    connection: &Connection,
    limit: usize,
    trace_id: &str,
) -> Result<Vec<OperationLogEntry>, CommandError> {
    let trace_prefix = format!("{}-%", trace_id.trim());
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, created_at, level, module, action, message, detail, trace_id
            FROM operate_log
            WHERE trace_id = ?1 OR trace_id LIKE ?2
            ORDER BY id DESC
            LIMIT ?3
            "#,
        )
        .map_err(|error| {
            command_error(format!("Prepare operation log trace query failed: {error}"))
        })?;
    let rows = statement
        .query_map(
            (trace_id, trace_prefix, limit as i64),
            operation_log_from_row,
        )
        .map_err(|error| command_error(format!("Query operation logs by trace failed: {error}")))?;
    collect_operation_logs(rows)
}

fn collect_operation_logs<F>(
    rows: rusqlite::MappedRows<'_, F>,
) -> Result<Vec<OperationLogEntry>, CommandError>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<OperationLogEntry>,
{
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|error| {
            command_error(format!(
                "鐠囪褰囬幙宥勭稊閺冦儱绻旂悰灞姐亼鐠愩儻绱皗{error}"
            ))
        })?);
    }
    entries.reverse();
    Ok(entries)
}

fn operation_log_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OperationLogEntry> {
    Ok(OperationLogEntry {
        id: row.get(0)?,
        created_at: row.get(1)?,
        level: row.get(2)?,
        module: row.get(3)?,
        action: row.get(4)?,
        message: row.get(5)?,
        detail: row.get(6)?,
        trace_id: row.get(7)?,
    })
}

async fn call_ai(
    settings: &AppSettings,
    messages: Vec<ChatMessage>,
) -> Result<String, CommandError> {
    let profile = &settings.ai_profile;
    if profile.api_key.trim().is_empty() {
        return Err(command_error("鐠囧嘲鍘涙繅顐㈠晸 AI API Key閵?"));
    }
    if profile.base_url.trim().is_empty() {
        return Err(command_error("鐠囧嘲鍘涙繅顐㈠晸 AI Base URL閵?"));
    }
    if profile.model.trim().is_empty() {
        return Err(command_error("鐠囧嘲鍘涙繅顐㈠晸 AI 濡€崇€烽崥宥囆為妴?"));
    }

    let base = profile.base_url.trim().trim_end_matches('/');
    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(AI_REQUEST_TIMEOUT_SECONDS))
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| command_error(format!("Build AI HTTP client failed: {error}")))?;

    let request_body = ChatCompletionRequest {
        model: profile.model.clone(),
        messages,
        stream: true,
    };
    let mut last_error = None;
    let response = 'attempts: loop {
        for attempt in 1..=AI_REQUEST_MAX_ATTEMPTS {
            let result = client
                .post(&url)
                .bearer_auth(profile.api_key.trim())
                .header(
                    reqwest::header::USER_AGENT,
                    "A-Book-in-30-Minutes/0.1 OpenAI-Compatible-Client",
                )
                .header(
                    reqwest::header::ACCEPT,
                    "text/event-stream, application/json",
                )
                .json(&request_body)
                .send()
                .await;

            match result {
                Ok(response) if response.status().is_success() => break 'attempts response,
                Ok(response) => {
                    let status = response.status();
                    let detail = response.text().await.unwrap_or_default();
                    let message = if detail.trim().is_empty() {
                        format!("AI 閺堝秴濮熸潻鏂挎礀闁挎瑨顕ら敍娆籘TP {status}")
                    } else {
                        format!(
                            "AI 閺堝秴濮熸潻鏂挎礀闁挎瑨顕ら敍娆籘TP {status} {}",
                            text_preview(&detail, 500)
                        )
                    };
                    if !should_retry_ai_status(status) || attempt == AI_REQUEST_MAX_ATTEMPTS {
                        return Err(command_error(message));
                    }
                    last_error = Some(message);
                }
                Err(error) => {
                    let message = format!("Failed to launch video pipeline: {error}");

                    if attempt == AI_REQUEST_MAX_ATTEMPTS {
                        return Err(command_error(message));
                    }
                    last_error = Some(message);
                }
            }

            let delay_seconds = 20 * attempt as u64;
            sleep_before_ai_retry(delay_seconds).await;
        }
        return Err(command_error(
            last_error.unwrap_or_else(|| "AI request failed.".to_string()),
        ));
    };

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let body = response
        .text()
        .await
        .map_err(|error| command_error(format!("AI 閸濆秴绨茬拠璇插絿婢惰精瑙﹂敍姝縠{error}")))?;
    let content = if content_type.contains("text/event-stream") || body.contains("data:") {
        parse_streaming_chat_content(&body)?
    } else {
        parse_blocking_chat_content(&body)?
    };

    if content.trim().is_empty() {
        Err(command_error("AI 閺堝秴濮熷▽鈩冩箒鏉╂柨娲栭崘鍛啇閵?"))
    } else {
        Ok(content)
    }
}

async fn sleep_before_ai_retry(seconds: u64) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_secs(seconds));
    })
    .await;
}

fn should_retry_ai_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::FORBIDDEN
        || status.is_server_error()
}

#[derive(Debug, Deserialize)]
struct StreamingChatChunk {
    choices: Vec<StreamingChatChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamingChatChoice {
    delta: Option<StreamingChatDelta>,
}

#[derive(Debug, Deserialize)]
struct StreamingChatDelta {
    content: Option<String>,
}

fn parse_streaming_chat_content(body: &str) -> Result<String, CommandError> {
    let mut content = String::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("data:") {
            continue;
        }
        let data = trimmed.trim_start_matches("data:").trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let chunk = serde_json::from_str::<StreamingChatChunk>(data).map_err(|error| {
            command_error(format!("AI 濞翠礁绱￠崫宥呯安鐟欙絾鐎芥径杈Е閿涙{error}"))
        })?;
        for choice in chunk.choices {
            if let Some(delta) = choice.delta {
                if let Some(part) = delta.content {
                    content.push_str(&part);
                }
            }
        }
    }
    Ok(content)
}

fn parse_blocking_chat_content(body: &str) -> Result<String, CommandError> {
    let response = serde_json::from_str::<ChatCompletionResponse>(body)
        .map_err(|error| command_error(format!("AI 閸濆秴绨茬憴锝嗙€芥径杈Е閿涙{error}")))?;
    response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| command_error("AI 閺堝秴濮熷▽鈩冩箒鏉╂柨娲栭崘鍛啇閵?"))
}
async fn call_feishu(settings: &AppSettings, text: &str) -> Result<FeishuSendResult, CommandError> {
    let webhook_url = settings.feishu_profile.webhook_url.trim();
    if webhook_url.is_empty() {
        return Err(command_error("Feishu webhook URL is empty."));
    }
    let content = if settings.feishu_profile.title.trim().is_empty() {
        text.to_string()
    } else {
        format!("{}\n\n{}", settings.feishu_profile.title.trim(), text)
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(FEISHU_REQUEST_TIMEOUT_SECONDS))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| command_error(format!("Build Feishu HTTP client failed: {error}")))?;
    let response = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("User-Agent", "ABookIn30Minutes/0.1")
        .json(&serde_json::json!({
            "msg_type": "text",
            "content": { "text": content }
        }))
        .send()
        .await
        .map_err(|error| command_error(format!("Send Feishu request failed: {error}")))?
        .error_for_status()
        .map_err(|error| command_error(format!("Feishu request returned error: {error}")))?;
    let body = response
        .json::<FeishuWebhookResponse>()
        .await
        .map_err(|error| command_error(format!("Parse Feishu response failed: {error}")))?;
    let code = body.code.unwrap_or(-1);
    if code != 0 {
        let msg = body.msg.unwrap_or_else(|| "unknown".to_string());
        return Err(command_error(format!(
            "Feishu webhook failed: code={code} msg={msg}"
        )));
    }
    Ok(FeishuSendResult {
        ok: true,
        message: "Feishu message sent.".to_string(),
    })
}

async fn notify_generation_completed(
    data: &AppData,
    settings: &AppSettings,
    materials: &BookMaterials,
    elapsed: Duration,
    source_path: &str,
    trace_id: &str,
) {
    if settings.feishu_profile.webhook_url.trim().is_empty() {
        data.logger.debug(
            "feishu",
            "materials_notify.skip",
            "Feishu webhook is empty",
            "",
            trace_id,
        );
        return;
    }
    let message = format!(
        "Materials generated\nBook: {}\nTitle: {}\nChars: {}\nSubtitles: {}\nModel: {}\nElapsed: {}\nSource: {}",
        materials.overview.title,
        materials.video_title,
        count_han_chars(&materials.narration),
        materials.subtitles.len(),
        materials.model,
        format_human_duration(elapsed),
        source_path,
    );
    match call_feishu(settings, &message).await {
        Ok(_) => data.logger.trace_info(
            "feishu",
            "materials_notify.done",
            "Materials Feishu notification sent",
            format!("source={source_path}"),
            trace_id,
        ),
        Err(error) => data.logger.trace_error(
            "feishu",
            "materials_notify.failed",
            "Materials Feishu notification failed",
            &error.message,
            trace_id,
        ),
    }
}

async fn notify_audio_generation_completed(
    data: &AppData,
    settings: &AppSettings,
    title: &str,
    source_path: &str,
    result: &GenerateAudioResult,
    elapsed: Duration,
    trace_id: &str,
) {
    if settings.feishu_profile.webhook_url.trim().is_empty() {
        data.logger.debug(
            "feishu",
            "audio_notify.skip",
            "Feishu webhook is empty",
            "",
            trace_id,
        );
        return;
    }
    let duration = result
        .duration_ms
        .map(|value| format_human_duration(Duration::from_millis(value)))
        .unwrap_or_else(|| "unknown".to_string());
    let message = format!(
        "Audio generated\nTitle: {}\nChars: {}\nChunks: {}\nDuration: {}\nElapsed: {}\nAudio: {}\nOutput: {}\nSource: {}",
        title,
        result.chars,
        result.chunks,
        duration,
        format_human_duration(elapsed),
        result.audio_file,
        result.output_dir,
        source_path,
    );
    match call_feishu(settings, &message).await {
        Ok(_) => data.logger.trace_info(
            "feishu",
            "audio_notify.done",
            "Audio Feishu notification sent",
            format!("source={source_path} audio={}", result.audio_file),
            trace_id,
        ),
        Err(error) => data.logger.trace_error(
            "feishu",
            "audio_notify.failed",
            "Audio Feishu notification failed",
            &error.message,
            trace_id,
        ),
    }
}

fn format_human_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs_f64();
    let hours = (total_seconds / 3600.0).floor() as u64;
    let minutes = ((total_seconds - hours as f64 * 3600.0) / 60.0).floor() as u64;
    let seconds = total_seconds - hours as f64 * 3600.0 - minutes as f64 * 60.0;
    if hours > 0 {
        format!("{hours}h{minutes}m{seconds:.1}s")
    } else if minutes > 0 {
        format!("{minutes}m{seconds:.1}s")
    } else {
        format!("{seconds:.1}s")
    }
}

fn validate_speech_profile(profile: &SpeechProfile) -> Result<(), CommandError> {
    if profile.speech_key.trim().is_empty() {
        return Err(command_error("Speech key cannot be empty."));
    }
    if profile.region.trim().is_empty() {
        return Err(command_error("Speech region cannot be empty."));
    }
    if profile.voice_name.trim().is_empty() {
        return Err(command_error("Speech voice cannot be empty."));
    }
    if profile.output_format.trim().is_empty() {
        return Err(command_error("Speech output format cannot be empty."));
    }
    Ok(())
}

async fn synthesize_speech_to_file(
    profile: &SpeechProfile,
    ssml: &str,
    output_file: &Path,
) -> Result<(), CommandError> {
    let region = profile.region.trim();
    let url = format!("https://{region}.tts.speech.microsoft.com/cognitiveservices/v1");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(SPEECH_REQUEST_TIMEOUT_SECONDS))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| command_error(format!("Build speech HTTP client failed: {error}")))?;
    let response = client
        .post(url)
        .header("Ocp-Apim-Subscription-Key", profile.speech_key.trim())
        .header("Content-Type", "application/ssml+xml; charset=utf-8")
        .header("X-Microsoft-OutputFormat", profile.output_format.trim())
        .header("User-Agent", "ABookIn30Minutes/0.1")
        .body(ssml.to_string())
        .send()
        .await
        .map_err(|error| command_error(format!("Speech request failed: {error}")))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| command_error(format!("Read speech response failed: {error}")))?;
    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        return Err(command_error(format!(
            "Speech service returned HTTP {status}: {}",
            text_preview(&body, 300)
        )));
    }
    if bytes.is_empty() {
        return Err(command_error("Speech service returned empty audio."));
    }
    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| command_error(format!("Create speech output dir failed: {error}")))?;
    }
    fs::write(output_file, bytes)
        .map_err(|error| command_error(format!("Write speech audio failed: {error}")))?;
    Ok(())
}

fn build_ssml(text: &str, profile: &SpeechProfile) -> String {
    let rate = profile.rate.trim();
    let pitch = profile.pitch.trim();
    let locale = profile.locale.trim();
    let locale = if locale.is_empty() { "zh-CN" } else { locale };
    let prosody_open = if rate.is_empty() && pitch.is_empty() {
        "<prosody>".to_string()
    } else {
        format!(
            "<prosody rate=\"{}\" pitch=\"{}\">",
            escape_xml_attr(if rate.is_empty() { "+0%" } else { rate }),
            escape_xml_attr(if pitch.is_empty() { "+0Hz" } else { pitch })
        )
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<speak version=\"1.0\" xml:lang=\"{}\" xmlns=\"http://www.w3.org/2001/10/synthesis\" xmlns:mstts=\"https://www.w3.org/2001/mstts\">\n  <voice name=\"{}\">\n    {}{}{}\n  </voice>\n</speak>",
        escape_xml_attr(locale),
        escape_xml_attr(profile.voice_name.trim()),
        prosody_open,
        escape_xml_text(text),
        "</prosody>"
    )
}

fn split_speech_text(text: &str, max_chars: usize) -> Vec<String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_sentences = 0usize;
    let mut current_estimated_ms = 0u64;
    for sentence in split_sentences(&normalized) {
        let sentence_chars = sentence.chars().count();
        if sentence_chars > max_chars {
            if !current.trim().is_empty() {
                chunks.push(current.trim().to_string());
                current.clear();
                current_sentences = 0;
                current_estimated_ms = 0;
            }
            chunks.extend(split_by_char_limit(&sentence, max_chars));
            continue;
        }
        let sentence_estimated_ms = estimate_speech_duration_ms(&sentence);
        let next_len = current.chars().count() + sentence_chars + 1;
        let next_sentences = current_sentences + 1;
        let next_estimated_ms = current_estimated_ms + sentence_estimated_ms;
        if (next_len > max_chars
            || next_sentences > SPEECH_CHUNK_MAX_SENTENCES
            || next_estimated_ms > SPEECH_CHUNK_MAX_ESTIMATED_MS)
            && !current.trim().is_empty()
        {
            chunks.push(current.trim().to_string());
            current.clear();
            current_sentences = 0;
            current_estimated_ms = 0;
        }
        current.push_str(&sentence);
        current.push('\n');
        current_sentences += 1;
        current_estimated_ms += sentence_estimated_ms;
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks
}

fn estimate_speech_duration_ms(text: &str) -> u64 {
    let chars = text.chars().filter(|char| !char.is_whitespace()).count() as u64;
    let punctuation_pauses = text
        .chars()
        .filter(|char| {
            matches!(
                char,
                '?' | '?' | '?' | '.' | '!' | '?' | ';' | '?' | ',' | '?'
            )
        })
        .count() as u64;
    (chars * 260 + punctuation_pauses * 180).max(800)
}

fn build_audio_manifest(
    trace_id: &str,
    source: &str,
    text: &str,
    chunks: &[String],
    output_dir: &Path,
    parts_dir: &Path,
    ssml_dir: &Path,
    file_stem: &str,
) -> AudioManifest {
    let now = chrono::Local::now().to_rfc3339();
    let mut sentence_start = 1usize;
    let parts = chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            let sentence_count = split_sentences(chunk).len().max(1);
            let sentence_end = sentence_start + sentence_count - 1;
            let part = AudioManifestPart {
                index: index + 1,
                sentence_start,
                sentence_end,
                chars: chunk.chars().count(),
                estimated_duration_ms: estimate_speech_duration_ms(chunk),
                status: "pending".to_string(),
                text_file: String::new(),
                ssml_file: ssml_dir
                    .join(format!("part_{:03}.ssml", index + 1))
                    .to_string_lossy()
                    .into_owned(),
                audio_file: parts_dir
                    .join(format!("part_{:03}.mp3", index + 1))
                    .to_string_lossy()
                    .into_owned(),
                error: None,
                elapsed_ms: None,
            };
            sentence_start = sentence_end + 1;
            part
        })
        .collect::<Vec<_>>();
    AudioManifest {
        trace_id: trace_id.to_string(),
        source: source.to_string(),
        status: "generating".to_string(),
        chars: text.chars().count(),
        chunks: chunks.len(),
        final_audio_file: output_dir
            .join(format!("{file_stem}.mp3"))
            .to_string_lossy()
            .into_owned(),
        duration_ms: None,
        created_at: now.clone(),
        updated_at: now,
        parts,
    }
}

fn write_audio_manifest(path: &Path, manifest: &AudioManifest) -> Result<(), CommandError> {
    let json = serde_json::to_string_pretty(manifest).map_err(|error| {
        command_error(format!(
            "鎼村繐鍨崠鏍叾妫?manifest 婢惰精瑙﹂敍姝縠{error}"
        ))
    })?;
    fs::write(path, json).map_err(|error| {
        command_error(format!(
            "閸愭瑥鍙嗛棅鎶筋暥 manifest 婢惰精瑙﹂敍姝縠{error}"
        ))
    })?;
    Ok(())
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut current = String::new();
    for char in text.chars() {
        current.push(char);
        if matches!(char, '?' | '?' | '?' | '.' | '!' | '?' | '\n') {
            let value = current.trim();
            if !value.is_empty() {
                output.push(value.to_string());
            }
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        output.push(current.trim().to_string());
    }
    output
}

fn split_by_char_limit(text: &str, max_chars: usize) -> Vec<String> {
    let chars = text.chars().collect::<Vec<_>>();
    chars
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect::<String>())
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn run_ffmpeg_version(ffmpeg_path: &str) -> Result<String, CommandError> {
    let path = ffmpeg_path.trim();
    if path.is_empty() {
        return Err(command_error(
            "鐠囧嘲鍘涢崷銊╁帳缂冾喕鑵戞繅顐㈠晸 ffmpeg.exe 鐠侯垰绶為妴鍌炴毐缁嬪灝鍨庡▓鍨閹恒儵娓剁憰浣峰▏閻?ffmpeg閵?",
        ));
    }
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err(command_error(
            "ffmpeg.exe 鐠侯垰绶炴稉宥呯摠閸︻煉绱濈拠閿嬵梾閺屻儵鍘ょ純顔衡偓?",
        ));
    }
    let output = Command::new(&path_buf)
        .arg("-version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| command_error(format!("閹笛嗩攽 ffmpeg.exe 婢惰精瑙﹂敍姝縠{error}")))?;
    if !output.status.success() {
        return Err(command_error(format!(
            "ffmpeg exited with status {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .next()
        .unwrap_or("ffmpeg 閸欘垳鏁?")
        .to_string())
}

fn concat_audio_parts(
    ffmpeg_path: &str,
    output_dir: &Path,
    part_files: &[PathBuf],
    final_audio: &Path,
    logger: &OperationLogger,
    trace_id: &str,
) -> Result<(), CommandError> {
    let concat_file = output_dir.join("concat.txt");
    let content = part_files
        .iter()
        .map(|path| {
            let concat_path = path.strip_prefix(output_dir).unwrap_or(path);
            format!(
                "file '{}'",
                concat_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .replace('\'', "'\\''")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&concat_file, content).map_err(|error| {
        command_error(format!(
            "閸愭瑥鍙?ffmpeg 閹峰吋甯村〒鍛礋婢惰精瑙﹂敍姝縠{error}"
        ))
    })?;
    logger.trace_info(
        "audio",
        "ffmpeg.concat",
        "瀵偓婵濞囬悽?ffmpeg 閹峰吋甯撮崚鍡橆唽闂婃娊顣?",
        format!(
            "parts={} concat_file={} output={}",
            part_files.len(),
            concat_file.to_string_lossy(),
            final_audio.to_string_lossy()
        ),
        trace_id,
    );
    let output = Command::new(ffmpeg_path.trim())
        .args(["-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&concat_file)
        .args([
            "-ar",
            "48000",
            "-ac",
            "2",
            "-codec:a",
            "libmp3lame",
            "-b:a",
            "192k",
            "-write_xing",
            "1",
        ])
        .arg(final_audio)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| command_error(format!("閹笛嗩攽 ffmpeg 閹峰吋甯存径杈Е閿涙{error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        logger.trace_error(
            "audio",
            "ffmpeg.concat.failed",
            "ffmpeg concat failed",
            text_preview(&stderr, 600),
            trace_id,
        );
        return Err(command_error(format!(
            "ffmpeg concat failed: {}",
            text_preview(&stderr, 300)
        )));
    }
    logger.trace_info(
        "audio",
        "ffmpeg.concat.done",
        "ffmpeg 閹峰吋甯撮崚鍡橆唽闂婃娊顣堕幋鎰",
        final_audio.to_string_lossy(),
        trace_id,
    );
    Ok(())
}

fn probe_audio_duration_ms(
    ffmpeg_path: &str,
    audio_file: &Path,
    logger: &OperationLogger,
    trace_id: &str,
) -> Result<u64, CommandError> {
    let path = ffmpeg_path.trim();
    if path.is_empty() {
        return Err(command_error(
            "閺堫亪鍘ょ純?ffmpeg.exe閿涘本妫ゅ▔鏇熷赴濞村鐓舵０鎴炴闂€瑁も偓?",
        ));
    }
    let output = Command::new(path)
        .args(["-hide_banner", "-i"])
        .arg(audio_file)
        .arg("-f")
        .arg("null")
        .arg("-")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            command_error(format!(
                "閹笛嗩攽 ffmpeg 閹恒垺绁撮棅鎶筋暥閺冨爼鏆辨径杈Е閿涙{error}"
            ))
        })?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let duration = parse_ffmpeg_duration_ms(&stderr).ok_or_else(|| {
        command_error("閺堫亣鍏樻禒?ffmpeg 鏉堟挸鍤憴锝嗙€介棅鎶筋暥閺冨爼鏆遍妴?")
    })?;
    logger.trace_info(
        "audio",
        "duration.probe",
        "闂婃娊顣堕弮鍫曟毐閹恒垺绁寸€瑰本鍨?",
        format!(
            "audio_file={} duration_ms={duration}",
            audio_file.to_string_lossy()
        ),
        trace_id,
    );
    Ok(duration)
}

fn parse_ffmpeg_duration_ms(output: &str) -> Option<u64> {
    let regex = Regex::new(r"Duration:\s*(\d{2}):(\d{2}):(\d{2})\.(\d{2})").ok()?;
    let captures = regex.captures(output)?;
    let hours = captures.get(1)?.as_str().parse::<u64>().ok()?;
    let minutes = captures.get(2)?.as_str().parse::<u64>().ok()?;
    let seconds = captures.get(3)?.as_str().parse::<u64>().ok()?;
    let centiseconds = captures.get(4)?.as_str().parse::<u64>().ok()?;
    Some(((hours * 3600 + minutes * 60 + seconds) * 1000) + centiseconds * 10)
}

fn resolve_audio_base_dir(data: &AppData, output_dir: &str) -> Result<PathBuf, CommandError> {
    if output_dir.is_empty() {
        let parent = data.settings_path.parent().ok_or_else(|| {
            command_error("閺冪姵纭剁€规矮缍呮妯款吇闂婃娊顣舵潏鎾冲毉閻╊喖缍嶉妴?")
        })?;
        return Ok(parent.join("audio_exports"));
    }
    Ok(PathBuf::from(output_dir))
}

fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_attr(value: &str) -> String {
    escape_xml_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn speech_profile_debug_detail(settings: &AppSettings) -> String {
    let key_length = settings.speech_profile.speech_key.trim().chars().count();
    format!(
        "provider={} locale={} region={} voice={} output_format={} rate={} pitch={} speech_key_present={} speech_key_length={} ffmpeg_configured={}",
        settings.speech_profile.provider,
        settings.speech_profile.locale,
        settings.speech_profile.region,
        settings.speech_profile.voice_name,
        settings.speech_profile.output_format,
        settings.speech_profile.rate,
        settings.speech_profile.pitch,
        key_length > 0,
        key_length,
        !settings.tool_profile.ffmpeg_path.trim().is_empty()
    )
}

fn build_audio_trace_id(provided: Option<&str>) -> String {
    let provided = provided.unwrap_or("").trim();
    if !provided.is_empty() {
        return sanitize_trace_id(provided);
    }
    format!("audio-{}", chrono::Local::now().format("%Y%m%d-%H%M%S-%3f"))
}

fn build_audio_data_url(path: &Path) -> Result<String, CommandError> {
    let bytes = fs::read(path)
        .map_err(|error| command_error(format!("Read audio file failed: {error}")))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:audio/mpeg;base64,{encoded}"))
}

fn init_app_tables(db_path: &Path, logger: &OperationLogger) {
    match Connection::open(db_path) {
        Ok(connection) => {
            if let Err(error) = ensure_material_tasks_table(&connection) {
                logger.error(
                    "materials",
                    "tasks.init",
                    "Initialize material tasks table failed",
                    error.message,
                );
            }
            if let Err(error) = ensure_speech_region_key_table(&connection) {
                logger.error(
                    "settings",
                    "speech_region_keys.init",
                    "Initialize speech region key table failed",
                    error.message,
                );
            }
            if let Err(error) = ensure_speech_voices_table(&connection)
                .and_then(|_| seed_speech_voices(&connection))
            {
                logger.error(
                    "settings",
                    "speech_voices.init",
                    "Initialize speech voices table failed",
                    error.message,
                );
            }
        }
        Err(error) => logger.error(
            "settings",
            "app_tables.init",
            "Open app database failed",
            error.to_string(),
        ),
    }
}

fn ensure_material_tasks_table(connection: &Connection) -> Result<(), CommandError> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS material_tasks (
              path TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              extension TEXT NOT NULL,
              size INTEGER NOT NULL,
              category TEXT NOT NULL DEFAULT '閸楀﹤鐨弮璺烘儔鐎瑰奔绔撮張顑垮姛',
              status TEXT NOT NULL DEFAULT 'pending',
              progress INTEGER NOT NULL DEFAULT 0,
              narration_chars INTEGER,
              material_output_dir TEXT,
              message TEXT NOT NULL DEFAULT '',
              audio_status TEXT NOT NULL DEFAULT 'pending',
              audio_progress INTEGER NOT NULL DEFAULT 0,
              audio_output_dir TEXT,
              audio_file TEXT,
              audio_duration_ms INTEGER,
              audio_chunks INTEGER,
              audio_message TEXT NOT NULL DEFAULT '',
              video_status TEXT NOT NULL DEFAULT 'pending',
              video_progress INTEGER NOT NULL DEFAULT 0,
              video_file TEXT,
              video_duration_ms INTEGER,
              video_file_size INTEGER,
              video_message TEXT NOT NULL DEFAULT '',
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_material_tasks_category ON material_tasks(category);
            CREATE INDEX IF NOT EXISTS idx_material_tasks_updated_at ON material_tasks(updated_at);
            "#,
        )
        .map_err(|error| command_error(format!("Ensure material tasks table failed: {error}")))?;
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN category TEXT NOT NULL DEFAULT '閸楀﹤鐨弮璺烘儔鐎瑰奔绔撮張顑垮姛'",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN status TEXT NOT NULL DEFAULT 'pending'",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN progress INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN narration_chars INTEGER",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN material_output_dir TEXT",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN message TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN audio_status TEXT NOT NULL DEFAULT 'pending'",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN audio_progress INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN audio_output_dir TEXT",
        [],
    );
    let _ = connection.execute("ALTER TABLE material_tasks ADD COLUMN audio_file TEXT", []);
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN audio_duration_ms INTEGER",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN audio_chunks INTEGER",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN audio_message TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN video_status TEXT NOT NULL DEFAULT 'pending'",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN video_progress INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = connection.execute("ALTER TABLE material_tasks ADD COLUMN video_file TEXT", []);
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN video_duration_ms INTEGER",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN video_file_size INTEGER",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN video_message TEXT NOT NULL DEFAULT ''",
        [],
    );
    Ok(())
}

fn upsert_material_task(connection: &Connection, file: &MaterialFile) -> Result<(), CommandError> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    connection
        .execute(
            r#"
            INSERT INTO material_tasks
              (path, name, extension, size, category, status, progress, narration_chars, material_output_dir, message, audio_status, audio_progress, audio_output_dir, audio_file, audio_duration_ms, audio_chunks, audio_message, video_status, video_progress, video_file, video_duration_ms, video_file_size, video_message, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?24)
            ON CONFLICT(path) DO UPDATE SET
              name = excluded.name,
              extension = excluded.extension,
              size = excluded.size,
              category = CASE
                WHEN material_tasks.category IS NULL OR material_tasks.category = '' THEN excluded.category
                ELSE material_tasks.category
              END,
              material_output_dir = COALESCE(material_tasks.material_output_dir, excluded.material_output_dir),
              updated_at = excluded.updated_at
            "#,
            params![
                file.path,
                file.name,
                file.extension,
                file.size as i64,
                normalize_material_category(&file.category),
                normalize_task_status(&file.status),
                clamp_task_progress(file.progress),
                file.narration_chars,
                file.material_output_dir,
                file.message,
                normalize_task_status(&file.audio_status),
                clamp_task_progress(file.audio_progress),
                file.audio_output_dir,
                file.audio_file,
                file.audio_duration_ms,
                file.audio_chunks,
                file.audio_message,
                normalize_task_status(&file.video_status),
                clamp_task_progress(file.video_progress),
                file.video_file,
                file.video_duration_ms,
                file.video_file_size,
                file.video_message,
                now
            ],
        )
        .map_err(|error| command_error(format!("Upsert material task failed: {error}")))?;
    Ok(())
}

fn material_file_from_path(path: &str, category: &str) -> Result<MaterialFile, CommandError> {
    let path_buf = PathBuf::from(path);
    let metadata = fs::metadata(&path_buf)
        .map_err(|error| command_error(format!("Read file metadata failed: {error}")))?;
    let name = path_buf
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_string();
    let extension = path_buf
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    Ok(MaterialFile {
        path: path.to_string(),
        name,
        extension,
        size: metadata.len(),
        category: normalize_material_category(category),
        status: "pending".to_string(),
        progress: 0,
        narration_chars: None,
        material_output_dir: None,
        message: String::new(),
        audio_status: "pending".to_string(),
        audio_progress: 0,
        audio_output_dir: None,
        audio_file: None,
        audio_duration_ms: None,
        audio_chunks: None,
        audio_message: String::new(),
        video_status: "pending".to_string(),
        video_progress: 0,
        video_file: None,
        video_duration_ms: None,
        video_file_size: None,
        video_message: String::new(),
    })
}

fn load_material_task_by_path(
    connection: &Connection,
    path: &str,
) -> Result<Option<MaterialFile>, CommandError> {
    let result = connection.query_row(
        "SELECT path, name, extension, size, category, status, progress, narration_chars, material_output_dir, message, audio_status, audio_progress, audio_output_dir, audio_file, audio_duration_ms, audio_chunks, audio_message, video_status, video_progress, video_file, video_duration_ms, video_file_size, video_message FROM material_tasks WHERE path = ?1",
        params![path],
        material_task_from_row,
    );
    match result {
        Ok(file) => Ok(Some(file)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(command_error(format!(
            "鐠囪褰囩槐鐘虫綏娴犺濮熸径杈Е閿涙{error}"
        ))),
    }
}

fn collect_material_tasks<F>(
    rows: rusqlite::MappedRows<'_, F>,
) -> Result<Vec<MaterialFile>, CommandError>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<MaterialFile>,
{
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| command_error(format!("鐠囪褰囩槐鐘虫綏娴犺濮熸径杈Е閿涙{error}")))
}

fn material_task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MaterialFile> {
    let size: i64 = row.get(3)?;
    Ok(MaterialFile {
        path: row.get(0)?,
        name: row.get(1)?,
        extension: row.get(2)?,
        size: size.max(0) as u64,
        category: row.get(4)?,
        status: normalize_task_status(&row.get::<_, String>(5)?),
        progress: clamp_task_progress(row.get(6)?),
        narration_chars: row.get(7)?,
        material_output_dir: row.get(8)?,
        message: row.get(9)?,
        audio_status: normalize_task_status(&row.get::<_, String>(10)?),
        audio_progress: clamp_task_progress(row.get(11)?),
        audio_output_dir: row.get(12)?,
        audio_file: row.get(13)?,
        audio_duration_ms: row.get(14)?,
        audio_chunks: row.get(15)?,
        audio_message: row.get(16)?,
        video_status: normalize_task_status(&row.get::<_, String>(17)?),
        video_progress: clamp_task_progress(row.get(18)?),
        video_file: row.get(19)?,
        video_duration_ms: row.get(20)?,
        video_file_size: row.get(21)?,
        video_message: row.get(22)?,
    })
}

fn update_material_task_output_dir(
    connection: &Connection,
    path: &str,
    output_dir: &str,
) -> Result<(), CommandError> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    connection
        .execute(
            "UPDATE material_tasks SET material_output_dir = ?2, updated_at = ?3 WHERE path = ?1",
            params![path, output_dir, now],
        )
        .map_err(|error| {
            command_error(format!(
                "閸愭瑥鍙嗛悽鐔稿灇缁辩姵娼楅弬鍥︽婢剁懓銇戠拹銉窗{error}"
            ))
        })?;
    Ok(())
}

fn update_material_task_audio_status(
    connection: &Connection,
    path: &str,
    status: &str,
    progress: i64,
    output_dir: Option<&str>,
    audio_file: Option<&str>,
    duration_ms: Option<i64>,
    chunks: Option<i64>,
    message: Option<&str>,
) -> Result<(), CommandError> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    connection
        .execute(
            r#"
            UPDATE material_tasks
            SET audio_status = ?2,
                audio_progress = ?3,
                audio_output_dir = COALESCE(?4, audio_output_dir),
                audio_file = COALESCE(?5, audio_file),
                audio_duration_ms = COALESCE(?6, audio_duration_ms),
                audio_chunks = COALESCE(?7, audio_chunks),
                audio_message = ?8,
                updated_at = ?9
            WHERE path = ?1
            "#,
            params![
                path,
                normalize_task_status(status),
                clamp_task_progress(progress),
                output_dir,
                audio_file,
                duration_ms,
                chunks,
                message.unwrap_or_default(),
                now
            ],
        )
        .map_err(|error| {
            command_error(format!("Update material task audio status failed: {error}"))
        })?;
    Ok(())
}

fn clear_material_task_output_dir(connection: &Connection, path: &str) -> Result<(), CommandError> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    connection
        .execute(
            "UPDATE material_tasks SET material_output_dir = NULL, updated_at = ?2 WHERE path = ?1",
            params![path, now],
        )
        .map_err(|error| command_error(format!("Clear material output dir failed: {error}")))?;
    Ok(())
}

fn migrate_task_outputs_to_source_output(
    connection: &Connection,
    file: &mut MaterialFile,
) -> Result<(), CommandError> {
    let source = Path::new(&file.path);
    let output_dir = source_output_dir(source)?;
    fs::create_dir_all(&output_dir)
        .map_err(|error| command_error(format!("閸掓稑缂?output 閻╊喖缍嶆径杈Е閿涙{error}")))?;

    let mut material_output_dir = file.material_output_dir.clone();
    if let Some(value) = file
        .material_output_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let old = PathBuf::from(value);
        if old.exists() && !path_is_inside(&old, &output_dir) {
            let target = unique_child_path(
                &output_dir,
                old.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("materials"),
            );
            copy_path_recursive(&old, &target)?;
            material_output_dir = Some(target.to_string_lossy().into_owned());
            file.material_output_dir = material_output_dir.clone();
        }
    }

    let mut audio_output_dir = file.audio_output_dir.clone();
    if let Some(value) = file
        .audio_output_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let old = PathBuf::from(value);
        if old.exists() && !path_is_inside(&old, &output_dir) {
            let target = unique_child_path(
                &output_dir,
                old.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("audio"),
            );
            copy_path_recursive(&old, &target)?;
            audio_output_dir = Some(target.to_string_lossy().into_owned());
            file.audio_output_dir = audio_output_dir.clone();
        }
    }

    let mut audio_file = file.audio_file.clone();
    if let Some(value) = file
        .audio_file
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let old = PathBuf::from(value);
        if old.is_file() && !path_is_inside(&old, &output_dir) {
            fs::create_dir_all(&output_dir).map_err(|error| {
                command_error(format!(
                    "閸掓稑缂?output 闂婃娊顣堕惄顔肩秿婢惰精瑙﹂敍姝縠{error}"
                ))
            })?;
            let target = unique_child_path(
                &output_dir,
                old.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("audio.mp3"),
            );
            fs::copy(&old, &target).map_err(|error| {
                command_error(format!("鏉╀胶些闂婃娊顣堕弬鍥︽婢惰精瑙﹂敍姝縠{error}"))
            })?;
            audio_file = Some(target.to_string_lossy().into_owned());
            file.audio_file = audio_file.clone();
            if file.audio_output_dir.is_none() {
                audio_output_dir = Some(output_dir.to_string_lossy().into_owned());
                file.audio_output_dir = audio_output_dir.clone();
            }
        }
    }
    if let (Some(material_dir), Some(value)) =
        (material_output_dir.as_deref(), audio_file.as_deref())
    {
        let material_dir = PathBuf::from(material_dir);
        let old = PathBuf::from(value);
        if old.is_file() && !path_is_inside(&old, &material_dir) {
            fs::create_dir_all(&material_dir).map_err(|error| {
                command_error(format!("Create material audio directory failed: {error}"))
            })?;
            let target = unique_child_path(
                &material_dir,
                old.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("audio.mp3"),
            );
            fs::copy(&old, &target).map_err(|error| {
                command_error(format!("Copy audio into material output failed: {error}"))
            })?;
            audio_file = Some(target.to_string_lossy().into_owned());
            audio_output_dir = Some(material_dir.to_string_lossy().into_owned());
            file.audio_file = audio_file.clone();
            file.audio_output_dir = audio_output_dir.clone();
        }
    }

    let mut video_file = file.video_file.clone();
    let mut video_file_size = file.video_file_size;
    if let Some(value) = file
        .video_file
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let old = PathBuf::from(value);
        if old.is_file() && !path_is_inside(&old, &output_dir) {
            fs::create_dir_all(&output_dir).map_err(|error| {
                command_error(format!(
                    "閸掓稑缂?output 鐟欏棝顣堕惄顔肩秿婢惰精瑙﹂敍姝縠{error}"
                ))
            })?;
            let target = unique_child_path(
                &output_dir,
                old.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("video.mp4"),
            );
            fs::copy(&old, &target).map_err(|error| {
                command_error(format!("鏉╀胶些鐟欏棝顣堕弬鍥︽婢惰精瑙﹂敍姝縠{error}"))
            })?;
            video_file_size = fs::metadata(&target)
                .ok()
                .map(|metadata| metadata.len().min(i64::MAX as u64) as i64);
            video_file = Some(target.to_string_lossy().into_owned());
            file.video_file = video_file.clone();
            file.video_file_size = video_file_size;
        }
    }

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    connection
        .execute(
            "UPDATE material_tasks SET material_output_dir=COALESCE(?2, material_output_dir), audio_output_dir=COALESCE(?3, audio_output_dir), audio_file=COALESCE(?4, audio_file), video_file=COALESCE(?5, video_file), video_file_size=COALESCE(?6, video_file_size), updated_at=?7 WHERE path=?1",
            params![
                file.path,
                material_output_dir,
                audio_output_dir,
                audio_file,
                video_file,
                video_file_size,
                now
            ],
        )
        .map_err(|error| command_error(format!("閸ョ偛锝?output 娴溠呭⒖鐠侯垰绶炴径杈Е閿涙{error}")))?;
    Ok(())
}

fn path_is_inside(path: &Path, parent: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let parent = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    path.starts_with(parent)
}

fn unique_child_path(parent: &Path, preferred_name: &str) -> PathBuf {
    let clean = sanitize_file_name(preferred_name).trim().to_string();
    let name = if clean.is_empty() {
        "output".to_string()
    } else {
        clean
    };
    let mut candidate = parent.join(&name);
    if !candidate.exists() {
        return candidate;
    }
    let name_path = Path::new(&name);
    let stem = name_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let extension = name_path.extension().and_then(|value| value.to_str());
    for index in 2..1000 {
        let filename = match extension {
            Some(ext) if !ext.is_empty() => format!("{stem}_{index}.{ext}"),
            _ => format!("{stem}_{index}"),
        };
        candidate = parent.join(filename);
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!(
        "{stem}_{}",
        chrono::Local::now().format("%Y%m%d_%H%M%S")
    ))
}

fn copy_path_recursive(source: &Path, target: &Path) -> Result<(), CommandError> {
    if source.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                command_error(format!("閸掓稑缂撴潻浣盒╅惄顔肩秿婢惰精瑙﹂敍姝縠{error}"))
            })?;
        }
        fs::copy(source, target)
            .map_err(|error| command_error(format!("鏉╀胶些閺傚洣娆㈡径杈Е閿涙{error}")))?;
        return Ok(());
    }
    fs::create_dir_all(target).map_err(|error| {
        command_error(format!("閸掓稑缂撴潻浣盒╅惄顔肩秿婢惰精瑙﹂敍姝縠{error}"))
    })?;
    for entry in fs::read_dir(source).map_err(|error| {
        command_error(format!(
            "鐠囪褰囬崢鍡楀蕉娴溠呭⒖閻╊喖缍嶆径杈Е閿涙{error}"
        ))
    })? {
        let entry = entry.map_err(|error| {
            command_error(format!(
                "鐠囪褰囬崢鍡楀蕉娴溠呭⒖閺夛紕娲版径杈Е閿涙{error}"
            ))
        })?;
        copy_path_recursive(&entry.path(), &target.join(entry.file_name()))?;
    }
    Ok(())
}

fn find_existing_material_output_dir(
    data: &AppData,
    file: &MaterialFile,
) -> Result<Option<PathBuf>, CommandError> {
    let mut base_dirs = vec![source_output_dir(Path::new(&file.path))?];
    let app_export_dir = resolve_export_base_dir(data, "")?;
    if !base_dirs.iter().any(|path| path == &app_export_dir) {
        base_dirs.push(app_export_dir);
    }

    let hints = material_output_hints(file);
    let mut candidates = Vec::new();
    for base_dir in base_dirs {
        if !base_dir.exists() {
            continue;
        }
        if base_dir.join("materials.json").exists() || base_dir.join("narration.txt").exists() {
            candidates.push((
                base_dir
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .ok(),
                base_dir.clone(),
            ));
            continue;
        }
        for entry in fs::read_dir(&base_dir)
            .map_err(|error| command_error(format!("Read output directory failed: {error}")))?
        {
            let entry = entry
                .map_err(|error| command_error(format!("Read output entry failed: {error}")))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_lowercase();
            if path.join("materials.json").exists() || path.join("narration.txt").exists() {
                if !hints
                    .iter()
                    .any(|hint| !hint.is_empty() && name.contains(hint))
                {
                    continue;
                }
                let modified = entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .ok();
                candidates.push((modified, path));
            }
        }
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    Ok(candidates.into_iter().map(|(_, path)| path).next())
}
fn material_output_dir_matches(file: &MaterialFile, path: &Path) -> bool {
    if path.join("materials.json").exists() || path.join("narration.txt").exists() {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();
    material_output_hints(file)
        .iter()
        .any(|hint| !hint.is_empty() && name.contains(hint))
}

fn material_output_hints(file: &MaterialFile) -> Vec<String> {
    let source_stem = Path::new(&file.path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(&file.name);
    let mut hints = Vec::new();
    for value in [source_stem, file.name.as_str()] {
        let sanitized = sanitize_file_name(value).to_lowercase();
        if !sanitized.is_empty() && !hints.contains(&sanitized) {
            hints.push(sanitized);
        }
    }
    hints
}

fn normalize_material_category(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        DEFAULT_MATERIAL_CATEGORY.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_task_status(value: &str) -> String {
    match value {
        "generating" | "success" | "failed" => value.to_string(),
        _ => "pending".to_string(),
    }
}

fn clamp_task_progress(value: i64) -> i64 {
    value.clamp(0, 100)
}

fn update_material_task_progress_db(
    db_path: &Path,
    path: &str,
    status: &str,
    progress: i64,
    message: &str,
) {
    if let Ok(connection) = Connection::open(db_path) {
        let _ = connection.execute(
            "UPDATE material_tasks SET status=?2, progress=?3, message=?4, updated_at=?5 WHERE path=?1",
            params![
                path,
                normalize_task_status(status),
                clamp_task_progress(progress),
                message,
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
            ],
        );
    }
}

fn ensure_speech_region_key_table(connection: &Connection) -> Result<(), CommandError> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS speech_region_keys (
              region TEXT PRIMARY KEY,
              speech_key TEXT NOT NULL,
              voice_name TEXT NOT NULL DEFAULT '',
              output_format TEXT NOT NULL DEFAULT '',
              rate TEXT NOT NULL DEFAULT '',
              pitch TEXT NOT NULL DEFAULT '',
              updated_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|error| {
            command_error(format!("Ensure speech region key table failed: {error}"))
        })?;
    let _ = connection.execute(
        "ALTER TABLE speech_region_keys ADD COLUMN voice_name TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE speech_region_keys ADD COLUMN output_format TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE speech_region_keys ADD COLUMN rate TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE speech_region_keys ADD COLUMN pitch TEXT NOT NULL DEFAULT ''",
        [],
    );
    Ok(())
}

fn ensure_speech_voices_table(connection: &Connection) -> Result<(), CommandError> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS speech_voices (
              voice_name TEXT PRIMARY KEY,
              locale TEXT NOT NULL,
              language TEXT NOT NULL,
              voice_type TEXT NOT NULL,
              gender TEXT NOT NULL,
              styles TEXT NOT NULL,
              roles TEXT NOT NULL,
              source_url TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_speech_voices_locale ON speech_voices(locale);
            "#,
        )
        .map_err(|error| command_error(format!("Ensure speech voices table failed: {error}")))?;
    Ok(())
}

fn seed_speech_voices(connection: &Connection) -> Result<(), CommandError> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut statement = connection
        .prepare(
            r#"
            INSERT OR REPLACE INTO speech_voices
              (locale, language, voice_type, voice_name, gender, styles, roles, source_url, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .map_err(|error| command_error(format!("Prepare seed speech voices failed: {error}")))?;
    for (locale, language, voice_type, voice_name, gender, styles, roles) in SPEECH_VOICE_SEEDS {
        statement
            .execute(params![
                locale,
                language,
                voice_type,
                voice_name,
                gender,
                styles,
                roles,
                MICROSOFT_TTS_LANGUAGE_SUPPORT_URL,
                now
            ])
            .map_err(|error| {
                command_error(format!(
                    "閸愭瑥鍙嗗顔胯拫鐠囶參鐓堕崚妤勩€冩径杈Е閿涙{error}"
                ))
            })?;
    }
    Ok(())
}

fn speech_voice_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SpeechVoice> {
    Ok(SpeechVoice {
        locale: row.get(0)?,
        language: row.get(1)?,
        voice_type: row.get(2)?,
        voice_name: row.get(3)?,
        gender: row.get(4)?,
        styles: row.get(5)?,
        roles: row.get(6)?,
        source_url: row.get(7)?,
    })
}

pub(crate) fn command_error(message: impl Into<String>) -> CommandError {
    CommandError {
        message: message.into(),
    }
}

fn find_video_pipeline(app: &tauri::AppHandle) -> Result<(PathBuf, PathBuf), CommandError> {
    let mut seeds = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        seeds.push(current);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            seeds.push(parent.to_path_buf());
        }
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        seeds.push(resource_dir);
    }
    for seed in seeds {
        let direct = seed.join("book_video_pipeline.py");
        if direct.exists() {
            return Ok((seed.clone(), direct));
        }
        for candidate in seed.ancestors() {
            let script = candidate.join("tmp").join("book_video_pipeline.py");
            if script.exists() {
                return Ok((candidate.to_path_buf(), script));
            }
            let nested = candidate.join("a-book-in-30-minutes");
            let nested_script = nested.join("tmp").join("book_video_pipeline.py");
            if nested_script.exists() {
                return Ok((nested, nested_script));
            }
        }
    }
    Err(command_error(
        "找不到视频流水线脚本 tmp/book_video_pipeline.py，请确认 dev 版目录完整。",
    ))
}

fn source_output_dir(source_path: &Path) -> Result<PathBuf, CommandError> {
    let parent = source_path
        .parent()
        .ok_or_else(|| command_error("閺冪姵纭剁€规矮缍呭┃鎰姛閹碘偓閸︺劎娲拌ぐ鏇樷偓?"))?;
    Ok(parent.join("output"))
}
fn find_python_command() -> String {
    "python".to_string()
}

fn parse_last_json_object(stdout: &str) -> Result<serde_json::Value, CommandError> {
    for (index, _) in stdout.match_indices('{').rev() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&stdout[index..]) {
            return Ok(value);
        }
    }
    Err(command_error(format!(
        "Could not parse JSON from video pipeline output: {}",
        text_preview(stdout, 600)
    )))
}
fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .map(ToString::to_string)
        .filter(|item| !item.trim().is_empty())
}

fn json_i64(value: &serde_json::Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|item| item.as_i64())
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> CommandError {
    CommandError {
        message: error.to_string(),
    }
}

fn build_trace_id(provided: Option<&str>) -> String {
    let provided = provided.unwrap_or("").trim();
    if !provided.is_empty() {
        return sanitize_trace_id(provided);
    }
    format!(
        "materials-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S-%3f")
    )
}

fn sanitize_trace_id(value: &str) -> String {
    value
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.') {
                char
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(96)
        .collect()
}

fn ai_profile_debug_detail(settings: &AppSettings) -> String {
    let key_length = settings.ai_profile.api_key.trim().chars().count();
    format!(
        "profile={} provider={} model={} base_url={} api_key_present={} api_key_length={} notifications_enabled={}",
        settings.ai_profile.name,
        settings.ai_profile.provider,
        settings.ai_profile.model,
        settings.ai_profile.base_url,
        key_length > 0,
        key_length,
        settings.notifications_enabled
    )
}

fn source_file_detail(path: &Path) -> String {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let size = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or_default();
    format!(
        "path={} extension={} size_bytes={}",
        path.to_string_lossy(),
        extension,
        size
    )
}

fn emit_material_progress(
    app: &tauri::AppHandle,
    trace_id: &str,
    path: &str,
    step: usize,
    message: &str,
) {
    let bounded_step = step.clamp(1, MATERIAL_PROGRESS_STEPS);
    let progress = ((bounded_step * 100) / MATERIAL_PROGRESS_STEPS) as i64;
    let status = if bounded_step >= MATERIAL_PROGRESS_STEPS {
        "success"
    } else {
        "generating"
    };
    let _ = app.emit(
        "material-task-progress",
        MaterialTaskProgressEvent {
            trace_id: trace_id.to_string(),
            path: path.to_string(),
            status: status.to_string(),
            progress,
            step: bounded_step,
            total_steps: MATERIAL_PROGRESS_STEPS,
            message: format!("{bounded_step}/{MATERIAL_PROGRESS_STEPS} {message}"),
        },
    );
}

fn chapter_debug_list(book: &EpubBook) -> String {
    book.overview
        .chapters
        .iter()
        .take(8)
        .map(|chapter| format!("{}({})", chapter.title, chapter.chars))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn text_preview(value: &str, max_chars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let preview = compact.chars().take(max_chars).collect::<String>();
    preview.replace(['\r', '\n', '\t'], " ")
}

enum SourceReadResult {
    Ok(EpubBook),
    Err(CommandError),
    Panic(String),
    Timeout,
}

fn read_source_book_with_timeout(path: PathBuf, _timeout: Duration) -> SourceReadResult {
    match read_source_book(&path) {
        Ok(book) => SourceReadResult::Ok(book),
        Err(error) => SourceReadResult::Err(error),
    }
}

fn read_source_book(path: &Path) -> Result<EpubBook, CommandError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "epub" => read_epub(path),
        "txt" => read_txt_book(path),
        "docx" => Err(command_error(
            "DOCX is not supported yet. Please convert it to TXT or EPUB.",
        )),
        "pdf" => Err(command_error(
            "PDF is not supported yet. Please convert it to TXT or EPUB.",
        )),
        _ => Err(command_error(format!(
            "Unsupported source file type: {}. Supported: EPUB or TXT.",
            if extension.is_empty() {
                "unknown"
            } else {
                extension.as_str()
            }
        ))),
    }
}

fn read_txt_book(path: &Path) -> Result<EpubBook, CommandError> {
    let text = fs::read_to_string(path)
        .map_err(|error| command_error(format!("Failed to read TXT file: {error}")))?;
    let title = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled")
        .trim()
        .to_string();
    let title = if title.is_empty() {
        "Untitled".to_string()
    } else {
        title
    };
    let total_chars = count_han_chars(&text);
    let chapter_title = title.clone();

    Ok(EpubBook {
        overview: EpubOverview {
            title,
            creator: String::new(),
            publisher: String::new(),
            language: "zh-CN".to_string(),
            total_chars,
            chapters: vec![EpubChapterSummary {
                title: chapter_title.clone(),
                chars: total_chars,
            }],
        },
        chapters: vec![EpubChapter {
            title: chapter_title,
            text,
        }],
    })
}

fn build_local_book_materials_payload(
    book: &EpubBook,
    request: &BookMaterialsRequest,
) -> AiBookMaterialsPayload {
    let source = build_source_packet(book);
    let title = if book.overview.title.trim().is_empty() {
        "?????"
    } else {
        book.overview.title.trim()
    };
    let mut narration = format!("???????{title}???????????????????????????????{source}");
    let min_chars = request.target_min_chars.max(1000);
    while count_han_chars(&narration) < min_chars {
        narration
            .push_str("\n\n??????????????????????????????????????????????????????????????????????");
    }
    AiBookMaterialsPayload {
        video_title: format!("?{title}?30???????"),
        description: format!("?{title}????????"),
        tags: vec!["??".to_string(), "??".to_string(), "????".to_string()],
        narration,
    }
}

fn build_repair_prompt(
    payload: &AiBookMaterialsPayload,
    min_chars: usize,
    max_chars: usize,
) -> String {
    build_narration_rewrite_prompt(
        payload,
        count_han_chars(&payload.narration),
        min_chars,
        max_chars,
    )
}

fn tail_chars(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(max_chars);
    chars[start..].iter().collect()
}

fn build_book_materials_prompt(book: &EpubBook, request: &BookMaterialsRequest) -> String {
    let target_min = request.target_min_chars;
    let target_max = request.target_max_chars;
    let source_packet = build_source_packet(book);
    format!(
        "You are writing a Chinese audiobook video package. Return only valid JSON with keys: videoTitle, description, tags, narration. The narration must be a continuous Chinese script between {target_min} and {target_max} Chinese characters. Do not output markdown.\n\nBook title: {title}\nAuthor: {author}\nLanguage: {language}\nExtra direction: {direction}\n\nSource excerpts:\n{source_packet}",
        title = book.overview.title,
        author = book.overview.creator,
        language = request.language,
        direction = request.extra_direction.clone(),
    )
}

fn build_source_packet(book: &EpubBook) -> String {
    let mut packet = Vec::new();
    let total = book.chapters.len().max(1);
    let mut selected_indexes = vec![
        0usize,
        total / 5,
        total * 2 / 5,
        total * 3 / 5,
        total * 4 / 5,
        total.saturating_sub(1),
    ];
    selected_indexes.sort_unstable();
    selected_indexes.dedup();
    for index in selected_indexes {
        if let Some(chapter) = book.chapters.get(index) {
            let text = chapter.text.replace('\n', " ");
            let excerpt = truncate_chars(&text, 520);
            packet.push(format!("Chapter: {}\n{}", chapter.title, excerpt));
        }
        if packet.join("\n\n").chars().count() > 4200 {
            break;
        }
    }
    if packet.is_empty() {
        for chapter in book.chapters.iter().take(4) {
            packet.push(format!(
                "Chapter: {}\n{}",
                chapter.title,
                truncate_chars(&chapter.text.replace('\n', " "), 520)
            ));
        }
    }
    packet.join("\n\n")
}

fn build_narration_rewrite_prompt(
    payload: &AiBookMaterialsPayload,
    current_chars: usize,
    min_chars: usize,
    max_chars: usize,
) -> String {
    format!(
        "Rewrite the JSON so narration is between {min_chars} and {max_chars} Chinese characters. Current narration Chinese chars: {current_chars}. Return only valid JSON with the same keys. Existing JSON:\n{}",
        serde_json::to_string(payload).unwrap_or_default()
    )
}

fn build_narration_extension_prompt(
    payload: &AiBookMaterialsPayload,
    current_chars: usize,
    min_chars: usize,
    max_chars: usize,
) -> String {
    let tail = tail_chars(&payload.narration, 700);
    format!(
        "Continue and expand this Chinese narration from {current_chars} to {min_chars}-{max_chars} Chinese characters. Return only the additional narration text, no markdown. Tail:\n{tail}"
    )
}

fn clean_narration_extension(value: &str) -> String {
    let trimmed = value.trim();
    if let Ok(payload) = serde_json::from_str::<AiBookMaterialsPayload>(trimmed) {
        return payload.narration.trim().to_string();
    }
    trimmed.trim_matches('`').trim().to_string()
}

fn merge_narration_extension(base: &str, extension: &str) -> String {
    let base = base.trim();
    let extension = extension.trim();
    if base.is_empty() {
        extension.to_string()
    } else if extension.is_empty() {
        base.to_string()
    } else {
        format!("{base}\n\n{extension}")
    }
}

fn build_local_narration_extension(
    payload: &AiBookMaterialsPayload,
    _current_chars: usize,
    min_chars: usize,
    max_chars: usize,
) -> String {
    let mut output = String::new();
    let seed = if payload.description.trim().is_empty() {
        payload.video_title.trim()
    } else {
        payload.description.trim()
    };
    while count_han_chars(&output) + count_han_chars(&payload.narration) < min_chars {
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(seed);
        output.push_str(" ???????????????????????????????????????????????????");
        if count_han_chars(&output) + count_han_chars(&payload.narration) > max_chars {
            break;
        }
    }
    output
}

fn fallback_excerpt(text: &str, seed: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return String::new();
    }
    let window = 180usize.min(chars.len());
    let max_start = chars.len().saturating_sub(window);
    let start = if max_start == 0 {
        0
    } else {
        (seed * 379) % max_start
    };
    chars[start..start + window].iter().collect::<String>()
}

fn build_local_fallback_paragraph(
    title: &str,
    chapter_title: &str,
    excerpt: &str,
    index: usize,
) -> String {
    let cleaned = excerpt.split_whitespace().collect::<Vec<_>>().join(" ");
    format!(
        "?{title}???{chapter_title}????????????? {index} ??????????????????????????????{cleaned} ???????????????????????????????????",
        index = index + 1
    )
}

fn trim_to_han_limit(value: &str, max_han_chars: usize) -> String {
    let mut han_chars = 0usize;
    let mut output = String::new();
    for ch in value.chars() {
        if is_han_char(ch) {
            if han_chars >= max_han_chars {
                break;
            }
            han_chars += 1;
        }
        output.push(ch);
    }
    output.trim().to_string()
}

fn is_han_char(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

fn parse_book_materials_payload(content: &str) -> Result<AiBookMaterialsPayload, CommandError> {
    let json = extract_json_object(content)
        .ok_or_else(|| command_error("AI 濞屸剝婀佹潻鏂挎礀閸欘垵袙閺嬫劗娈?JSON閵?"))?;
    let payload = serde_json::from_str::<AiBookMaterialsPayload>(&json)
        .map_err(|error| command_error(format!("AI JSON 鐟欙絾鐎芥径杈Е閿涙{error}")))?;
    if payload.video_title.trim().is_empty() {
        return Err(command_error(
            "AI 鏉╂柨娲栭惃鍕潒妫版垶鐖ｆ０妯硅礋缁屾亽鈧?",
        ));
    }
    if payload.narration.trim().is_empty() {
        return Err(command_error("AI 鏉╂柨娲栭惃鍕⒑閻х晫顭堟稉铏光敄閵?"));
    }
    Ok(payload)
}

fn extract_json_object(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }
    let fence_re = Regex::new(r#"(?s)```(?:json)?\s*(\{.*?\})\s*```"#).ok()?;
    if let Some(captures) = fence_re.captures(trimmed) {
        return captures.get(1).map(|value| value.as_str().to_string());
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end > start {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

fn split_subtitles(narration: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in narration.chars() {
        if ch.is_whitespace() {
            continue;
        }
        let should_break = matches!(
            ch,
            '?' | '?' | '?' | '.' | '!' | '?' | ';' | '?' | ',' | '?' | '\n' | '\r'
        );
        if !should_break {
            current.push(ch);
        }
        if should_break || current.chars().count() >= 14 {
            push_clean_subtitle(&mut lines, &current);
            current.clear();
        }
    }
    push_clean_subtitle(&mut lines, &current);
    if lines.is_empty() {
        push_subtitle_chunks(&mut lines, narration, 14);
    }
    lines
}

fn push_clean_subtitle(lines: &mut Vec<String>, cleaned: &str) {
    if cleaned.is_empty() {
        return;
    }
    push_subtitle_chunks(lines, cleaned, 14);
}

fn push_subtitle_chunks(lines: &mut Vec<String>, text: &str, max_chars: usize) {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        lines.push(text.to_string());
        return;
    }
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        lines.push(chars[start..end].iter().collect::<String>());
        start = end;
    }
}

fn resolve_export_base_dir(data: &AppData, output_dir: &str) -> Result<PathBuf, CommandError> {
    if output_dir.is_empty() {
        let parent = data
            .settings_path
            .parent()
            .ok_or_else(|| command_error("閺冪姵纭剁€规矮缍呮妯款吇鐎电厧鍤惄顔肩秿閵?"))?;
        return Ok(parent.join("exports"));
    }
    Ok(PathBuf::from(output_dir))
}

fn write_book_materials_package(
    data: &AppData,
    output_dir: &str,
    materials: &BookMaterials,
    trace_id: &str,
) -> Result<ExportBookMaterialsResult, CommandError> {
    let base_dir = resolve_export_base_dir(data, output_dir)?;
    fs::create_dir_all(&base_dir).map_err(|error| {
        command_error(format!("閸掓稑缂撶€电厧鍤惄顔肩秿婢惰精瑙﹂敍姝縠{error}"))
    })?;

    let output_dir = base_dir;

    let mut files = Vec::new();
    write_material_file(&output_dir, &mut files, "title.txt", &materials.video_title)?;
    write_material_file(
        &output_dir,
        &mut files,
        "description.txt",
        &materials.description,
    )?;
    write_material_file(
        &output_dir,
        &mut files,
        "tags.txt",
        &materials.tags.join(", "),
    )?;
    write_material_file(
        &output_dir,
        &mut files,
        "narration.txt",
        &materials.narration,
    )?;
    write_material_file(
        &output_dir,
        &mut files,
        "subtitles.txt",
        &materials.subtitles.join("\n"),
    )?;
    write_material_file(
        &output_dir,
        &mut files,
        "draft.srt",
        &build_srt(&materials.subtitles),
    )?;
    write_material_file(&output_dir, &mut files, "prompt.txt", &materials.prompt)?;
    write_material_file(
        &output_dir,
        &mut files,
        "overview.json",
        &serde_json::to_string_pretty(&materials.overview).unwrap_or_default(),
    )?;
    write_material_file(
        &output_dir,
        &mut files,
        "materials.json",
        &serde_json::to_string_pretty(materials).unwrap_or_default(),
    )?;
    write_material_file(
        &output_dir,
        &mut files,
        "README.md",
        &build_export_readme(materials),
    )?;

    data.logger.debug(
        "materials",
        "export.files",
        "Material package files written",
        format!(
            "trace_id={} files={} output_dir={}",
            trace_id,
            files.len(),
            output_dir.to_string_lossy()
        ),
        trace_id,
    );
    Ok(ExportBookMaterialsResult {
        output_dir: output_dir.to_string_lossy().into_owned(),
        files,
    })
}

fn sanitize_file_name(value: &str) -> String {
    let invalid_re = Regex::new(
        r#"[\/:*?"<>|
	]+"#,
    )
    .expect("valid regex");
    let space_re = Regex::new(r#"\s+"#).expect("valid regex");
    let cleaned = invalid_re.replace_all(value.trim(), "_");
    let cleaned = space_re.replace_all(&cleaned, "_");
    cleaned.trim_matches('_').chars().take(60).collect()
}

fn write_material_file(
    output_dir: &Path,
    files: &mut Vec<String>,
    file_name: &str,
    content: &str,
) -> Result<(), CommandError> {
    let path = output_dir.join(file_name);
    let mut file = fs::File::create(&path)
        .map_err(|error| command_error(format!("Create file {file_name} failed: {error}")))?;
    file.write_all(content.as_bytes())
        .map_err(|error| command_error(format!("Write file {file_name} failed: {error}")))?;
    files.push(path.to_string_lossy().into_owned());
    Ok(())
}

fn open_directory_in_explorer(path: &Path) -> Result<(), CommandError> {
    Command::new("explorer")
        .arg(path)
        .spawn()
        .map_err(|error| {
            command_error(format!(
                "閹垫挸绱戦悽鐔稿灇缁辩姵娼楅弬鍥︽婢剁懓銇戠拹銉窗{error}"
            ))
        })?;
    Ok(())
}

fn build_srt(subtitles: &[String]) -> String {
    let mut output = String::new();
    let mut start_ms = 0u64;
    for (index, line) in subtitles.iter().enumerate() {
        let duration_ms = estimate_subtitle_duration_ms(line);
        let end_ms = start_ms + duration_ms;
        output.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            format_srt_time(start_ms),
            format_srt_time(end_ms),
            line
        ));
        start_ms = end_ms + 120;
    }
    output
}

fn estimate_subtitle_duration_ms(line: &str) -> u64 {
    let han = count_han_chars(line).max(1) as u64;
    (han * 260).clamp(900, 4800)
}

fn format_srt_time(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

fn build_export_readme(materials: &BookMaterials) -> String {
    format!(
        "# {title}

- Book: {book}
- Creator: {creator}
- Model: {model}
- Narration Chinese chars: {narration_chars}
- Subtitle lines: {subtitle_count}

Files: title.txt, description.txt, tags.txt, narration.txt, subtitles.txt, draft.srt, prompt.txt, overview.json, materials.json.
",
        title = materials.video_title,
        book = materials.overview.title,
        creator = materials.overview.creator,
        model = materials.model,
        narration_chars = count_han_chars(&materials.narration),
        subtitle_count = materials.subtitles.len()
    )
}
