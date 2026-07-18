use crate::epub::{count_han_chars, read_epub, truncate_chars};
use crate::models::{
    AiBookMaterialsPayload, AiGenerateRequest, AiGenerateResult, AiTestResult, AppSettings,
    AppStatePayload, AudioManifest, AudioManifestPart, BookMaterials, BookMaterialsRequest,
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, EpubBook, EpubChapter,
    EpubChapterSummary, EpubOverview, ExportBookMaterialsRequest, ExportBookMaterialsResult,
    FeishuSendRequest, FeishuSendResult, FeishuWebhookResponse, GenerateAudioRequest,
    GenerateAudioResult, GenerateBookVideoRequest, GenerateBookVideoResult,
    GeminiContent, GeminiGenerateRequest, GeminiGenerateResponse, GeminiPart,
    GenerateMaterialTaskAudioRequest, GeneratePublishMaterialsRequest,
    GeneratePublishMaterialsResult, GetMaterialTaskStepsRequest, GetMaterialTaskStepsResult,
    GetMaterialTasksRequest, GetOperationLogsRequest, GetOperationLogsResult,
    GetSpeechVoicesResult, MaterialFile, MaterialOutputDirRequest, MaterialTaskPathRequest,
    MaterialTaskProgressEvent, MaterialTaskStep, OperationLogEntry,
    ResetMaterialTasksRequest, ScanMaterialFilesRequest, ScanMaterialFilesResult,
    SpeechPreviewRequest, SpeechProfile, SpeechRegionKeyRequest, SpeechRegionKeyResult,
    SpeechTestResult, SpeechVoice, ToolTestResult, UpdateInfo, UpdateMaterialTaskStageStatusRequest,
    UpdateMaterialTaskStatusRequest,
};
use crate::operation_log::OperationLogger;
use base64::Engine;
use regex::Regex;
use rusqlite::params;
use rusqlite::Connection;
use serde::Deserialize;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, State};

const SOURCE_READ_TIMEOUT_SECONDS: u64 = 30;
const AI_REQUEST_TIMEOUT_SECONDS: u64 = 600;
const AI_REQUEST_MAX_ATTEMPTS: usize = 3;
const FEISHU_REQUEST_TIMEOUT_SECONDS: u64 = 20;
const SPEECH_REQUEST_TIMEOUT_SECONDS: u64 = 300;
const SPEECH_CHUNK_MAX_CHARS: usize = 900;
const SPEECH_CHUNK_MAX_SENTENCES: usize = 100;
const SPEECH_CHUNK_MAX_ESTIMATED_MS: u64 = 8 * 60 * 1000;
const MICROSOFT_TTS_LANGUAGE_SUPPORT_URL: &str =
    "Operation completed.";
const DEFAULT_MATERIAL_CATEGORY: &str = "半小时听完一本书";
const MATERIAL_TASK_SELECT_COLUMNS: &str = "path, name, extension, size, category, status, progress, narration_chars, material_output_dir, message, audio_status, audio_progress, audio_output_dir, audio_file, audio_duration_ms, audio_chunks, audio_message, image_status, image_progress, image_output_dir, image_message, subtitle_status, subtitle_progress, subtitle_file, subtitle_message, video_status, video_progress, video_file, video_duration_ms, video_file_size, video_message";
const MATERIAL_PROGRESS_STEPS: usize = 4;
const STAGE_CONTENT_DIR: &str = "01_content";
const STAGE_AUDIO_DIR: &str = "02_audio";
const STAGE_SUBTITLES_DIR: &str = "03_subtitles";
const STAGE_VIDEO_DIR: &str = "05_video";
const STAGE_PUBLISH_DIR: &str = "06_publish";
const NARRATION_TARGET_MIN_UNIT_HAN: usize = 8;
const NARRATION_TARGET_MAX_UNIT_HAN: usize = 12;
const SUBTITLE_MAX_CHARS: usize = 18;
const SUBTITLE_SOFT_CHARS: usize = 10;
const SUBTITLE_MIN_TAIL_CHARS: usize = 6;
const SOURCE_MIN_USABLE_HAN_CHARS: usize = 200;
const NARRATION_LOCAL_TRIM_TOLERANCE: usize = 360;

const SPEECH_VOICE_SEEDS: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaoxiaoNeural", "Female", "assistant, chat, customerservice, newscast, affectionate, angry, calm, cheerful, disgruntled, fearful, gentle, lyrical, sad, serious", "Girl, YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunxiNeural", "Male", "assistant, chat, narration-relaxed, angry, cheerful, depressed, disgruntled, embarrassed, fearful, sad, serious", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunjianNeural", "Male", "narration-relaxed, sports-commentary, sports-commentary-excited", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaoyiNeural", "Female", "affectionate, angry, cheerful, disgruntled, embarrassed, fearful, gentle, sad, serious", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunyangNeural", "Male", "customerservice, narration-professional, newscast-casual", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaochenNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "MultilingualNeural", "zh-CN-XiaochenMultilingualNeural", "Female", "multilingual", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaohanNeural", "Female", "calm, fearful, cheerful, disgruntled, serious, angry, sad, gentle, affectionate, embarrassed", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaomengNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaomoNeural", "Female", "affectionate, angry, calm, cheerful, depressed, disgruntled, embarrassed, envious, fearful, gentle, sad, serious", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaoqiuNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaorouNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaoruiNeural", "Female", "angry, calm, fearful, sad", "Senior"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaoshuangNeural", "Female", "chat", "Child"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaoxiaoDialectsNeural", "Female", "dialect", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "MultilingualNeural", "zh-CN-XiaoxiaoMultilingualNeural", "Female", "multilingual", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaoyanNeural", "Female", "general", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaoyouNeural", "Female", "general", "Child"),
    ("zh-CN", "中文（普通话，简体）", "MultilingualNeural", "zh-CN-XiaoyuMultilingualNeural", "Female", "multilingual", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-XiaozhenNeural", "Female", "angry, cheerful, disgruntled, fearful, sad, serious", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunfengNeural", "Male", "angry, cheerful, depressed, disgruntled, fearful, sad, serious", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunhaoNeural", "Male", "advertisement-upbeat", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunjieNeural", "Male", "angry, cheerful, depressed, disgruntled, documentary-narration, fearful, sad, serious", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunxiaNeural", "Male", "angry, calm, cheerful, fearful, sad", "Child"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunyeNeural", "Male", "general", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "MultilingualNeural", "zh-CN-YunyiMultilingualNeural", "Male", "multilingual", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "Neural", "zh-CN-YunzeNeural", "Male", "calm, cheerful, depressed, disgruntled, documentary-narration, fearful, sad, serious", "OlderAdult"),
    ("zh-CN", "中文（普通话，简体）", "MultilingualNeural", "zh-CN-YunfanMultilingualNeural", "Male", "multilingual", "YoungAdult"),
    ("zh-CN", "中文（普通话，简体）", "MultilingualNeural", "zh-CN-YunxiaoMultilingualNeural", "Male", "multilingual", "YoungAdult"),
    ("en-US", "英语（美国）", "Neural", "en-US-JennyNeural", "Female", "assistant, chat, customerservice, newscast", "YoungAdult"),
    ("en-US", "英语（美国）", "Neural", "en-US-GuyNeural", "Male", "newscast", "YoungAdult"),
    ("en-US", "英语（美国）", "Neural", "en-US-AriaNeural", "Female", "chat, customerservice, newscast", "YoungAdult"),
    ("en-US", "英语（美国）", "Neural", "en-US-DavisNeural", "Male", "chat", "YoungAdult"),
    ("en-US", "英语（美国）", "Neural", "en-US-JaneNeural", "Female", "general", "YoungAdult"),
    ("en-US", "英语（美国）", "Neural", "en-US-JasonNeural", "Male", "general", "YoungAdult"),
    ("en-US", "英语（美国）", "Neural", "en-US-NancyNeural", "Female", "general", "YoungAdult"),
    ("en-US", "英语（美国）", "Neural", "en-US-TonyNeural", "Male", "general", "YoungAdult"),
    ("en-GB", "英语（英国）", "Neural", "en-GB-SoniaNeural", "Female", "general", "YoungAdult"),
    ("en-GB", "英语（英国）", "Neural", "en-GB-RyanNeural", "Male", "general", "YoungAdult"),
    ("en-GB", "英语（英国）", "Neural", "en-GB-LibbyNeural", "Female", "general", "YoungAdult"),
];

pub struct AppData {
    settings: Mutex<AppSettings>,
    db_path: PathBuf,
    logger: OperationLogger,
    app_started_at: String,
}

#[derive(Clone)]
struct AudioTaskProgress {
    db_path: PathBuf,
    path: String,
    trace_id: String,
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

    fn step(&self, step_code: &str, step_name: &str, status: &str, progress: i64, detail: &str) {
        upsert_material_task_step_db(
            &self.db_path,
            &self.trace_id,
            &self.path,
            step_code,
            step_name,
            status,
            progress,
            detail,
        );
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

        init_app_tables(&db_path, &logger);

        let legacy_settings_file_exists = settings_path.exists();
        let settings_load_result = load_settings_from_database_or_migrate_legacy_json(&db_path, &settings_path);
        let settings_load_error = settings_load_result.as_ref().err().cloned();
        let settings_loaded = settings_load_result.is_ok();
        let mut settings = settings_load_result.unwrap_or_default();
        let settings_migrated = sanitize_persisted_settings(&mut settings);
        if settings_migrated || settings_loaded {
            let _ = save_settings_to_database(&db_path, &settings);
        }

        let app_started_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        logger.info("app", "startup", "应用已启动。");
        logger.debug(
            "settings",
            "load",
            "配置已从数据库加载。",
            format!(
                "配置源=app.db；旧配置路径={}；旧配置存在={}；已清理异常配置={}；密钥已填写={}；密钥长度={}；加载错误={}",
                settings_path.to_string_lossy(),
                legacy_settings_file_exists,
                settings_migrated,
                !settings.ai_profile.api_key.trim().is_empty(),
                settings.ai_profile.api_key.trim().chars().count(),
                settings_load_error.unwrap_or_else(|| "无".to_string())
            ),
            "startup",
        );
        Self {
            settings: Mutex::new(settings),
            db_path,
            logger,
            app_started_at,
        }
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<(), CommandError> {
        save_settings_to_database(&self.db_path, settings)?;
        self.logger.info("settings", "save", "配置已保存到数据库。");
        Ok(())
    }
}

pub async fn run_e2e_materials_cli(epub_path: &str) -> Result<(), CommandError> {
    let started = Instant::now();
    let app_data_dir = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("com.abookin30minutes.desktop");
    let db_path = app_data_dir.join("app.db");
    let settings_path = app_data_dir.join("settings.json");
    let log_dir = app_data_dir.join("logs");
    let logger = OperationLogger::new(db_path.clone(), log_dir);
    init_app_tables(&db_path, &logger);
    let mut settings = load_settings_from_database_or_migrate_legacy_json(&db_path, &settings_path)
        .unwrap_or_default();
    if sanitize_persisted_settings(&mut settings) {
        let _ = save_settings_to_database(&db_path, &settings);
    }

    let trace_id = format!("e2e-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    logger.trace_info(
        "materials",
        "generate.start",
        "Start generating book materials.",
        format!("trace_id={} source={}", trace_id, epub_path),
        &trace_id,
    );

    let epub = PathBuf::from(epub_path.trim());
    if epub_path.trim().is_empty() {
        return Err(command_error("EPUB path is required."));
    }
    if !epub.exists() {
        return Err(command_error(format!("EPUB file does not exist: {epub_path}")));
    }
    let request = BookMaterialsRequest {
        epub_path: epub.to_string_lossy().into_owned(),
        target_min_chars: settings.material_profile.target_min_chars,
        target_max_chars: settings.material_profile.target_max_chars,
        channel_name: settings.material_profile.channel_name.clone(),
        language: settings.material_profile.language.clone(),
        extra_direction: settings.material_profile.extra_direction.clone(),
        trace_id: Some(trace_id.clone()),
    };
    let book = read_epub(&epub)?;
    logger.trace_info(
        "materials",
        "source.read.done",
        "Source book read successfully.",
        format!(
            "title={} creator={} chapters={} total_han_chars={}",
            book.overview.title,
            book.overview.creator,
            book.chapters.len(),
            book.overview.total_chars
        ),
        &trace_id,
    );

    let prompt = build_book_materials_prompt(&book, &request);
    let system_prompt =
        "You are a Chinese audiobook script editor. Return only valid JSON with keys videoTitle, description, tags, narration."
            .to_string();
    logger.trace_info(
        "materials",
        "ai.request",
        "Requesting AI material generation JSON.",
        format!(
            "model={} base_url={} prompt_chars={}",
            current_ai_model(&settings),
            current_ai_base_url(&settings),
            prompt.chars().count()
        ),
        &trace_id,
    );
    let content = call_ai(
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
    .unwrap_or_default();
    let mut payload = if content.trim().is_empty() {
        logger.error(
            "materials",
            "ai.initial.empty",
            "AI returned no usable material content.",
            "AI response was empty or failed.",
        );
        return Err(command_error("AI returned no usable material content."));
    } else {
        parse_book_materials_payload(&content)?
    };
    payload.narration = sanitize_generated_narration(&payload.narration);
    let min_chars = request.target_min_chars.max(1000);
    let max_chars = request.target_max_chars.max(min_chars + 1);
    for _ in 0..=6 {
        let current_chars = count_han_chars(&payload.narration);
        if current_chars >= min_chars && current_chars <= max_chars {
            break;
        }
        let repair_prompt = if current_chars < min_chars {
            build_narration_extension_prompt(&payload, current_chars, min_chars, max_chars)
        } else {
            build_repair_prompt(&payload, min_chars, max_chars)
        };
        let Ok(repair_response) = call_ai(
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
        .await else {
            break;
        };
        if current_chars < min_chars {
            let extension = clean_narration_extension(&repair_response);
            if extension.trim().is_empty() || narration_extension_is_repetitive(&payload.narration, &extension) {
                break;
            }
            payload.narration = merge_narration_extension(&payload.narration, &extension);
            payload.narration = sanitize_generated_narration(&payload.narration);
        } else if let Ok(mut next_payload) = parse_book_materials_payload(&repair_response) {
            next_payload.narration = sanitize_generated_narration(&next_payload.narration);
            payload = next_payload;
        } else {
            break;
        }
    }
    let final_chars = count_han_chars(&payload.narration);
    if final_chars < min_chars || final_chars > max_chars {
        return Err(command_error(format!(
            "Narration length out of range: {} target {}..{}",
            final_chars, min_chars, max_chars
        )));
    }
    let mut subtitles = normalize_ai_subtitles(&payload.subtitles, &payload.narration)
        .unwrap_or_else(|| split_subtitles(&payload.narration));
    if subtitles_need_ai_rewrite(&subtitles, &payload.narration)
        && ai_subtitle_rewrite_enabled()
    {
        if let Some(rewritten) = rewrite_subtitles_with_ai(&settings, &payload.narration, &subtitles).await {
            subtitles = rewritten;
        }
    }
    let materials = BookMaterials {
        video_title: payload.video_title,
        description: payload.description,
        tags: payload.tags,
        narration: payload.narration,
        subtitles,
        prompt,
        model: current_ai_model(&settings),
        overview: book.overview,
    };
    let output_dir = source_output_dir(&epub)?.to_string_lossy().into_owned();
    let data = AppData {
        settings: Mutex::new(settings),
        db_path: db_path.clone(),
        logger: logger.clone(),
        app_started_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    let result = write_book_materials_package(&data, &output_dir, &materials, &trace_id)?;
    if let Ok(connection) = Connection::open(&db_path) {
        ensure_material_tasks_table(&connection)?;
        update_material_task_output_dir(&connection, request.epub_path.trim(), &result.output_dir)?;
        update_material_task_progress_db(
            &db_path,
            request.epub_path.trim(),
            "success",
            100,
            "Book materials generated.",
        );
    }
    logger.trace_info(
        "materials",
        "generate.done",
        "Book materials generated.",
        format!(
            "elapsed_ms={} output_dir={} files={} title={} narration_han_chars={} subtitles={}",
            started.elapsed().as_millis(),
            result.output_dir,
            result.files.len(),
            materials.video_title,
            count_han_chars(&materials.narration),
            materials.subtitles.len()
        ),
        &trace_id,
    );
    println!(
        "E2E materials passed trace_id={} output_dir={} narration_han_chars={} subtitles={}",
        trace_id,
        result.output_dir,
        count_han_chars(&materials.narration),
        materials.subtitles.len()
    );
    Ok(())
}

pub async fn run_e2e_audio_cli(epub_path: &str) -> Result<(), CommandError> {
    let app_data_dir = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("com.abookin30minutes.desktop");
    let db_path = app_data_dir.join("app.db");
    let settings_path = app_data_dir.join("settings.json");
    let log_dir = app_data_dir.join("logs");
    let logger = OperationLogger::new(db_path.clone(), log_dir);
    init_app_tables(&db_path, &logger);
    let mut settings = load_settings_from_database_or_migrate_legacy_json(&db_path, &settings_path)
        .unwrap_or_default();
    if sanitize_persisted_settings(&mut settings) {
        let _ = save_settings_to_database(&db_path, &settings);
    }
    let epub = PathBuf::from(epub_path.trim());
    if !epub.exists() {
        return Err(command_error(format!("EPUB file does not exist: {epub_path}")));
    }
    let output_dir = source_output_dir(&epub)?;
    let narration_file = staged_material_path(&output_dir, "narration.txt");
    let narration = fs::read_to_string(&narration_file).map_err(|error| {
        command_error(format!(
            "Read narration file failed: {} ({error})",
            narration_file.to_string_lossy()
        ))
    })?;
    let data = AppData {
        settings: Mutex::new(settings),
        db_path: db_path.clone(),
        logger: logger.clone(),
        app_started_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    let trace_id = format!("e2e-audio-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let file_name = epub
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("narration")
        .to_string();
    let result = generate_audio_from_text(
        &data,
        narration,
        output_dir.to_string_lossy().into_owned(),
        file_name,
        Some(trace_id.clone()),
        epub.to_string_lossy().into_owned(),
        None,
    )
    .await?;
    if let Ok(connection) = Connection::open(&db_path) {
        ensure_material_tasks_table(&connection)?;
        let _ = update_material_task_audio_status(
            &connection,
            epub.to_string_lossy().as_ref(),
            "success",
            100,
            Some(&result.output_dir),
            Some(&result.audio_file),
            result.duration_ms.map(|value| value.min(i64::MAX as u64) as i64),
            Some(result.chunks.min(i64::MAX as usize) as i64),
            Some("Audio generated."),
        );
    }
    println!(
        "Audio generated: trace_id={} file={} duration_ms={:?} chunks={}",
        trace_id, result.audio_file, result.duration_ms, result.chunks
    );
    Ok(())
}

fn sanitize_persisted_settings(settings: &mut AppSettings) -> bool {
    let mut changed = false;
    if settings.tool_profile.ffmpeg_path.trim().is_empty() {
        settings.tool_profile.ffmpeg_path = r"D:\03_Dev\ffmpeg\bin\ffmpeg.exe".to_string();
        changed = true;
    }
    if looks_like_garbled_text(&settings.feishu_profile.test_message) {
        settings.feishu_profile.test_message = "听书素材生成工具飞书连通性测试成功。".to_string();
        changed = true;
    }
    if looks_like_garbled_text(&settings.material_profile.channel_name) {
        settings.material_profile.channel_name = "半小时听完一本书".to_string();
        changed = true;
    }
    if looks_like_garbled_text(&settings.material_profile.category_name) {
        settings.material_profile.category_name = "半小时听完一本书".to_string();
        changed = true;
    }
    if settings
        .material_profile
        .categories
        .iter()
        .any(|value| looks_like_garbled_text(value))
    {
        settings.material_profile.categories = vec![
            "半小时听完一本书".to_string(),
            "睡前听完一本书".to_string(),
            "A Book in 30 Minutes".to_string(),
        ];
        changed = true;
    }
    if looks_like_garbled_text(&settings.material_profile.extra_direction) {
        settings.material_profile.extra_direction =
            "睡前听书风格，温柔、克制、有陪伴感。旁白目标为 30-35 分钟语音，最佳落在 7500~7800 个中文字；标题和简介服务于 YouTube 中文频道。"
                .to_string();
        changed = true;
    }
    changed
}

fn looks_like_garbled_text(value: &str) -> bool {
    value.contains('\u{fffd}')
        || value.contains("\u{003f}\u{003f}\u{003f}")
        || value.contains("\u{9357}")
        || value.contains("\u{95c2}")
        || value.contains("\u{7035}")
        || value.contains("\u{942b}")
        || value.contains("\u{95c1}")
        || value.contains("\u{95bb}")
        || value.contains("\u{9207}")
        || value.contains("\u{951f}\u{fffd}")
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
}

#[tauri::command]
pub fn get_app_state(data: State<'_, AppData>) -> Result<AppStatePayload, CommandError> {
    data.logger
        .info("app", "get_app_state", "已读取应用状态。");
    Ok(AppStatePayload {
        settings: data.settings.lock().map_err(lock_error)?.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
pub fn get_settings(data: State<'_, AppData>) -> Result<AppSettings, CommandError> {
    data.logger.info("settings", "get", "已读取配置。");
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
        .info("ai", "generate_text", "开始生成 AI 文本。");
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
                "AI 文本生成失败。",
                &error.message,
            );
            return Err(error);
        }
    };
    data.logger
        .info("ai", "generate_text", "AI 文本生成成功。");
    let model = current_ai_model(&settings);
    Ok(AiGenerateResult {
        content,
        model,
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
        .info("feishu", "send_message", "开始发送飞书消息。");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    match call_feishu(&settings, &request.text).await {
        Ok(result) => {
            data.logger.info(
                "feishu",
                "send_message",
                "飞书消息发送成功。",
            );
            Ok(result)
        }
        Err(error) => {
            data.logger.error(
                "feishu",
                "send_message",
                "飞书消息发送失败。",
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
        "正在准备素材生成任务",
    );
    data.logger.trace_info(
        "materials",
        "generate.start",
        "Start generating book materials.",
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
        "Loaded AI settings for material generation.",
        ai_profile_debug_detail(&settings),
        &trace_id,
    );
    let epub_path = Path::new(request.epub_path.trim());
    if request.epub_path.trim().is_empty() {
        data.logger.trace_error(
            "materials",
            "generate.validate",
            "Source path is empty.",
            "Please choose an EPUB file before generating materials.",
            &trace_id,
        );
        return Err(command_error(
            "Please choose an EPUB file before generating materials.",
        ));
    }
    if !epub_path.exists() {
        data.logger.trace_error(
            "materials",
            "generate.validate",
            "Source file does not exist.",
            request.epub_path.trim(),
            &trace_id,
        );
        return Err(command_error(
            "Source file does not exist. Please check the EPUB path.",
        ));
    }
    data.logger.debug(
        "materials",
        "source.file",
        "Source file resolved.",
        source_file_detail(epub_path),
        &trace_id,
    );

    let read_started = Instant::now();
    emit_material_progress(
        &app,
        &trace_id,
        request.epub_path.trim(),
        1,
        "Reading source book.",
    );
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        request.epub_path.trim(),
        "A-01",
        "文本：解析书籍",
        "generating",
        10,
        "正在读取并解析 EPUB 源文件。",
    );
    data.logger.trace_info(
        "materials",
        "source.read",
        "Reading source book.",
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
                "Source book read successfully.",
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
                "Source book read successfully. Preparing AI prompt.",
            );
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                request.epub_path.trim(),
                "A-01",
                "文本：解析书籍",
                "success",
                100,
                "EPUB 解析完成，已提取章节和基础信息。",
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
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                request.epub_path.trim(),
                "A-01",
                "文本：解析书籍",
                "failed",
                10,
                &error.message,
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
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                request.epub_path.trim(),
                "A-01",
                "文本：解析书籍",
                "failed",
                10,
                &message,
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
                "Source book read timed out.",
                detail,
                &trace_id,
            );
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                request.epub_path.trim(),
                "A-01",
                "文本：解析书籍",
                "failed",
                10,
                "EPUB 解析超时。",
            );
            return Err(command_error(format!(
                "Source book read timed out after {} seconds. Please check whether the EPUB is too large or malformed.",
                SOURCE_READ_TIMEOUT_SECONDS
            )));
        }
    };
    let usable_source_chars = usable_source_han_chars(&book);
    if usable_source_chars < SOURCE_MIN_USABLE_HAN_CHARS {
        let detail = format!(
            "usable_source_han_chars={} minimum={} chapters={}",
            usable_source_chars,
            SOURCE_MIN_USABLE_HAN_CHARS,
            book.chapters.len()
        );
        data.logger.trace_error(
            "materials",
            "source.content_insufficient",
            "Source book has insufficient readable正文 content.",
            &detail,
            &trace_id,
        );
        upsert_material_task_step_db(
            &data.db_path,
            &trace_id,
            request.epub_path.trim(),
            "A-01",
            "文本：解析书籍",
            "failed",
            25,
            "正文内容不足，疑似只有版权页，未继续调用 AI。",
        );
        return Err(command_error(
            "源文件没有足够的正文内容，疑似只有版权页，无法生成可靠旁白。",
        ));
    }
    let prompt = build_book_materials_prompt(&book, &request);
    update_material_task_progress_db(
        &data.db_path,
        request.epub_path.trim(),
        "generating",
        35,
        "AI prompt built. Waiting to request material JSON.",
    );
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        request.epub_path.trim(),
        "A-02",
        "文本：标题简介标签",
        "generating",
        25,
        "已构建 AI 提示词，准备生成标题、简介和标签。",
    );
    data.logger.debug(
        "materials",
        "prompt.build",
        "AI prompt built.",
        format!(
            "prompt_chars={} prompt_han_chars={} prompt_preview={}",
            prompt.chars().count(),
            count_han_chars(&prompt),
            text_preview(&prompt, 360)
        ),
        &trace_id,
    );
    let system_prompt =
        "You are writing a Chinese audiobook video package. Return only valid JSON with keys: videoTitle, description, tags, narration. The narration must be a continuous Chinese script. Do not output markdown."
            .to_string();
    let ai_started = Instant::now();
    emit_material_progress(
        &app,
        &trace_id,
        request.epub_path.trim(),
        2,
        "Requesting AI material generation.",
    );
    update_material_task_progress_db(
        &data.db_path,
        request.epub_path.trim(),
        "generating",
        45,
        "Requesting AI material generation JSON.",
    );
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        request.epub_path.trim(),
        "A-02",
        "文本：标题简介标签",
        "generating",
        45,
        "正在请求 AI 生成素材 JSON。",
    );
    data.logger.trace_info(
        "materials",
        "ai.request",
        "Requesting AI material generation JSON.",
        format!(
            "model={} messages=2 system_prompt_chars={} user_prompt_chars={} base_url={}",
            current_ai_model(&settings),
            system_prompt.chars().count(),
            prompt.chars().count(),
            current_ai_base_url(&settings)
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
                "AI material generation response received.",
                format!(
                    "elapsed_ms={} response_chars={} response_han_chars={} response_preview={}",
                    ai_started.elapsed().as_millis(),
                    content.chars().count(),
                    count_han_chars(&content),
                    text_preview(&content, 300)
                ),
                &trace_id,
            );
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                request.epub_path.trim(),
                "A-02",
                "文本：标题简介标签",
                "generating",
                70,
                "AI 已返回素材 JSON，正在解析。",
            );
            content
        }
        Err(error) => {
            let detail = format!("AI 素材请求失败：{}", error.message);
            update_material_task_progress_db(
                &data.db_path,
                request.epub_path.trim(),
                "failed",
                0,
                &detail,
            );
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                request.epub_path.trim(),
                "A-02",
                "文本：标题简介标签",
                "failed",
                45,
                &detail,
            );
            data.logger.trace_error(
                "materials",
                "ai.request.failed",
                "AI material generation request failed.",
                &error.message,
                &trace_id,
            );
            return Err(command_error(detail));
        }
    };

    let mut payload = if content.trim().is_empty() {
        data.logger.trace_error(
            "materials",
            "ai.initial.empty",
            "AI returned no usable material content.",
            "AI response was empty.",
            &trace_id,
        );
        upsert_material_task_step_db(
            &data.db_path,
            &trace_id,
            request.epub_path.trim(),
            "A-02",
            "文本：标题简介标签",
            "failed",
            70,
            "AI 未返回可用素材内容。",
        );
        return Err(command_error("AI returned no usable material content."));
    } else {
        match parse_book_materials_payload(&content) {
            Ok(payload) => {
                data.logger.debug(
                    "materials",
                    "ai.parse",
                    "AI material JSON parsed.",
                    format!(
                        "title={} description_chars={} tags={} narration_han_chars={}",
                        payload.video_title,
                        payload.description.chars().count(),
                        payload.tags.len(),
                        count_han_chars(&payload.narration)
                    ),
                    &trace_id,
                );
                upsert_material_task_step_db(
                    &data.db_path,
                    &trace_id,
                    request.epub_path.trim(),
                    "A-02",
                    "文本：标题简介标签",
                    "success",
                    100,
                    "标题、简介和标签已解析完成。",
                );
                upsert_material_task_step_db(
                    &data.db_path,
                    &trace_id,
                    request.epub_path.trim(),
                    "A-03",
                    "文本：旁白文稿",
                    "generating",
                    35,
                    "正在检查旁白长度并按目标字数修复。",
                );
                let mut payload = payload;
                payload.narration = sanitize_generated_narration(&payload.narration);
                payload
            }
            Err(error) => {
                data.logger.trace_error(
                    "materials",
                    "ai.parse.failed",
                    "AI material JSON parse failed.",
                    &error.message,
                    &trace_id,
                );
                upsert_material_task_step_db(
                    &data.db_path,
                    &trace_id,
                    request.epub_path.trim(),
                    "A-02",
                    "文本：标题简介标签",
                    "failed",
                    70,
                    "AI 素材 JSON 解析失败。",
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
                "Narration length is within target range.",
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
        if narration_chars > max_chars
            && narration_chars <= max_chars.saturating_add(NARRATION_LOCAL_TRIM_TOLERANCE)
        {
            payload.narration = trim_narration_to_han_chars(&payload.narration, max_chars);
            payload.narration = sanitize_generated_narration(&payload.narration);
            data.logger.trace_info(
                "materials",
                "ai.repair.local_trim",
                "Narration was slightly over target and was trimmed locally.",
                format!(
                    "before_han_chars={} after_han_chars={} target={}..{}",
                    narration_chars,
                    count_han_chars(&payload.narration),
                    min_chars,
                    max_chars
                ),
                &trace_id,
            );
            continue;
        }
        data.logger.warn(
            "materials",
            "ai.repair.required",
            "Narration length repair is required.",
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
            "Requesting AI narration repair.",
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
                "AI narration repair response received.",
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
                    payload.narration = sanitize_generated_narration(&payload.narration);
                    data.logger.trace_info(
                        "materials",
                        "ai.repair.done",
                        "Narration extension merged.",
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
                        "Narration repair response could not be parsed.",
                        "Repair response did not contain usable narration text.",
                        &trace_id,
                    );
                    break;
                }
            } else if let Ok(mut next_payload) = parse_book_materials_payload(&repaired) {
                next_payload.narration = sanitize_generated_narration(&next_payload.narration);
                data.logger.trace_info(
                    "materials",
                    "ai.repair.done",
                    "Narration repair completed.",
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
                    "Narration repair JSON parse failed.",
                    "Repair response was not valid material JSON.",
                    &trace_id,
                );
                break;
            }
        } else {
            data.logger.warn(
                "materials",
                "ai.repair.failed",
                "AI narration repair request failed.",
                "Keeping current narration and continuing with fallback if needed.",
                &trace_id,
            );
            break;
        }
    }
    let before_fallback_chars = count_han_chars(&payload.narration);
    if before_fallback_chars < min_chars {
        data.logger.warn(
            "materials",
            "ai.repair.local_fallback_disabled",
            "Local narration padding is disabled to avoid repetitive filler.",
            format!(
                "before_han_chars={} target={}..{}",
                before_fallback_chars, min_chars, max_chars
            ),
            &trace_id,
        );
    }
    let max_total_chars = max_chars.saturating_add(1800).min(10000);
    if payload.narration.chars().count() > max_total_chars {
        payload.narration = trim_narration_to_total_chars(&payload.narration, max_total_chars);
        payload.narration = sanitize_generated_narration(&payload.narration);
        data.logger.warn(
            "materials",
            "ai.repair.total_length_trimmed",
            "Narration total length exceeded visual text target and was trimmed at a sentence boundary.",
            format!(
                "max_total_chars={} after_total_chars={} after_han_chars={}",
                max_total_chars,
                payload.narration.chars().count(),
                count_han_chars(&payload.narration)
            ),
            &trace_id,
        );
    }
    let final_narration_chars = count_han_chars(&payload.narration);
    if final_narration_chars < min_chars || final_narration_chars > max_chars {
        data.logger.trace_error(
            "materials",
            "ai.repair.out_of_range",
            "Narration length is still outside target range.",
            &format!(
                "final_han_chars={} target={}..{} title={}",
                final_narration_chars, min_chars, max_chars, payload.video_title
            ),
            &trace_id,
        );
        upsert_material_task_step_db(
            &data.db_path,
            &trace_id,
            request.epub_path.trim(),
            "A-03",
            "文本：旁白文稿",
            "failed",
            70,
            "旁白字数仍不在目标范围内。",
        );
        return Err(command_error(format!(
            "AI generated narration length is {} Chinese characters, outside target range {}-{}. Please adjust settings or retry.",
            final_narration_chars, min_chars, max_chars
        )));
    }
    emit_material_progress(
        &app,
        &trace_id,
        request.epub_path.trim(),
        3,
        "Splitting narration into subtitles.",
    );
    let mut subtitles = split_subtitles(&payload.narration);
    if subtitles_need_ai_rewrite(&subtitles, &payload.narration)
        && ai_subtitle_rewrite_enabled()
    {
        if let Some(rewritten) = rewrite_subtitles_with_ai(&settings, &payload.narration, &subtitles).await {
            subtitles = rewritten;
        }
    }
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        request.epub_path.trim(),
        "A-03",
        "文本：旁白文稿",
        "success",
        100,
        "旁白文稿已生成并切分字幕文本。",
    );
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        request.epub_path.trim(),
        "A-04",
        "文本：保存素材包",
        "generating",
        60,
        "正在写入素材包文件。",
    );
    data.logger.trace_info(
        "materials",
        "subtitle.split",
        "Subtitles split from narration.",
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
        model: current_ai_model(&settings),
        overview: book.overview,
    };
    data.logger.trace_info(
        "materials",
        "generate.done",
        "Book materials generated.",
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
                "Book materials exported.",
                format!(
                    "files={} output_dir={}",
                    result.files.len(),
                    result.output_dir
                ),
                &trace_id,
            );
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                request.epub_path.trim(),
                "A-04",
                "文本：保存素材包",
                "success",
                100,
                &format!("素材包已保存：{}", result.output_dir),
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
                "Book materials export failed.",
                &error.message,
                &trace_id,
            );
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                request.epub_path.trim(),
                "A-04",
                "文本：保存素材包",
                "failed",
                60,
                &error.message,
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
        "Material generation completed.",
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
            "开始扫描素材路径：{}",
            request.path.trim()
        ),
    );
    let input = request.path.trim();
    if input.is_empty() {
        return Err(command_error(
            "请先选择素材文件或目录。",
        ));
    }
    let path = PathBuf::from(input);
    if !path.exists() {
        return Err(command_error(
            "素材路径不存在，请检查后重试。",
        ));
    }

    let directory = if path.is_dir() {
        path
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| command_error("无法解析素材所在目录。"))?
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
            image_status: "pending".to_string(),
            image_progress: 0,
            image_output_dir: None,
            image_message: String::new(),
            subtitle_status: "pending".to_string(),
            subtitle_progress: 0,
            subtitle_file: None,
            subtitle_message: String::new(),
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
            "Operation progress: {} {} {} {} {}",
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
            "打开素材任务数据库失败：{error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    let category = request.category.unwrap_or_default().trim().to_string();
    let mut files = if category.is_empty() {
        let mut statement = connection
            .prepare(
                &format!("SELECT {MATERIAL_TASK_SELECT_COLUMNS} FROM material_tasks ORDER BY updated_at DESC, name ASC")
            )
            .map_err(|error| command_error(format!("Operation failed: {error}")))?;
        let rows = statement
            .query_map([], material_task_from_row)
            .map_err(|error| {
                command_error(format!("Operation failed: {error}"))
            })?;
        collect_material_tasks(rows)?
    } else {
        let mut statement = connection
            .prepare(
                &format!(
                    "SELECT {MATERIAL_TASK_SELECT_COLUMNS} FROM material_tasks WHERE category = ?1 ORDER BY updated_at DESC, name ASC"
                )
            )
            .map_err(|error| command_error(format!("Operation failed: {error}")))?;
        let rows = statement
            .query_map(params![category], material_task_from_row)
            .map_err(|error| {
                command_error(format!("Operation failed: {error}"))
            })?;
        collect_material_tasks(rows)?
    };
    files.retain(|file| Path::new(&file.path).exists());
    for file in &mut files {
        let _ = migrate_task_outputs_to_source_output(&connection, file);
        let before = file.clone();
        normalize_material_task_outputs(file);
        persist_reconciled_task_status(&connection, &before, file);
    }
    Ok(ScanMaterialFilesResult {
        directory: String::new(),
        files,
    })
}

#[tauri::command]
pub fn get_material_task(
    data: State<'_, AppData>,
    request: MaterialTaskPathRequest,
) -> Result<Option<MaterialFile>, CommandError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Ok(None);
    }
    let connection = Connection::open(&data.db_path)
        .map_err(|error| command_error(format!("Open material task database failed: {error}")))?;
    ensure_material_tasks_table(&connection)?;
    let mut file = load_material_task_by_path(&connection, path)?;
    if let Some(file) = file.as_mut() {
        let _ = migrate_task_outputs_to_source_output(&connection, file);
        let before = file.clone();
        normalize_material_task_outputs(file);
        persist_reconciled_task_status(&connection, &before, file);
    }
    Ok(file)
}

#[tauri::command]
pub fn update_material_task_status(
    data: State<'_, AppData>,
    request: UpdateMaterialTaskStatusRequest,
) -> Result<MaterialFile, CommandError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(command_error("Operation completed."));
    }
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "Operation failed: {error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    let progress = clamp_task_progress(request.progress);
    let status = normalize_task_status(&request.status);
    let message = request.message.unwrap_or_default();
    let category = normalize_material_category(
        request
            .category
            .as_deref()
            .unwrap_or(DEFAULT_MATERIAL_CATEGORY),
    );
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
                category = ?7,
                updated_at = ?8
            WHERE path = ?1
            "#,
            params![
                path,
                status,
                progress,
                request.narration_chars,
                request.material_output_dir,
                message,
                category,
                now
            ],
        )
        .map_err(|error| command_error(format!("Update material task status failed: {error}")))?;
    if connection.changes() == 0 {
        upsert_material_task(&connection, &material_file_from_path(path, &category)?)?;
        connection
            .execute(
                r#"
                UPDATE material_tasks
                SET status = ?2,
                    progress = ?3,
                    narration_chars = ?4,
                    material_output_dir = COALESCE(?5, material_output_dir),
                    message = ?6,
                    category = ?7,
                    updated_at = ?8
                WHERE path = ?1
                "#,
                params![
                    path,
                    status,
                    progress,
                    request.narration_chars,
                    request.material_output_dir,
                    message,
                    category,
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
        format!("Material task status updated: path={path} status={status} progress={progress}"),
    );
    load_material_task_by_path(&connection, path)?
        .ok_or_else(|| command_error("Material task was not found after status update."))
}

#[tauri::command]
pub fn update_material_task_stage_status(
    data: State<'_, AppData>,
    request: UpdateMaterialTaskStageStatusRequest,
) -> Result<MaterialFile, CommandError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(command_error("请先选择 EPUB 任务。"));
    }
    let stage = request.stage.trim().to_ascii_lowercase();
    let status = normalize_task_status(&request.status);
    let progress = clamp_task_progress(request.progress);
    let message = request.message.unwrap_or_default();
    let connection = Connection::open(&data.db_path)
        .map_err(|error| command_error(format!("Open material task database failed: {error}")))?;
    ensure_material_tasks_table(&connection)?;
    if connection
        .query_row(
            "SELECT 1 FROM material_tasks WHERE path = ?1 LIMIT 1",
            params![path],
            |_| Ok(()),
        )
        .is_err()
    {
        upsert_material_task(&connection, &material_file_from_path(path, DEFAULT_MATERIAL_CATEGORY)?)?;
    }
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match stage.as_str() {
        "audio" => {
            connection
                .execute(
                    "UPDATE material_tasks
                     SET audio_status = ?2, audio_progress = ?3, audio_file = ?4,
                         audio_message = ?5, updated_at = ?6
                     WHERE path = ?1",
                    params![path, status, progress, request.output_path, message, now],
                )
                .map_err(|error| command_error(format!("Update audio task status failed: {error}")))?;
        }
        "image" => {
            connection
                .execute(
                    "UPDATE material_tasks
                     SET image_status = ?2, image_progress = ?3, image_output_dir = ?4,
                         image_message = ?5, updated_at = ?6
                     WHERE path = ?1",
                    params![path, status, progress, request.output_path, message, now],
                )
                .map_err(|error| command_error(format!("Update image task status failed: {error}")))?;
        }
        "subtitle" => {
            connection
                .execute(
                    "UPDATE material_tasks
                     SET subtitle_status = ?2, subtitle_progress = ?3, subtitle_file = ?4,
                         subtitle_message = ?5, updated_at = ?6
                     WHERE path = ?1",
                    params![path, status, progress, request.output_path, message, now],
                )
                .map_err(|error| command_error(format!("Update subtitle task status failed: {error}")))?;
        }
        _ => {
            connection
                .execute(
                    "UPDATE material_tasks
                     SET video_status = ?2, video_progress = ?3, video_file = ?4,
                         video_message = ?5, updated_at = ?6
                     WHERE path = ?1",
                    params![path, status, progress, request.output_path, message, now],
                )
                .map_err(|error| command_error(format!("Update video task status failed: {error}")))?;
        }
    }
    load_material_task_by_path(&connection, path)?
        .ok_or_else(|| command_error("Material task was not found after stage status update."))
}

#[tauri::command]
pub fn remove_material_task(
    data: State<'_, AppData>,
    request: MaterialTaskPathRequest,
) -> Result<bool, CommandError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(command_error("Operation completed."));
    }
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "Operation failed: {error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    connection
        .execute("DELETE FROM material_tasks WHERE path = ?1", params![path])
        .map_err(|error| command_error(format!("Remove material task failed: {error}")))?;
    data.logger.info(
        "materials",
        "tasks.remove",
        format!("Task path: {path}"),
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
            "Operation failed: {error}"
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
                "Output directory operation failed.",
                params![path, now],
            )
            .map_err(|error| command_error(format!("重置素材任务失败：{error}")))?;
        data.logger.info(
            "materials",
            "tasks.reset",
            format!("已重置素材任务：path={path}"),
        );
    } else {
        connection
            .execute(
                "Output directory operation failed.",
                params![now],
            )
            .map_err(|error| command_error(format!("Operation failed: {error}")))?;
        data.logger.info(
            "materials",
            "tasks.reset_all",
            "Operation completed.",
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
        "YouTube publish material operation completed.",
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
        "YouTube publish material operation completed.",
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
        return Err(command_error("Operation completed."));
    }
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "Operation failed: {error}"
        ))
    })?;
    ensure_material_tasks_table(&connection)?;
    let file = load_material_task_by_path(&connection, path)?
        .ok_or_else(|| command_error("Operation completed."))?;
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
            return Err(command_error("Operation completed."));
        }
    } else if let Some(found) = find_existing_material_output_dir(&data, &file)? {
        let output_dir = found.to_string_lossy().into_owned();
        update_material_task_output_dir(&connection, path, &output_dir)?;
        found
    } else {
        return Err(command_error(
            "Operation completed.",
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
        "Output directory operation failed.",
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
        return Err(command_error("Operation completed."));
    }
    let trace_id = build_audio_trace_id(request.trace_id.as_deref());
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!(
            "Operation failed: {error}"
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
            return Err(command_error("Operation completed."));
        }
    } else if let Some(found) = find_existing_material_output_dir(&data, &file)? {
        update_material_task_output_dir(&connection, path, &found.to_string_lossy())?;
        file.material_output_dir = Some(found.to_string_lossy().into_owned());
        found
    } else {
        return Err(command_error("Operation completed."));
    };
    let narration_file = staged_material_path(&material_dir, "narration.txt");
    if !narration_file.exists() {
        return Err(command_error(
            "Narration file operation failed.",
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
        Some("正在读取旁白文件。"),
    )?;
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        path,
        "B-01",
        "音频：读取旁白",
        "generating",
        10,
        &format!("正在读取旁白文件：{}", narration_file.to_string_lossy()),
    );
    let text = fs::read_to_string(&narration_file)
        .map_err(|error| command_error(format!("Narration file operation failed: {error}")))?;
    update_material_task_audio_status(
        &connection,
        path,
        "generating",
        20,
        None,
        None,
        None,
        None,
        Some("旁白已读取，正在拆分音频片段。"),
    )?;
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        path,
        "B-01",
        "音频：读取旁白",
        "success",
        100,
        &format!("旁白已读取，字符数：{}", text.chars().count()),
    );
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        path,
        "B-02",
        "音频：拆分片段",
        "generating",
        20,
        "正在按语音服务限制拆分旁白文本。",
    );
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
        Some("音频片段拆分完成，正在生成语音。"),
    )?;
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        path,
        "B-02",
        "音频：拆分片段",
        "success",
        100,
        "旁白文本已拆分，准备请求语音服务。",
    );
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        path,
        "B-03",
        "音频：生成语音",
        "generating",
        30,
        "正在请求语音服务生成分段 mp3。",
    );
    let audio_progress = AudioTaskProgress {
        db_path: data.db_path.clone(),
        path: path.to_string(),
        trace_id: trace_id.clone(),
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
            upsert_material_task_step_db(
                &data.db_path,
                &trace_id,
                path,
                "B-03",
                "音频：生成语音",
                "failed",
                0,
                &error.message,
            );
            data.logger.trace_error(
                "audio",
                "task.failed",
                "音频任务失败。",
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
        Some("音频已生成。"),
    )?;
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        path,
        "B-03",
        "音频：生成语音",
        "success",
        100,
        &format!("已生成 {} 个音频片段。", result.chunks),
    );
    upsert_material_task_step_db(
        &data.db_path,
        &trace_id,
        path,
        "B-04",
        "音频：合成音频",
        "success",
        100,
        &format!("最终音频：{}", result.audio_file),
    );
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
        "Material task audio status updated.",
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
        return Err(command_error("请先选择 EPUB 文件。"));
    }
    let epub = PathBuf::from(epub_path);
    if !epub.exists() {
        return Err(command_error(format!(
            "EPUB file does not exist: {epub_path}"
        )));
    }
    let (pipeline_root, script) = find_video_pipeline(&app)?;
    let python = find_python_command();
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!("Open material task database failed: {error}"))
    })?;
    ensure_material_tasks_table(&connection)?;
    upsert_material_task(
        &connection,
        &material_file_from_path(epub_path, DEFAULT_MATERIAL_CATEGORY)?,
    )?;
    let pipeline_stage = normalize_video_pipeline_stage(request.pipeline_stage.as_deref());
    let app_material_dir = resolve_task_material_dir_for_video(&data.db_path, epub_path);
    if pipeline_stage == "image" && !has_aligned_chinese_srt(app_material_dir.as_deref()) {
        let message = "图片阶段必须在字幕阶段之后执行。请先生成音频和字幕，确认已产出中文字幕 SRT 后再生成图片。";
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = connection.execute(
            "UPDATE material_tasks
             SET image_status = 'failed', image_progress = 0, image_message = ?2,
                 video_status = CASE WHEN video_status = 'generating' THEN 'failed' ELSE video_status END,
                 video_progress = CASE WHEN video_status = 'generating' THEN 0 ELSE video_progress END,
                 video_message = CASE WHEN video_status = 'generating' THEN '前置图片阶段失败，视频流水线已终止。' ELSE video_message END,
                 updated_at = ?3
             WHERE path = ?1",
            params![epub_path, message, now],
        );
        return Err(command_error(message));
    }
    let log_module = video_pipeline_stage_log_module(&pipeline_stage);
    let pipeline_label = video_pipeline_stage_pipeline_label(&pipeline_stage);
    data.logger.trace_info(
        log_module,
        "pipeline.spawn",
        &format!("{pipeline_label}后台任务已创建"),
        format!(
            "阶段={} epub={} 输出目录={} script={} python={}",
            video_pipeline_stage_label(&pipeline_stage),
            epub.to_string_lossy(),
            app_material_dir
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| "自动推断".to_string()),
            script.to_string_lossy(),
            python
        ),
        &trace_id,
    );
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    match pipeline_stage.as_str() {
        "image" => {
            let _ = connection.execute(
                "UPDATE material_tasks SET image_status = 'generating', image_progress = 0, image_output_dir = NULL, image_message = '正在生成图片素材', updated_at = ?2 WHERE path = ?1",
                params![epub_path, now],
            );
            reset_image_task_steps_for_trace(&data.db_path, &trace_id, epub_path);
        }
        "subtitle" => {
            let _ = connection.execute(
                "UPDATE material_tasks SET subtitle_status = 'generating', subtitle_progress = 0, subtitle_file = NULL, subtitle_message = '正在生成字幕文件', updated_at = ?2 WHERE path = ?1",
                params![epub_path, now],
            );
        }
        _ => {
            let _ = connection.execute(
                "UPDATE material_tasks SET video_status = 'generating', video_progress = 20, video_message = '正在生成视频', updated_at = ?2 WHERE path = ?1",
                params![epub_path, now],
            );
        }
    }

    let db_path = data.db_path.clone();
    let logger = data.logger.clone();
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let epub_path_owned = epub_path.to_string();
    let allow_placeholder_visuals = request.allow_placeholder_visuals.unwrap_or(false);
    let controlled_programmatic_visuals = request.controlled_programmatic_visuals.unwrap_or(true);
    let ignore_existing_visual_assets = request.ignore_existing_visual_assets.unwrap_or(true);
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
            settings,
            allow_placeholder_visuals,
            controlled_programmatic_visuals,
            ignore_existing_visual_assets,
            pipeline_stage,
        );
    });

    Ok(GenerateBookVideoResult {
        material_dir: String::new(),
        pipeline_manifest: String::new(),
        cover: None,
        visual_story_plan: None,
        visual_timeline: None,
        no_subtitle_video: None,
        hard_subtitle_video: None,
        hard_subtitle_manifest: None,
        elapsed_seconds: 0.0,
    })
}

fn normalize_video_pipeline_stage(stage: Option<&str>) -> String {
    match stage.unwrap_or("video").trim().to_ascii_lowercase().as_str() {
        "image" => "image".to_string(),
        "subtitle" => "subtitle".to_string(),
        _ => "video".to_string(),
    }
}

fn has_aligned_chinese_srt(dir: Option<&Path>) -> bool {
    let Some(dir) = dir else {
        return false;
    };
    [
        "hard_subtitle.aeneas.cmn.srt",
        "hard_subtitle.aeneas.chn.srt",
        "hard_subtitle.aeneas.zh.srt",
    ]
    .iter()
    .flat_map(|name| [dir.join(STAGE_SUBTITLES_DIR).join(name), dir.join(name)])
    .any(|path| path.is_file())
}

#[tauri::command]
pub fn generate_publish_materials(
    data: State<'_, AppData>,
    request: GeneratePublishMaterialsRequest,
) -> Result<GeneratePublishMaterialsResult, CommandError> {
    let trace_id = build_trace_id(request.trace_id.as_deref());
    let epub_path = request.epub_path.trim();
    if epub_path.is_empty() {
        return Err(command_error("请先选择 EPUB 任务。"));
    }
    let epub = PathBuf::from(epub_path);
    if !epub.exists() {
        return Err(command_error(format!("EPUB file does not exist: {epub_path}")));
    }
    let output_dir = resolve_publish_output_dir(&data.db_path, epub_path, &epub)?;
    let materials_path = staged_material_path(&output_dir, "materials.json");
    if !materials_path.exists() {
        return Err(command_error(format!(
            "找不到 materials.json，请先生成素材：{}",
            materials_path.to_string_lossy()
        )));
    }
    let materials_content = fs::read_to_string(&materials_path)
        .map_err(|error| command_error(format!("Read materials.json failed: {error}")))?;
    let materials_json: serde_json::Value = serde_json::from_str(&materials_content)
        .map_err(|error| command_error(format!("Parse materials.json failed: {error}")))?;
    let title = json_string(&materials_json, "videoTitle")
        .or_else(|| json_string(&materials_json, "title"))
        .unwrap_or_else(|| epub.file_stem().and_then(|value| value.to_str()).unwrap_or("book").to_string());
    let description = json_string(&materials_json, "description").unwrap_or_default();
    let tags = json_string_array(&materials_json, "tags");
    let publish_dir = output_dir.join(STAGE_PUBLISH_DIR);
    fs::create_dir_all(&publish_dir).map_err(|error| {
        command_error(format!("Operation failed: {error}"))
    })?;
    let markdown_file = publish_dir.join("youtube_publish.md");
    let srt_path = output_dir
        .join(STAGE_SUBTITLES_DIR)
        .join("hard_subtitle.aeneas.zh-en.srt");
    let chapters = build_publish_chapters(&srt_path);
    let video_path = output_dir
        .join(STAGE_VIDEO_DIR)
        .join(format!(
            "{}_中英双语字幕_精修版.mp4",
            sanitize_file_name(&extract_publish_book_title(&title))
        ));
    let markdown = build_youtube_publish_markdown(
        &title,
        &description,
        &tags,
        &chapters,
        &output_dir,
        &video_path,
    );
    fs::write(&markdown_file, markdown)
        .map_err(|error| command_error(format!("Write youtube_publish.md failed: {error}")))?;
    data.logger.trace_info(
        "publish",
        "materials.generated",
        "YouTube publish markdown generated",
        format!(
            "trace_id={} output={} chapters={} tags={}",
            trace_id,
            markdown_file.to_string_lossy(),
            chapters.len(),
            tags.len()
        ),
        &trace_id,
    );
    Ok(GeneratePublishMaterialsResult {
        output_dir: output_dir.to_string_lossy().into_owned(),
        markdown_file: markdown_file.to_string_lossy().into_owned(),
        title,
        chapters: chapters.len(),
        tags,
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
    settings: AppSettings,
    allow_placeholder_visuals: bool,
    controlled_programmatic_visuals: bool,
    ignore_existing_visual_assets: bool,
    pipeline_stage: String,
) {
    let started = Instant::now();
    let log_module = video_pipeline_stage_log_module(&pipeline_stage);
    let pipeline_label = video_pipeline_stage_pipeline_label(&pipeline_stage);
    logger.trace_info(
        log_module,
        "pipeline.start",
        &format!("{pipeline_label}开始执行"),
        format!(
            "阶段={} epub={} script={} python={}",
            video_pipeline_stage_label(&pipeline_stage),
            epub.to_string_lossy(),
            script.to_string_lossy(),
            python
        ),
        &trace_id,
    );
    update_visual_stage_after_background(
        &db_path,
        &epub_path,
        &pipeline_stage,
        "generating",
        video_pipeline_initial_progress(&pipeline_stage),
        None,
        None,
        None,
        None,
        &format!("Preparing {pipeline_label}"),
    );

    let mut command = Command::new(&python);
    let app_material_dir = resolve_task_material_dir_for_video(&db_path, &epub_path);
    let app_video_dir = app_material_dir.clone();
    command
        .current_dir(&pipeline_root)
        .env("PYTHONIOENCODING", "UTF-8")
        .env("ABOOK_AI_BASE_URL", settings.ai_profile.base_url.trim())
        .env("ABOOK_AI_API_KEY", settings.ai_profile.api_key.trim())
        .env("ABOOK_AI_MODEL", settings.ai_profile.model.trim())
        .env("ABOOK_SUBTITLE_SOURCE_LANGUAGE", "cmn")
        .env(
            "BOOK_IMAGE_BACKEND",
            settings.pipeline_profile.image_backend.trim(),
        )
        .arg(&script)
        .arg("--epub")
        .arg(&epub)
        .arg("--skip-notify")
        .arg("--audio-language")
        .arg("cmn");
    if let Some(video_dir) = app_video_dir.as_ref() {
        command.arg("--output-dir").arg(video_dir);
    }
    if let Some(music_file) = find_background_music_file(&settings) {
        command.arg("--background-music").arg(music_file);
    }
    if let Some(header_audio) = find_header_audio_file(&script) {
        command.arg("--header-audio").arg(header_audio);
    }
    if allow_placeholder_visuals {
        command.arg("--allow-placeholder-visuals");
    }
    if controlled_programmatic_visuals {
        command.arg("--controlled-programmatic-visuals");
    }
    if ignore_existing_visual_assets {
        command.arg("--ignore-existing-visual-assets");
    }
    if pipeline_stage == "image" {
        command.arg("--visual-assets-only");
    }
    if pipeline_stage == "subtitle" {
        command.arg("--audio-subtitle-only");
    }

    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let message = format!("Failed to launch video pipeline: {error}");
            logger.trace_error(
                log_module,
                "pipeline.failed",
                &format!("{pipeline_label}失败"),
                &message,
                &trace_id,
            );
            update_visual_stage_after_background(
            &db_path, &epub_path, &pipeline_stage, "failed", 0, None, None, None, None, &message,
            );
            return;
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_handle = stdout.map(|stream| {
        let logger = logger.clone();
        let trace_id = trace_id.clone();
        thread::spawn(move || collect_video_pipeline_stream(stream, logger, trace_id, "stdout"))
    });
    let stderr_handle = stderr.map(|stream| {
        let logger = logger.clone();
        let trace_id = trace_id.clone();
        let db_path = db_path.clone();
        let epub_path = epub_path.clone();
        thread::spawn(move || {
            collect_video_pipeline_progress_stream(stream, logger, trace_id, db_path, epub_path)
        })
    });
    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            let message = format!("Video pipeline wait failed: {error}");
            logger.trace_error(
                log_module,
                "pipeline.failed",
                &format!("{pipeline_label}失败"),
                &message,
                &trace_id,
            );
            update_visual_stage_after_background(
            &db_path, &epub_path, &pipeline_stage, "failed", 0, None, None, None, None, &message,
            );
            return;
        }
    };
    let stdout = stdout_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    update_visual_stage_after_background(
            &db_path,
        &epub_path,
        &pipeline_stage, "generating",
        75,
        None,
        None,
        None,
        None,
        "Video pipeline finished, parsing result",
    );
    if !status.success() {
        let detail = format!(
            "stdout:\n{}\n\nstderr:\n{}",
            text_preview(&stdout, 4000),
            text_preview(&stderr, 4000)
        );
        logger.trace_error(
            log_module,
            "pipeline.failed",
            &format!("{pipeline_label}失败"),
            &detail,
            &trace_id,
        );
        update_visual_stage_after_background(
            &db_path,
            &epub_path,
            &pipeline_stage, "failed",
            0,
            None,
            None,
            None,
            None,
            &text_preview(&(stderr + &stdout), 240),
        );
        if pipeline_stage == "image" {
            upsert_material_task_step_db(
                &db_path,
                &trace_id,
                &epub_path,
                "D-01",
                "图片：生成封面",
                "failed",
                0,
                "图片流水线执行失败。",
            );
        }
        return;
    }

    let json = match parse_last_json_object(&stdout) {
        Ok(json) => json,
        Err(error) => {
            logger.trace_error(
                log_module,
                "pipeline.parse_failed",
                &format!("{pipeline_label}结果解析失败"),
                &error.message,
                &trace_id,
            );
            update_visual_stage_after_background(
            &db_path,
                &epub_path,
                &pipeline_stage, "failed",
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
    update_visual_stage_after_background(
            &db_path,
        &epub_path,
        &pipeline_stage, "generating",
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
                log_module,
                "pipeline.parse_failed",
                &format!("{pipeline_label}结果解析失败"),
                message,
                &trace_id,
            );
            update_visual_stage_after_background(
            &db_path, &epub_path, &pipeline_stage, "failed", 0, None, None, None, None, message,
            );
            return;
        }
    };
    if pipeline_stage == "image" || json_bool(&json, "visualAssetsOnly").unwrap_or(false) {
        let visual_assets_dir = json_string(&json, "visualAssetsDir").or_else(|| json_string(&json, "visualStoryPlan"));
        let visual_asset_count = visual_assets_dir
            .as_deref()
            .and_then(|path| count_image_files_in_dir(Path::new(path)))
            .or_else(|| json_i64(&json, "visualAssetCount"))
            .unwrap_or(0);
        let cover = json_string(&json, "cover").unwrap_or_default();
        let material_dir_for_db = app_material_dir
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| material_dir.clone());
        update_visual_stage_after_background(
            &db_path,
            &epub_path,
            "image",
            "success",
            100,
            Some(&material_dir_for_db),
            visual_assets_dir.as_deref(),
            None,
            None,
            &format!("图片素材已生成：{} 张", visual_asset_count),
        );
        upsert_material_task_step_db(
            &db_path,
            &trace_id,
            &epub_path,
            "D-01",
            "图片：生成封面",
            "success",
            100,
            if cover.is_empty() { "封面随图片素材阶段完成。" } else { "封面已生成。" },
        );
        upsert_material_task_step_db(
            &db_path,
            &trace_id,
            &epub_path,
            "D-02",
            "图片：生成分镜图",
            "success",
            100,
            &format!("分镜图已生成：{} 张。", visual_asset_count),
        );
        logger.trace_info(
            "image",
            "pipeline.visual_done",
            "图片流水线完成",
            format!(
                "耗时={:.1}秒 素材目录={} 图片目录={} 图片数量={} 封面={}",
                started.elapsed().as_secs_f64(),
                material_dir,
                visual_assets_dir.unwrap_or_default(),
                visual_asset_count,
                cover
            ),
            &trace_id,
        );
        return;
    }
    if pipeline_stage == "subtitle" || json_bool(&json, "audioSubtitleOnly").unwrap_or(false) {
        let subtitle_file = json_string(&json, "hardSubtitleManifest")
            .or_else(|| json_string(&json, "hardSubtitleSrt"));
        let material_dir_for_db = app_material_dir
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| material_dir.clone());
        update_visual_stage_after_background(
            &db_path,
            &epub_path,
            "subtitle",
            "success",
            100,
            Some(&material_dir_for_db),
            subtitle_file.as_deref(),
            None,
            None,
            "SRT/ASS subtitles generated",
        );
        logger.trace_info(
            "video",
            "pipeline.subtitle_done",
            "Subtitle pipeline completed",
            format!(
                "elapsed_seconds={:.1} material_dir={} subtitle_file={}",
                started.elapsed().as_secs_f64(),
                material_dir,
                subtitle_file.unwrap_or_default()
            ),
            &trace_id,
        );
        return;
    }
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
        update_visual_stage_after_background(
            &db_path,
            &epub_path,
            &pipeline_stage, "failed",
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
    update_visual_stage_after_background(
            &db_path,
        &epub_path,
        &pipeline_stage, "success",
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
            "UPDATE material_tasks
             SET video_status = ?2, video_progress = ?3, material_output_dir = COALESCE(?4, material_output_dir),
                 video_file = ?5, video_duration_ms = ?6, video_file_size = ?7, video_message = ?8, updated_at = ?9
             WHERE path = ?1",
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

#[allow(clippy::too_many_arguments)]
fn update_visual_stage_after_background(
    db_path: &Path,
    epub_path: &str,
    stage: &str,
    status: &str,
    progress: i64,
    material_dir: Option<&str>,
    output_file: Option<&str>,
    video_duration_ms: Option<i64>,
    video_file_size: Option<i64>,
    message: &str,
) {
    if stage == "image" {
        if let Ok(connection) = Connection::open(db_path) {
            let _ = connection.execute(
                "UPDATE material_tasks
                 SET image_status = ?2, image_progress = ?3, material_output_dir = COALESCE(?4, material_output_dir),
                     image_output_dir = ?5, image_message = ?6, updated_at = ?7
                 WHERE path = ?1",
                params![
                    epub_path,
                    status,
                    progress,
                    material_dir,
                    output_file,
                    message,
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
                ],
            );
        }
        return;
    }
    if stage == "subtitle" {
        if let Ok(connection) = Connection::open(db_path) {
            let _ = connection.execute(
                "UPDATE material_tasks
                 SET subtitle_status = ?2, subtitle_progress = ?3, material_output_dir = COALESCE(?4, material_output_dir),
                     subtitle_file = ?5, subtitle_message = ?6, updated_at = ?7
                 WHERE path = ?1",
                params![
                    epub_path,
                    status,
                    progress,
                    material_dir,
                    output_file,
                    message,
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
                ],
            );
        }
        return;
    }
    update_video_task_after_background(
        db_path,
        epub_path,
        status,
        progress,
        material_dir,
        output_file,
        video_duration_ms,
        video_file_size,
        message,
    );
    if status == "success" {
        if let Ok(connection) = Connection::open(db_path) {
            let _ = connection.execute(
                "UPDATE material_tasks
                 SET status = 'success', progress = 100, material_output_dir = COALESCE(?2, material_output_dir),
                     message = '视频流水线已完成', updated_at = ?4
                 WHERE path = ?1",
                params![
                    epub_path,
                    material_dir,
                    output_file,
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
                ],
            );
        }
    }
}

fn collect_video_pipeline_stream<R: std::io::Read>(
    stream: R,
    logger: OperationLogger,
    trace_id: String,
    stream_name: &'static str,
) -> String {
    let mut collected = String::new();
    let reader = BufReader::new(stream);
    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        collected.push_str(&line);
        collected.push('\n');
        let action = format!("pipeline.{stream_name}");
        if stream_name == "stdout" {
            if let Some(summary) = summarize_video_pipeline_json_line(&line) {
                logger.trace_info(
                    "video",
                    "pipeline.result",
                    "视频流水线结果摘要",
                    summary,
                    &trace_id,
                );
                continue;
            }
        }
        logger.trace_info(
            "video",
            &action,
            "Video pipeline output",
            &line,
            &trace_id,
        );
    }
    collected
}

fn summarize_video_pipeline_json_line(line: &str) -> Option<String> {
    if !line.starts_with('{') || !line.ends_with('}') {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(line).ok()?;
    let material_dir = json_string(&json, "materialDir")?;
    let visual_assets_dir = json_string(&json, "visualAssetsDir");
    let cover = json_string(&json, "cover");
    let visual_count = visual_assets_dir
        .as_deref()
        .and_then(|path| count_image_files_in_dir(Path::new(path)))
        .or_else(|| json_i64(&json, "visualAssetCount"))
        .unwrap_or(0);
    let elapsed = json_f64(&json, "elapsedSeconds").unwrap_or(0.0);
    Some(format!(
        "素材目录={} 图片目录={} 图片数量={} 封面={} 字幕行数={} 模式={} 耗时={:.1}秒",
        material_dir,
        visual_assets_dir.unwrap_or_default(),
        visual_count,
        cover.unwrap_or_default(),
        json_i64(&json, "subtitleCount").unwrap_or(0),
        video_pipeline_result_mode(&json),
        elapsed
    ))
}

fn video_pipeline_result_mode(json: &serde_json::Value) -> &'static str {
    if json_bool(json, "visualAssetsOnly").unwrap_or(false) {
        return "仅图片素材";
    }
    if json_bool(json, "audioSubtitleOnly").unwrap_or(false) {
        return "仅音频字幕";
    }
    "完整视频"
}

fn video_pipeline_stage_label(stage: &str) -> &'static str {
    match stage {
        "image" => "图片",
        "subtitle" => "字幕",
        _ => "视频",
    }
}

fn video_pipeline_stage_pipeline_label(stage: &str) -> &'static str {
    match stage {
        "image" => "图片流水线",
        "subtitle" => "字幕流水线",
        _ => "视频流水线",
    }
}

fn video_pipeline_stage_log_module(stage: &str) -> &'static str {
    match stage {
        "image" => "image",
        "subtitle" => "subtitle",
        _ => "video",
    }
}

fn video_pipeline_initial_progress(stage: &str) -> i64 {
    match stage {
        "image" | "subtitle" => 0,
        _ => 45,
    }
}

fn reset_image_task_steps_for_trace(db_path: &Path, trace_id: &str, epub_path: &str) {
    upsert_material_task_step_db(
        db_path,
        trace_id,
        epub_path,
        "D-01",
        "图片：生成封面",
        "generating",
        0,
        "图片阶段已重新开始，正在准备封面和视觉素材。",
    );
    upsert_material_task_step_db(
        db_path,
        trace_id,
        epub_path,
        "D-02",
        "图片：生成分镜图",
        "pending",
        0,
        "等待图片生成服务返回分镜图。",
    );
}

fn count_image_files_in_dir(path: &Path) -> Option<i64> {
    let entries = fs::read_dir(path).ok()?;
    let count = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp"))
                .unwrap_or(false)
        })
        .count();
    Some(count.min(i64::MAX as usize) as i64)
}

fn collect_video_pipeline_progress_stream<R: std::io::Read>(
    stream: R,
    logger: OperationLogger,
    trace_id: String,
    db_path: PathBuf,
    epub_path: String,
) -> String {
    let mut collected = String::new();
    let reader = BufReader::new(stream);
    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        collected.push_str(&line);
        collected.push('\n');
        logger.trace_info(
            "video",
            "pipeline.stderr",
            "Video pipeline output",
            &line,
            &trace_id,
        );
        if let Some((done, total)) = parse_translated_subtitle_progress(&line) {
            let percent = if total > 0 {
                45 + ((done as f64 / total as f64) * 15.0).round() as i64
            } else {
                45
            }
            .clamp(45, 60);
            let message = format!("Translating bilingual subtitles: {done}/{total}");
            update_video_task_after_background(
                &db_path,
                &epub_path,
                "generating",
                percent,
                None,
                None,
                None,
                None,
                &message,
            );
        }
    }
    collected
}

fn parse_translated_subtitle_progress(line: &str) -> Option<(i64, i64)> {
    let regex = Regex::new(r"Translated subtitle cues\s+(\d+)/(\d+)").ok()?;
    let captures = regex.captures(line)?;
    let done = captures.get(1)?.as_str().parse::<i64>().ok()?;
    let total = captures.get(2)?.as_str().parse::<i64>().ok()?;
    Some((done, total))
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
            "SELECT material_output_dir FROM material_tasks WHERE path = ?1",
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

fn find_background_music_file(settings: &AppSettings) -> Option<PathBuf> {
    let configured = settings.tool_profile.background_music_path.trim();
    if !configured.is_empty() {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Some(path);
        }
    }
    let default_project_music = PathBuf::from(
        "D:\\04_GitHub\\world-cup-issue\\a-book-in-30-minutes\\music\\bf.mp3",
    );
    if default_project_music.is_file() {
        return Some(default_project_music);
    }
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

fn find_header_audio_file(script: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(parent) = script.parent() {
        candidates.push(parent.join("assets").join("header.mp3"));
        candidates.push(parent.join("header.mp3"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("assets").join("header.mp3"));
            candidates.push(parent.join("header.mp3"));
        }
    }
    candidates
        .into_iter()
        .find(|path| path.is_file() && path.metadata().map(|meta| meta.len() > 0).unwrap_or(false))
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
        "开始生成音频。",
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
            "音频文本为空。",
            "请先生成旁白文本，再生成音频。",
            &trace_id,
        );
        return Err(command_error(
            "音频文本为空，请先生成旁白文本。",
        ));
    }
    let chunks = split_speech_text(text, SPEECH_CHUNK_MAX_CHARS);
    if chunks.is_empty() {
        return Err(command_error(
            "音频文本拆分后为空，请检查旁白文本。",
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
    let output_dir = base_dir.join(STAGE_AUDIO_DIR);
    let parts_dir = output_dir.clone();
    let ssml_dir = output_dir.clone();
    fs::create_dir_all(&output_dir).map_err(|error| {
        command_error(format!(
            "Operation failed: {error}"
        ))
    })?;
    fs::create_dir_all(&parts_dir).map_err(|error| {
        command_error(format!(
            "Operation failed: {error}"
        ))
    })?;
    fs::create_dir_all(&ssml_dir)
        .map_err(|error| command_error(format!("SSML operation failed: {error}")))?;

    data.logger.debug(
        "audio",
        "generate.plan",
        "音频生成计划已创建。",
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
        callback.update(35, "音频片段已拆分，准备请求语音服务。");
        callback.step(
            "B-02",
            "音频：拆分片段",
            "success",
            100,
            &format!("已拆分为 {} 个音频片段。", chunks.len()),
        );
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
        .map_err(|error| command_error(format!("SSML operation failed: {error}")))?;

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
            command_error(format!("SSML operation failed: {error}"))
        })?;
        data.logger.trace_info(
            "audio",
            "speech.request",
            format!(
                "正在生成第 {}/{} 段语音。",
                index + 1,
                chunks.len()
            ),
            format!(
                "voice={} locale={} chunk_chars={} ssml_file={} output={}",
                settings.speech_profile.voice_name,
                if settings.speech_profile.locale.trim().is_empty() { "zh-CN" } else { settings.speech_profile.locale.trim() },
                chunk.chars().count(),
                part_ssml_file.to_string_lossy(),
                part_file.to_string_lossy()
            ),
            &trace_id,
        );
        if let Some(callback) = progress_callback.as_ref() {
            let progress = 40 + ((index as i64 * 45) / chunks.len().max(1) as i64);
            callback.step(
                "B-03",
                "音频：生成语音",
                "generating",
                progress.min(84),
                &format!("正在生成第 {}/{} 段语音。", index + 1, chunks.len()),
            );
        }
        let part_started = Instant::now();
        synthesize_speech_to_file(&settings.speech_profile, &ssml, &part_file)
            .await
            .map_err(|error| {
                data.logger.trace_error(
                    "audio",
                    "speech.request.failed",
                    format!("第 {} 段语音生成失败。", index + 1),
                    format!(
                        "{} ssml_file={}",
                        error.message,
                        part_ssml_file.to_string_lossy()
                    ),
                    &trace_id,
                );
                if let Some(callback) = progress_callback.as_ref() {
                    callback.step(
                        "B-03",
                        "音频：生成语音",
                        "failed",
                        0,
                        &format!(
                            "第 {}/{} 段语音生成失败：{}；SSML：{}",
                            index + 1,
                            chunks.len(),
                            error.message,
                            part_ssml_file.to_string_lossy()
                        ),
                    );
                }
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
                    "已生成第 {}/{} 段语音。",
                    index + 1,
                    chunks.len()
                ),
            );
            callback.step(
                "B-03",
                "音频：生成语音",
                "generating",
                progress.min(85),
                &format!("已生成第 {}/{} 段语音。", index + 1, chunks.len()),
            );
        }
        data.logger.trace_info(
            "audio",
            "speech.response",
            format!(
                "第 {}/{} 段语音生成完成。",
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
            command_error(format!("Operation failed: {error}"))
        })?;
    } else {
        if let Some(callback) = progress_callback.as_ref() {
            callback.update(88, "正在合成完整音频。");
            callback.step(
                "B-04",
                "音频：合成音频",
                "generating",
                88,
                "正在合并分段 mp3。",
            );
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
        callback.update(92, "正在读取音频时长。");
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
        "音频生成完成。",
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
            "Operation failed: {error}"
        ))
    })?;
    let entries = if let Some(trace_id) = trace_id {
        query_operation_logs_by_trace(&connection, limit, trace_id)?
    } else {
        query_operation_logs_since(&connection, limit, &data.app_started_at)?
    };
    Ok(GetOperationLogsResult { entries })
}

#[tauri::command]
pub fn get_material_task_steps(
    data: State<'_, AppData>,
    request: GetMaterialTaskStepsRequest,
) -> Result<GetMaterialTaskStepsResult, CommandError> {
    let connection = Connection::open(&data.db_path).map_err(|error| {
        command_error(format!("Open operation database failed: {error}"))
    })?;
    ensure_material_task_steps_table(&connection)?;
    let trace_id = request.trace_id.unwrap_or_default();
    let path = request.path.unwrap_or_default();
    let steps = if !trace_id.trim().is_empty() {
        query_material_task_steps_by_trace(&connection, trace_id.trim())?
    } else if !path.trim().is_empty() {
        query_material_task_steps_by_path(&connection, path.trim())?
    } else {
        Vec::new()
    };
    Ok(GetMaterialTaskStepsResult { steps })
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
                "Operation failed: {error}"
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
    if settings.active_ai_provider.trim() == "gemini" {
        return call_gemini_ai(settings, messages).await;
    }
    call_openai_compatible_ai(settings, messages).await
}

async fn call_openai_compatible_ai(
    settings: &AppSettings,
    messages: Vec<ChatMessage>,
) -> Result<String, CommandError> {
    let profile = &settings.ai_profile;
    if profile.api_key.trim().is_empty() {
        return Err(command_error("请先填写 AI API Key。"));
    }
    if profile.base_url.trim().is_empty() {
        return Err(command_error("请先填写 AI Base URL。"));
    }
    if profile.model.trim().is_empty() {
        return Err(command_error("请先填写 AI 模型名称。"));
    }

    let base = profile.base_url.trim().trim_end_matches('/');
    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    };

    let mut client_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(AI_REQUEST_TIMEOUT_SECONDS))
        .connect_timeout(Duration::from_secs(30));
    if profile.proxy_enabled && !profile.proxy_url.trim().is_empty() {
        let proxy = reqwest::Proxy::all(profile.proxy_url.trim())
            .map_err(|error| command_error(format!("AI 代理配置无效：{error}")))?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder
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
                        format!("AI 请求失败，HTTP {status}")
                    } else {
                        format!(
                            "AI 请求失败，HTTP {status}：{}",
                            text_preview(&detail, 500)
                        )
                    };
                    if !should_retry_ai_status(status) || attempt == AI_REQUEST_MAX_ATTEMPTS {
                        return Err(command_error(message));
                    }
                    last_error = Some(message);
                }
                Err(error) => {
                    let message = format!("AI 请求发送失败：{error}");

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
        .map_err(|error| command_error(format!("Read AI response body failed: {error}")))?;
    let content = if content_type.contains("text/event-stream") || body.contains("data:") {
        parse_streaming_chat_content(&body)?
    } else {
        parse_blocking_chat_content(&body)?
    };

    if content.trim().is_empty() {
        Err(command_error("AI returned empty content."))
    } else {
        Ok(content)
    }
}

async fn call_gemini_ai(
    settings: &AppSettings,
    messages: Vec<ChatMessage>,
) -> Result<String, CommandError> {
    let profile = &settings.gemini_profile;
    if profile.api_key.trim().is_empty() {
        return Err(command_error("请先填写 Gemini API Key。"));
    }
    if profile.base_url.trim().is_empty() {
        return Err(command_error("请先填写 Gemini Base URL。"));
    }
    if profile.model.trim().is_empty() {
        return Err(command_error("请先填写 Gemini 模型名称。"));
    }

    let base = profile.base_url.trim().trim_end_matches('/');
    let url = if base.ends_with(":generateContent") {
        base.to_string()
    } else {
        format!("{base}/models/{}:generateContent", profile.model.trim())
    };

    let mut client_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(AI_REQUEST_TIMEOUT_SECONDS))
        .connect_timeout(Duration::from_secs(30));
    let proxy_url = profile.proxy_url.trim();
    if profile.proxy_enabled && !proxy_url.is_empty() {
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|error| command_error(format!("Gemini 代理配置无效：{error}")))?;
        client_builder = client_builder.proxy(proxy);
    }
    let client = client_builder
        .build()
        .map_err(|error| command_error(format!("Build Gemini HTTP client failed: {error}")))?;

    let prompt = messages_to_gemini_prompt(messages);
    let request_body = GeminiGenerateRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: prompt }],
        }],
    };
    let mut last_error = None;
    let response = 'attempts: loop {
        for attempt in 1..=AI_REQUEST_MAX_ATTEMPTS {
            let result = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .header(
                    reqwest::header::USER_AGENT,
                    "A-Book-in-30-Minutes/0.1 Gemini-Client",
                )
                .header("X-goog-api-key", profile.api_key.trim())
                .json(&request_body)
                .send()
                .await;

            match result {
                Ok(response) if response.status().is_success() => break 'attempts response,
                Ok(response) => {
                    let status = response.status();
                    let detail = response.text().await.unwrap_or_default();
                    let message = if detail.trim().is_empty() {
                        format!("Gemini 请求失败，HTTP {status}")
                    } else {
                        format!(
                            "Gemini 请求失败，HTTP {status}：{}",
                            text_preview(&detail, 500)
                        )
                    };
                    if !should_retry_ai_status(status) || attempt == AI_REQUEST_MAX_ATTEMPTS {
                        return Err(command_error(message));
                    }
                    last_error = Some(message);
                }
                Err(error) => {
                    let message = format!("Gemini 请求发送失败：{error}");
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
            last_error.unwrap_or_else(|| "Gemini request failed.".to_string()),
        ));
    };

    let body = response
        .text()
        .await
        .map_err(|error| command_error(format!("Read Gemini response body failed: {error}")))?;
    parse_gemini_content(&body)
}

fn messages_to_gemini_prompt(messages: Vec<ChatMessage>) -> String {
    messages
        .into_iter()
        .map(|message| {
            let role = match message.role.as_str() {
                "system" => "System",
                "assistant" => "Assistant",
                _ => "User",
            };
            format!("{role}: {}", message.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn parse_gemini_content(body: &str) -> Result<String, CommandError> {
    let response = serde_json::from_str::<GeminiGenerateResponse>(body)
        .map_err(|error| command_error(format!("Gemini 响应解析失败：{error}")))?;
    let content = response
        .candidates
        .unwrap_or_default()
        .into_iter()
        .filter_map(|candidate| candidate.content)
        .flat_map(|content| content.parts)
        .map(|part| part.text)
        .collect::<Vec<_>>()
        .join("");
    if content.trim().is_empty() {
        Err(command_error("Gemini 返回内容为空。"))
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
    choices: Option<Vec<StreamingChatChoice>>,
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
    let mut saw_data = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("data:") {
            continue;
        }
        let data = trimmed.trim_start_matches("data:").trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        saw_data = true;
        let value = serde_json::from_str::<serde_json::Value>(data).map_err(|error| {
            command_error(format!(
                "Parse AI streaming response failed: {error}; chunk={}",
                text_preview(data, 500)
            ))
        })?;
        if let Some(error) = value.get("error") {
            return Err(command_error(format!(
                "AI streaming response returned error: {}",
                text_preview(&error.to_string(), 500)
            )));
        }
        let chunk = serde_json::from_value::<StreamingChatChunk>(value).map_err(|error| {
            command_error(format!(
                "Parse AI streaming response failed: {error}; chunk={}",
                text_preview(data, 500)
            ))
        })?;
        let Some(choices) = chunk.choices else {
            continue;
        };
        for choice in choices {
            if let Some(delta) = choice.delta {
                if let Some(part) = delta.content {
                    content.push_str(&part);
                }
            }
        }
    }
    if !saw_data && content.trim().is_empty() {
        return Err(command_error("AI streaming response did not contain data chunks."));
    }
    Ok(content)
}

fn parse_blocking_chat_content(body: &str) -> Result<String, CommandError> {
    let response = serde_json::from_str::<ChatCompletionResponse>(body)
        .map_err(|error| command_error(format!("AI operation failed: {error}")))?;
    response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| command_error("AI operation failed."))
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
    validate_ssml(ssml)?;
    let region = profile.region.trim();
    let url = format!("https://{region}.tts.speech.microsoft.com/cognitiveservices/v1");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(SPEECH_REQUEST_TIMEOUT_SECONDS))
        .connect_timeout(Duration::from_secs(20));
    let client = if profile.proxy_enabled && !profile.proxy_url.trim().is_empty() {
        let proxy = reqwest::Proxy::all(profile.proxy_url.trim())
            .map_err(|error| command_error(format!("Speech proxy configuration is invalid: {error}")))?;
        client.proxy(proxy).build()
    } else {
        client.build()
    }
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
        "<speak version=\"1.0\" xml:lang=\"{}\" xmlns=\"http://www.w3.org/2001/10/synthesis\"><voice name=\"{}\">{}{}</prosody></voice></speak>",
        escape_xml_attr(locale),
        escape_xml_attr(profile.voice_name.trim()),
        prosody_open,
        escape_xml_text(text)
    )
}

fn validate_ssml(ssml: &str) -> Result<(), CommandError> {
    let trimmed = ssml.trim_start();
    if !trimmed.starts_with("<speak") {
        return Err(command_error(format!(
            "SSML 格式错误：请求体必须以 <speak> 开始。预览：{}",
            text_preview(trimmed, 180)
        )));
    }
    if ssml.contains("Operation completed") {
        return Err(command_error(format!(
            "SSML 格式错误：请求体包含占位文案 Operation completed。预览：{}",
            text_preview(trimmed, 180)
        )));
    }
    Ok(())
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
                '。' | '？' | '?' | '！' | '!' | '；' | ';' | '，' | ',' | '.'
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
            "Manifest operation failed: {error}"
        ))
    })?;
    fs::write(path, json).map_err(|error| {
        command_error(format!(
            "Manifest operation failed: {error}"
        ))
    })?;
    Ok(())
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut current = String::new();
    for char in text.chars() {
        current.push(char);
        if matches!(char, '。' | '？' | '?' | '！' | '!' | '.' | '\n') {
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
            "ffmpeg operation failed.",
        ));
    }
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err(command_error(
            "ffmpeg operation failed.",
        ));
    }
    let output = Command::new(&path_buf)
        .arg("-version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| command_error(format!("ffmpeg operation failed: {error}")))?;
    if !output.status.success() {
        return Err(command_error(format!(
            "ffmpeg operation failed: code={:?} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .next()
        .unwrap_or("ffmpeg operation failed.")
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
            "ffmpeg operation failed: {error}"
        ))
    })?;
    logger.trace_info(
        "audio",
        "ffmpeg.concat",
        "ffmpeg operation failed.",
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
        .map_err(|error| command_error(format!("ffmpeg operation failed: {error}")))?;
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
        "ffmpeg operation completed.",
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
            "ffmpeg operation failed.",
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
                "ffmpeg operation failed: {error}"
            ))
        })?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let duration = parse_ffmpeg_duration_ms(&stderr).ok_or_else(|| {
        command_error("ffmpeg operation failed.")
    })?;
    logger.trace_info(
        "audio",
        "duration.probe",
        "Operation completed.",
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
        let parent = data
            .db_path
            .parent()
            .ok_or_else(|| command_error("\u{65e0}\u{6cd5}\u{786e}\u{5b9a}\u{5e94}\u{7528}\u{6570}\u{636e}\u{76ee}\u{5f55}\u{3002}"))?;
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
            if let Err(error) = ensure_settings_table(&connection) {
                logger.error(
                    "settings",
                    "settings.init",
                    "Initialize settings table failed",
                    error.message,
                );
            }
            if let Err(error) = ensure_material_tasks_table(&connection) {
                logger.error(
                    "materials",
                    "tasks.init",
                    "Initialize material tasks table failed",
                    error.message,
                );
            }
            if let Err(error) = ensure_material_task_steps_table(&connection) {
                logger.error(
                    "materials",
                    "task_steps.init",
                    "Initialize material task steps table failed",
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

fn ensure_settings_table(connection: &Connection) -> Result<(), CommandError> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS app_settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            "#,
        )
        .map_err(|error| command_error(format!("\u{521d}\u{59cb}\u{5316}\u{914d}\u{7f6e}\u{8868}\u{5931}\u{8d25}\u{ff1a}{error}")))?;
    Ok(())
}

fn load_settings_from_database_or_migrate_legacy_json(
    db_path: &Path,
    settings_path: &Path,
) -> Result<AppSettings, String> {
    if let Ok(connection) = Connection::open(db_path) {
        if ensure_settings_table(&connection).is_ok() {
            if let Ok(content) = connection.query_row(
                "SELECT value FROM app_settings WHERE key = 'settings'",
                [],
                |row| row.get::<_, String>(0),
            ) {
                return serde_json::from_str::<AppSettings>(content.trim_start_matches('\u{feff}'))
                    .map_err(|error| format!("\u{89e3}\u{6790}\u{6570}\u{636e}\u{5e93}\u{914d}\u{7f6e}\u{5931}\u{8d25}\u{ff1a}{error}"));
            }
        }
    }

    let content = fs::read_to_string(settings_path)
        .map_err(|error| format!("\u{8bfb}\u{53d6}\u{65e7}\u{914d}\u{7f6e}\u{6587}\u{4ef6}\u{5931}\u{8d25}\u{ff1a}{error}"))?;
    let settings = serde_json::from_str::<AppSettings>(content.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("\u{89e3}\u{6790}\u{65e7}\u{914d}\u{7f6e}\u{6587}\u{4ef6}\u{5931}\u{8d25}\u{ff1a}{error}"))?;
    if save_settings_to_database(db_path, &settings).is_ok() {
        let _ = fs::remove_file(settings_path);
    }
    Ok(settings)
}

fn save_settings_to_database(db_path: &Path, settings: &AppSettings) -> Result<(), CommandError> {
    let connection = Connection::open(db_path)
        .map_err(|error| command_error(format!("\u{6253}\u{5f00}\u{914d}\u{7f6e}\u{6570}\u{636e}\u{5e93}\u{5931}\u{8d25}\u{ff1a}{error}")))?;
    ensure_settings_table(&connection)?;
    let content = serde_json::to_string_pretty(settings)
        .map_err(|error| command_error(format!("\u{5e8f}\u{5217}\u{5316}\u{914d}\u{7f6e}\u{5931}\u{8d25}\u{ff1a}{error}")))?;
    let now = chrono::Local::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('settings', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![content, now],
        )
        .map_err(|error| command_error(format!("\u{4fdd}\u{5b58}\u{914d}\u{7f6e}\u{5230}\u{6570}\u{636e}\u{5e93}\u{5931}\u{8d25}\u{ff1a}{error}")))?;
    Ok(())
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
              category TEXT NOT NULL DEFAULT '半小时听完一本书',
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
              image_status TEXT NOT NULL DEFAULT 'pending',
              image_progress INTEGER NOT NULL DEFAULT 0,
              image_output_dir TEXT,
              image_message TEXT NOT NULL DEFAULT '',
              subtitle_status TEXT NOT NULL DEFAULT 'pending',
              subtitle_progress INTEGER NOT NULL DEFAULT 0,
              subtitle_file TEXT,
              subtitle_message TEXT NOT NULL DEFAULT '',
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
        "ALTER TABLE material_tasks ADD COLUMN category TEXT NOT NULL DEFAULT '半小时听完一本书'",
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
        "ALTER TABLE material_tasks ADD COLUMN image_status TEXT NOT NULL DEFAULT 'pending'",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN image_progress INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = connection.execute("ALTER TABLE material_tasks ADD COLUMN image_output_dir TEXT", []);
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN image_message TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN subtitle_status TEXT NOT NULL DEFAULT 'pending'",
        [],
    );
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN subtitle_progress INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = connection.execute("ALTER TABLE material_tasks ADD COLUMN subtitle_file TEXT", []);
    let _ = connection.execute(
        "ALTER TABLE material_tasks ADD COLUMN subtitle_message TEXT NOT NULL DEFAULT ''",
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

fn query_material_task_steps_by_trace(
    connection: &Connection,
    trace_id: &str,
) -> Result<Vec<MaterialTaskStep>, CommandError> {
    let mut statement = connection
        .prepare(
            "SELECT trace_id, path, step_code, step_name, status, progress, detail, started_at, finished_at, elapsed_ms, updated_at
             FROM material_task_steps
             WHERE trace_id=?1
             ORDER BY step_code ASC",
        )
        .map_err(|error| command_error(format!("Prepare task steps query failed: {error}")))?;
    let rows = statement
        .query_map(params![trace_id], material_task_step_from_row)
        .map_err(|error| command_error(format!("Query task steps failed: {error}")))?;
    collect_material_task_steps(rows)
}

fn query_material_task_steps_by_path(
    connection: &Connection,
    path: &str,
) -> Result<Vec<MaterialTaskStep>, CommandError> {
    let latest_trace_id: Option<String> = connection
        .query_row(
            "SELECT trace_id FROM material_task_steps WHERE path = ?1 ORDER BY updated_at DESC LIMIT 1",
            params![path],
            |row| row.get(0),
        )
        .ok();
    match latest_trace_id {
        Some(trace_id) => query_material_task_steps_by_trace(connection, &trace_id),
        None => Ok(Vec::new()),
    }
}

fn collect_material_task_steps<F>(
    rows: rusqlite::MappedRows<'_, F>,
) -> Result<Vec<MaterialTaskStep>, CommandError>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<MaterialTaskStep>,
{
    let mut steps = Vec::new();
    for row in rows {
        steps.push(row.map_err(|error| command_error(format!("Read task step row failed: {error}")))?);
    }
    Ok(steps)
}

fn material_task_step_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MaterialTaskStep> {
    Ok(MaterialTaskStep {
        trace_id: row.get(0)?,
        path: row.get(1)?,
        step_code: row.get(2)?,
        step_name: row.get(3)?,
        status: row.get(4)?,
        progress: row.get(5)?,
        detail: row.get(6)?,
        started_at: row.get(7)?,
        finished_at: row.get(8)?,
        elapsed_ms: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn ensure_material_task_steps_table(connection: &Connection) -> Result<(), CommandError> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS material_task_steps (
              trace_id TEXT NOT NULL,
              path TEXT NOT NULL,
              step_code TEXT NOT NULL,
              step_name TEXT NOT NULL,
              status TEXT NOT NULL DEFAULT 'pending',
              progress INTEGER NOT NULL DEFAULT 0,
              detail TEXT NOT NULL DEFAULT '',
              started_at TEXT,
              finished_at TEXT,
              elapsed_ms INTEGER,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              PRIMARY KEY (trace_id, step_code)
            );
            CREATE INDEX IF NOT EXISTS idx_material_task_steps_path ON material_task_steps(path);
            CREATE INDEX IF NOT EXISTS idx_material_task_steps_updated_at ON material_task_steps(updated_at);
            "#,
        )
        .map_err(|error| command_error(format!("Ensure material task steps table failed: {error}")))?;
    Ok(())
}

fn upsert_material_task(connection: &Connection, file: &MaterialFile) -> Result<(), CommandError> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    connection
        .execute(
            r#"
            INSERT INTO material_tasks
              (path, name, extension, size, category, status, progress, narration_chars, material_output_dir, message, audio_status, audio_progress, audio_output_dir, audio_file, audio_duration_ms, audio_chunks, audio_message, image_status, image_progress, image_output_dir, image_message, subtitle_status, subtitle_progress, subtitle_file, subtitle_message, video_status, video_progress, video_file, video_duration_ms, video_file_size, video_message, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?32)
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
                normalize_task_status(&file.image_status),
                clamp_task_progress(file.image_progress),
                file.image_output_dir,
                file.image_message,
                normalize_task_status(&file.subtitle_status),
                clamp_task_progress(file.subtitle_progress),
                file.subtitle_file,
                file.subtitle_message,
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
        image_status: "pending".to_string(),
        image_progress: 0,
        image_output_dir: None,
        image_message: String::new(),
        subtitle_status: "pending".to_string(),
        subtitle_progress: 0,
        subtitle_file: None,
        subtitle_message: String::new(),
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
        &format!("SELECT {MATERIAL_TASK_SELECT_COLUMNS} FROM material_tasks WHERE path = ?1"),
        params![path],
        material_task_from_row,
    );
    match result {
        Ok(file) => Ok(Some(file)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(command_error(format!(
            "Operation failed: {error}"
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
        .map_err(|error| command_error(format!("Operation failed: {error}")))
}

fn material_task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MaterialFile> {
    let size: i64 = row.get(3)?;
    let mut file = MaterialFile {
        path: row.get(0)?,
        name: row.get(1)?,
        extension: row.get(2)?,
        size: size.max(0) as u64,
        category: row.get(4)?,
        status: normalize_task_status(&row.get::<_, String>(5)?),
        progress: clamp_task_progress(row.get(6)?),
        narration_chars: row.get(7)?,
        material_output_dir: row.get(8)?,
        message: normalize_task_message(row.get(9)?, "material"),
        audio_status: normalize_task_status(&row.get::<_, String>(10)?),
        audio_progress: clamp_task_progress(row.get(11)?),
        audio_output_dir: row.get(12)?,
        audio_file: row.get(13)?,
        audio_duration_ms: row.get(14)?,
        audio_chunks: row.get(15)?,
        audio_message: normalize_task_message(row.get(16)?, "audio"),
        image_status: normalize_task_status(&row.get::<_, String>(17)?),
        image_progress: clamp_task_progress(row.get(18)?),
        image_output_dir: row.get(19)?,
        image_message: normalize_task_message(row.get(20)?, "image"),
        subtitle_status: normalize_task_status(&row.get::<_, String>(21)?),
        subtitle_progress: clamp_task_progress(row.get(22)?),
        subtitle_file: row.get(23)?,
        subtitle_message: normalize_task_message(row.get(24)?, "subtitle"),
        video_status: normalize_task_status(&row.get::<_, String>(25)?),
        video_progress: clamp_task_progress(row.get(26)?),
        video_file: row.get(27)?,
        video_duration_ms: row.get(28)?,
        video_file_size: row.get(29)?,
        video_message: normalize_task_message(row.get(30)?, "video"),
    };
    normalize_material_task_outputs(&mut file);
    Ok(file)
}

fn normalize_task_message(message: String, stage: &str) -> String {
    if !looks_like_garbled_text(&message) {
        return message;
    }
    match stage {
        "audio" => "\u{97f3}\u{9891}\u{4efb}\u{52a1}\u{72b6}\u{6001}\u{5f02}\u{5e38}\u{ff0c}\u{8bf7}\u{6309}\u{9700}\u{91cd}\u{65b0}\u{751f}\u{6210}\u{97f3}\u{9891}\u{3002}".to_string(),
        "image" => "\u{56fe}\u{7247}\u{4efb}\u{52a1}\u{72b6}\u{6001}\u{5f02}\u{5e38}\u{ff0c}\u{8bf7}\u{6309}\u{9700}\u{91cd}\u{65b0}\u{751f}\u{6210}\u{56fe}\u{7247}\u{3002}".to_string(),
        "subtitle" => "\u{5b57}\u{5e55}\u{4efb}\u{52a1}\u{72b6}\u{6001}\u{5f02}\u{5e38}\u{ff0c}\u{8bf7}\u{6309}\u{9700}\u{91cd}\u{65b0}\u{751f}\u{6210}\u{5b57}\u{5e55}\u{3002}".to_string(),
        "video" => "\u{89c6}\u{9891}\u{4efb}\u{52a1}\u{72b6}\u{6001}\u{5f02}\u{5e38}\u{ff0c}\u{8bf7}\u{6309}\u{9700}\u{91cd}\u{65b0}\u{751f}\u{6210}\u{89c6}\u{9891}\u{3002}".to_string(),
        _ => "\u{4efb}\u{52a1}\u{72b6}\u{6001}\u{5f02}\u{5e38}\u{ff0c}\u{8bf7}\u{6309}\u{9700}\u{91cd}\u{65b0}\u{751f}\u{6210}\u{5f53}\u{524d}\u{9636}\u{6bb5}\u{3002}".to_string(),
    }
}

fn normalize_material_task_outputs(file: &mut MaterialFile) {
    normalize_material_task_outputs_from_disk(file);
}

fn material_dir_has_text_assets(path: &Path) -> bool {
    path.is_dir()
        && staged_material_path(path, "narration.txt").is_file()
        && staged_material_path(path, "subtitles.txt").is_file()
        && staged_material_path(path, "materials.json").is_file()
}

fn normalize_material_task_outputs_from_disk(file: &mut MaterialFile) {
    let has_text_assets = file
        .material_output_dir
        .as_deref()
        .map(Path::new)
        .map(material_dir_has_text_assets)
        .unwrap_or(false);
    if has_text_assets {
        file.status = "success".to_string();
        file.progress = 100;
        if file.narration_chars.unwrap_or_default() <= 0 {
            file.narration_chars = file
                .material_output_dir
                .as_deref()
                .and_then(|dir| count_narration_chars_in_dir(Path::new(dir)));
        }
        if file.message.trim().is_empty() || looks_like_garbled_text(&file.message) {
            file.message = "文本素材已存在，已跳过文本生成。".to_string();
        }
    } else {
        file.status = "pending".to_string();
        file.progress = 0;
        file.narration_chars = None;
        file.material_output_dir = None;
        file.message = "文本素材缺失，请按需重新生成文本。".to_string();
    }

    if file.audio_status != "generating" && !path_option_exists(&file.audio_file) {
        file.audio_status = "pending".to_string();
        file.audio_progress = 0;
        file.audio_file = None;
        file.audio_duration_ms = None;
        file.audio_chunks = None;
        file.audio_message = "音频文件缺失，请按需重新生成音频。".to_string();
    }
    if file.image_status != "generating" && !path_option_exists(&file.image_output_dir) {
        file.image_status = "pending".to_string();
        file.image_progress = 0;
        file.image_output_dir = None;
        file.image_message = "图片素材缺失，请按需重新生成图片。".to_string();
    }
    if file.subtitle_status != "generating" && !path_option_exists(&file.subtitle_file) {
        file.subtitle_status = "pending".to_string();
        file.subtitle_progress = 0;
        file.subtitle_file = None;
        file.subtitle_message = "字幕文件缺失，请按需重新生成字幕。".to_string();
    }
    if file.video_status != "generating" && !path_option_exists(&file.video_file) {
        file.video_status = "pending".to_string();
        file.video_progress = 0;
        file.video_file = None;
        file.video_duration_ms = None;
        file.video_file_size = None;
        file.video_message = "视频文件缺失，请按需重新生成视频。".to_string();
    }
}

fn persist_reconciled_task_status(
    connection: &Connection,
    before: &MaterialFile,
    after: &MaterialFile,
) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    // 文本阶段
    if before.status != after.status || before.progress != after.progress || before.material_output_dir != after.material_output_dir {
        let _ = connection.execute(
            "UPDATE material_tasks
             SET status = ?2, progress = ?3, material_output_dir = ?4, narration_chars = ?5, message = ?6, updated_at = ?7
             WHERE path = ?1",
            params![after.path, after.status, after.progress, after.material_output_dir, after.narration_chars, after.message, now],
        );
    }
    // 音频阶段
    if before.audio_status != after.audio_status || before.audio_progress != after.audio_progress || before.audio_file != after.audio_file {
        let _ = connection.execute(
            "UPDATE material_tasks
             SET audio_status = ?2, audio_progress = ?3, audio_file = ?4, audio_message = ?5, updated_at = ?6
             WHERE path = ?1",
            params![after.path, after.audio_status, after.audio_progress, after.audio_file, after.audio_message, now],
        );
    }
    // 图片阶段
    if before.image_status != after.image_status || before.image_progress != after.image_progress || before.image_output_dir != after.image_output_dir {
        let _ = connection.execute(
            "UPDATE material_tasks
             SET image_status = ?2, image_progress = ?3, image_output_dir = ?4, image_message = ?5, updated_at = ?6
             WHERE path = ?1",
            params![after.path, after.image_status, after.image_progress, after.image_output_dir, after.image_message, now],
        );
    }
    // 字幕阶段
    if before.subtitle_status != after.subtitle_status || before.subtitle_progress != after.subtitle_progress || before.subtitle_file != after.subtitle_file {
        let _ = connection.execute(
            "UPDATE material_tasks
             SET subtitle_status = ?2, subtitle_progress = ?3, subtitle_file = ?4, subtitle_message = ?5, updated_at = ?6
             WHERE path = ?1",
            params![after.path, after.subtitle_status, after.subtitle_progress, after.subtitle_file, after.subtitle_message, now],
        );
    }
    // 视频阶段
    if before.video_status != after.video_status || before.video_progress != after.video_progress || before.video_file != after.video_file {
        let _ = connection.execute(
            "UPDATE material_tasks
             SET video_status = ?2, video_progress = ?3, video_file = ?4, video_message = ?5, updated_at = ?6
             WHERE path = ?1",
            params![after.path, after.video_status, after.video_progress, after.video_file, after.video_message, now],
        );
    }
}

fn count_narration_chars_in_dir(path: &Path) -> Option<i64> {
    fs::read_to_string(staged_material_path(path, "narration.txt"))
        .ok()
        .map(|content| count_han_chars(&content) as i64)
        .filter(|count| *count > 0)
}

fn path_option_exists(path: &Option<String>) -> bool {
    path.as_deref()
        .map(Path::new)
        .map(Path::exists)
        .unwrap_or(false)
}


fn normalize_loaded_task_for_manual_resume(file: &mut MaterialFile) {
    if file.status == "generating" {
        file.status = "pending".to_string();
        file.progress = 0;
        file.message = "上次素材任务未完成，请手动继续。".to_string();
    }
    if file.audio_status == "generating" {
        file.audio_status = "pending".to_string();
        file.audio_progress = 0;
        file.audio_message = "上次音频任务未完成，请手动继续。".to_string();
    }
    if file.video_status == "generating" {
        file.video_status = "pending".to_string();
        file.video_progress = 0;
        file.video_message = "上次视频任务未完成，请手动继续。".to_string();
    }
}

fn update_material_task_output_dir(
    connection: &Connection,
    path: &str,
    output_dir: &str,
) -> Result<(), CommandError> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    connection
        .execute(
            "UPDATE material_tasks
             SET material_output_dir = ?2, status = 'success', progress = 100, message = '素材包已保存', updated_at = ?3
             WHERE path = ?1",
            params![path, output_dir, now],
        )
        .map_err(|error| {
            command_error(format!(
                "Operation failed: {error}"
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
            "Output directory operation failed.",
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
        .map_err(|error| command_error(format!("Output directory operation failed: {error}")))?;

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
                    "Output directory operation failed: {error}"
                ))
            })?;
            let target = unique_child_path(
                &output_dir,
                old.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("audio.mp3"),
            );
            fs::copy(&old, &target).map_err(|error| {
                command_error(format!("Operation failed: {error}"))
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
                    "Output directory operation failed: {error}"
                ))
            })?;
            let target = unique_child_path(
                &output_dir,
                old.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("video.mp4"),
            );
            fs::copy(&old, &target).map_err(|error| {
                command_error(format!("Operation failed: {error}"))
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
            "UPDATE material_tasks
             SET material_output_dir = ?2, audio_output_dir = ?3, audio_file = ?4, video_file = ?5, video_file_size = ?6, updated_at = ?7
             WHERE path = ?1",
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
        .map_err(|error| command_error(format!("Output directory operation failed: {error}")))?;
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
                command_error(format!("Operation failed: {error}"))
            })?;
        }
        fs::copy(source, target)
            .map_err(|error| command_error(format!("Operation failed: {error}")))?;
        return Ok(());
    }
    fs::create_dir_all(target).map_err(|error| {
        command_error(format!("Operation failed: {error}"))
    })?;
    for entry in fs::read_dir(source).map_err(|error| {
        command_error(format!(
            "Operation failed: {error}"
        ))
    })? {
        let entry = entry.map_err(|error| {
            command_error(format!(
                "Operation failed: {error}"
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
        if has_material_package_files(&base_dir) {
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
            if has_material_package_files(&path) {
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
    if has_material_package_files(path) {
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
        if ensure_material_tasks_table(&connection).is_err() {
            return;
        }
        if let Ok(file) = material_file_from_path(path, DEFAULT_MATERIAL_CATEGORY) {
            let _ = upsert_material_task(&connection, &file);
        }
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let _ = connection.execute(
            "UPDATE material_tasks
             SET status = ?2, progress = ?3, message = ?4, updated_at = ?5
             WHERE path = ?1",
            params![
                path,
                normalize_task_status(status),
                clamp_task_progress(progress),
                message,
                now
            ],
        );
    }
}

fn upsert_material_task_step_db(
    db_path: &Path,
    trace_id: &str,
    path: &str,
    step_code: &str,
    step_name: &str,
    status: &str,
    progress: i64,
    detail: &str,
) {
    if let Ok(connection) = Connection::open(db_path) {
        if ensure_material_task_steps_table(&connection).is_err() {
            return;
        }
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let normalized_status = normalize_task_status(status);
        let normalized_progress = clamp_task_progress(progress);
        let existing_started_at = connection
            .query_row(
                "SELECT started_at FROM material_task_steps WHERE trace_id = ?1 AND step_code = ?2",
                params![trace_id, step_code],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        let started_at = existing_started_at
            .or_else(|| (normalized_status == "generating" || normalized_status == "success" || normalized_status == "failed").then(|| now.clone()));
        let existing_finished_at = connection
            .query_row(
                "SELECT finished_at FROM material_task_steps WHERE trace_id = ?1 AND step_code = ?2",
                params![trace_id, step_code],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        let finished_at = if normalized_status == "success" || normalized_status == "failed" {
            existing_finished_at.or_else(|| Some(now.clone()))
        } else {
            existing_finished_at
        };
        let elapsed_ms = match (started_at.as_deref(), finished_at.as_deref()) {
            (Some(start), Some(finish)) => elapsed_between_ms(start, finish),
            _ => None,
        };
        let _ = connection.execute(
            r#"
            INSERT INTO material_task_steps
              (trace_id, path, step_code, step_name, status, progress, detail, started_at, finished_at, elapsed_ms, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
            ON CONFLICT(trace_id, step_code) DO UPDATE SET
              path=excluded.path,
              step_name=excluded.step_name,
              status=excluded.status,
              progress=excluded.progress,
              detail=excluded.detail,
              started_at=COALESCE(material_task_steps.started_at, excluded.started_at),
              finished_at=COALESCE(material_task_steps.finished_at, excluded.finished_at),
              elapsed_ms=COALESCE(excluded.elapsed_ms, material_task_steps.elapsed_ms),
              updated_at=excluded.updated_at
            "#,
            params![
                trace_id,
                path,
                step_code,
                step_name,
                normalized_status,
                normalized_progress,
                detail,
                started_at,
                finished_at,
                elapsed_ms,
                now
            ],
        );
    }
}

fn elapsed_between_ms(start: &str, finish: &str) -> Option<i64> {
    let start_time = parse_step_datetime(start)?;
    let finish_time = parse_step_datetime(finish)?;
    Some((finish_time - start_time).num_milliseconds().max(1))
}

fn parse_step_datetime(value: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.3f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .ok()
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
                    "Operation failed: {error}"
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
        let seed_tmp = seed.join("tmp").join("book_video_pipeline.py");
        if seed_tmp.exists() {
            return Ok((seed.clone(), seed_tmp));
        }
        let seed_up_tmp = seed.join("_up_").join("tmp").join("book_video_pipeline.py");
        if seed_up_tmp.exists() {
            return Ok((seed.join("_up_"), seed_up_tmp));
        }
        let seed_resources_tmp = seed
            .join("resources")
            .join("tmp")
            .join("book_video_pipeline.py");
        if seed_resources_tmp.exists() {
            return Ok((seed.join("resources"), seed_resources_tmp));
        }
        for candidate in seed.ancestors() {
            let script = candidate.join("tmp").join("book_video_pipeline.py");
            if script.exists() {
                return Ok((candidate.to_path_buf(), script));
            }
            let up_script = candidate
                .join("_up_")
                .join("tmp")
                .join("book_video_pipeline.py");
            if up_script.exists() {
                return Ok((candidate.join("_up_"), up_script));
            }
            let resources_script = candidate
                .join("resources")
                .join("tmp")
                .join("book_video_pipeline.py");
            if resources_script.exists() {
                return Ok((candidate.join("resources"), resources_script));
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
        .ok_or_else(|| command_error("Operation completed."))?;
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

fn json_f64(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|item| item.as_f64())
}

fn json_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(|item| item.as_bool())
}

fn json_string_array(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|item| item.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::trim))
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_publish_output_dir(
    db_path: &Path,
    epub_path: &str,
    epub: &Path,
) -> Result<PathBuf, CommandError> {
    if let Some(dir) = resolve_task_material_dir_for_video(db_path, epub_path) {
        if dir.exists() {
            return Ok(dir);
        }
    }
    let output = epub.parent().unwrap_or_else(|| Path::new(".")).join("output");
    if output.exists() {
        return Ok(output);
    }
    Err(command_error(format!(
        "找不到 output 目录，请先生成素材：{}",
        output.to_string_lossy()
    )))
}

fn extract_publish_book_title(title: &str) -> String {
    let cleaned = title.trim();
    if let Some(start) = cleaned.find('《') {
        let body_start = start + '《'.len_utf8();
        if let Some(end) = cleaned[body_start..].find('》') {
            return cleaned[body_start..body_start + end].to_string();
        }
    }
    cleaned
        .split(['：', ':'])
        .next()
        .unwrap_or(cleaned)
        .trim_matches(['《', '》', ' '])
        .to_string()
}

fn build_publish_chapters(srt_path: &Path) -> Vec<(String, String)> {
    let content = fs::read_to_string(srt_path).unwrap_or_default();
    let cues = parse_srt_cues(&content);
    let chapter_rules = [
        ("开场：今晚不讲英雄，我们讲一个父亲", "我们讲一个父亲"),
        ("《亲爱的老爸》：海明威父子家书里的另一面", "亲爱的老爸"),
        ("战争、家庭与通信：远方父亲如何维持亲密", "战争"),
        ("【童年】父亲把远方折成一封信，寄给孩子", "第一站"),
        ("【寄宿学校】海、自由、学校和一个男孩的孤独", "寄宿学校"),
        ("【冲突】学校假期事件：爸爸站在我这边", "学校假期"),
        ("【语言】海明威连家书都写得像一场戏", "创作者"),
        ("【裂痕】离婚、继母、兄弟与复杂家庭现实", "现实的残酷"),
        ("【催信】十三天没有明信片，硬汉父亲半夜睡不着", "催孩子回信"),
        ("【成长】帕特里克怎样从父亲的光里走出自己", "青年时期"),
        ("【主题】亲情不会自动存在，它需要被写下、被回复", "真正讲的"),
        ("【收尾】夜深了，想象海明威坐在桌前写信", "夜深了"),
        ("晚安：再强大的人，也需要回信", "晚安"),
    ];
    chapter_rules
        .iter()
        .filter_map(|(label, needle)| {
            cues.iter()
                .find(|(_start, text)| text.contains(needle))
                .map(|(start, _text)| (format_youtube_chapter_time(start), (*label).to_string()))
        })
        .collect()
}

fn parse_srt_cues(content: &str) -> Vec<(String, String)> {
    let mut cues = Vec::new();
    for block in content.split("\n\n") {
        let mut lines = block.lines();
        let _index = lines.next();
        let Some(time_line) = lines.next() else {
            continue;
        };
        let Some(start) = time_line.split("-->").next().map(str::trim) else {
            continue;
        };
        let text = lines.next().unwrap_or_default().trim();
        if !start.is_empty() && !text.is_empty() {
            cues.push((start.to_string(), text.to_string()));
        }
    }
    cues
}

fn format_youtube_chapter_time(srt_time: &str) -> String {
    let time = srt_time.split(',').next().unwrap_or(srt_time).trim();
    let parts = time.split(':').collect::<Vec<_>>();
    if parts.len() == 3 && parts[0] == "00" {
        format!("{}:{}", parts[1], parts[2])
    } else {
        time.to_string()
    }
}

fn build_youtube_publish_markdown(
    title: &str,
    description: &str,
    tags: &[String],
    chapters: &[(String, String)],
    output_dir: &Path,
    video_path: &Path,
) -> String {
    let hashtags = build_hashtags(tags);
    let tag_line = tags.join(", ");
    let chapter_lines = chapters
        .iter()
        .map(|(time, label)| format!("{time} {label}"))
        .collect::<Vec<_>>()
        .join("\n");
    let hook = if description.trim().is_empty() {
        "我们熟悉的海明威，是硬汉，是战争、斗牛、猎枪和大海。但在《亲爱的老爸：海明威父子家书》里，他更多时候只是一个父亲。".to_string()
    } else {
        description.trim().to_string()
    };
    format!(
        "# YouTube 发布资料\n\n\
## 推荐标题\n{title}\n\n\
## 备选标题\n海明威不只是硬汉：他写给儿子的家书里，藏着一个笨拙又深情的父亲\n\n\
## 视频简介\n{hook}\n\n\
我们熟悉的海明威，是硬汉，是战争、斗牛、拳击、猎枪和大海，是写下《老人与海》的诺贝尔文学奖得主。\n\n\
但在《亲爱的老爸：海明威父子家书》里，他更多时候不是文学神话，而只是一个父亲。\n\n\
他写信给次子帕特里克。有时吹牛，有时开玩笑，有时暴躁，有时担心。他讲熊、狮子、野牛、猫、学校、成绩单、橄榄球，也讲战争、离别、家庭变故和无法说出口的想念。\n\n\
这本书最动人的地方，不是海明威多么伟大，而是他那么伟大，却仍然需要写信。需要解释，需要哄孩子，需要发脾气，需要说：请给我写信。\n\n\
这不是原书逐字朗读，而是基于书籍内容、章节线索与家书主题创作的中文转述、摘要与解读。愿这期节目，陪你在安静的夜里，听见亲情里那些笨拙、幽默、焦虑，又真诚的回声。\n\n\
## 关键时间线\n{chapter_lines}\n\n\
## 标签\n{tag_line}\n\n\
## Hashtags\n{hashtags}\n\n\
## 置顶评论\n你印象里的海明威，是“硬汉”更多，还是“父亲”更多？\n\n\
如果你也曾经等过一封信，或者等过一个人主动联系你，欢迎在评论区聊聊。\n\n\
## 发布文件\n- 视频：{video}\n- 输出目录：{output}\n",
        title = title.trim(),
        hook = hook,
        chapter_lines = chapter_lines,
        tag_line = tag_line,
        hashtags = hashtags,
        video = video_path.to_string_lossy(),
        output = output_dir.to_string_lossy()
    )
}

fn build_hashtags(tags: &[String]) -> String {
    let preferred = [
        "半小时听完一本书",
        "亲爱的老爸",
        "海明威",
        "父子家书",
        "文学解读",
        "睡前听书",
        "中英双语字幕",
    ];
    preferred
        .iter()
        .filter(|item| tags.iter().any(|tag| tag == **item) || **item == "中英双语字幕")
        .map(|item| format!("#{item}"))
        .collect::<Vec<_>>()
        .join(" ")
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
    let key_length = if settings.active_ai_provider.trim() == "gemini" {
        settings.gemini_profile.api_key.trim().chars().count()
    } else {
        settings.ai_profile.api_key.trim().chars().count()
    };
    format!(
        "active_provider={} model={} api_key_present={} api_key_length={} notifications_enabled={}",
        settings.active_ai_provider,
        current_ai_model(settings),
        key_length > 0,
        key_length,
        settings.notifications_enabled
    )
}

fn current_ai_model(settings: &AppSettings) -> String {
    if settings.active_ai_provider.trim() == "gemini" {
        settings.gemini_profile.model.clone()
    } else {
        settings.ai_profile.model.clone()
    }
}

fn current_ai_base_url(settings: &AppSettings) -> String {
    if settings.active_ai_provider.trim() == "gemini" {
        settings.gemini_profile.base_url.clone()
    } else {
        settings.ai_profile.base_url.clone()
    }
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
        "未命名书籍"
    } else {
        book.overview.title.trim()
    };
    let mut narration = format!(
        "今天我们一起读《{title}》。这本书的核心，不只是情节本身，更是它如何把人物、情感和时代处境慢慢推到我们面前。\n\n{source}"
    );
    narration = sanitize_generated_narration(&narration);
    let min_chars = request.target_min_chars.max(1000);
    while count_han_chars(&narration) < min_chars {
        narration.push_str(
            "\n\n把这些片段连起来看，我们会发现，作者真正关心的，是人在关系中的选择。也是那些迟疑、退让和改变。故事不急着给结论。它让我们在细节里靠近人物。也看见更长久的情绪回声。",
        );
        narration = sanitize_generated_narration(&narration);
    }
    AiBookMaterialsPayload {
        video_title: format!("《{title}》30 分钟听完一本书"),
        description: format!("用 30 分钟读完《{title}》，梳理故事脉络、人物关系和核心主题。"),
        tags: vec!["听书".to_string(), "读书".to_string(), "30分钟一本书".to_string()],
        narration,
        subtitles: Vec::new(),
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

fn trim_narration_to_total_chars(value: &str, max_chars: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max_chars {
        return value.trim().to_string();
    }
    let mut end = max_chars.min(chars.len());
    let search_start = end.saturating_sub(360);
    for index in (search_start..end).rev() {
        if matches!(chars[index], '。' | '？' | '?' | '！' | '!' | '；' | ';') {
            end = index + 1;
            break;
        }
    }
    chars[..end].iter().collect::<String>().trim().to_string()
}

fn sanitize_source_excerpt_text(value: &str) -> String {
    split_narration_units(value)
        .into_iter()
        .filter(|unit| !is_publisher_metadata_fragment(unit))
        .collect::<Vec<_>>()
        .join("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn usable_source_han_chars(book: &EpubBook) -> usize {
    book.chapters
        .iter()
        .map(|chapter| count_han_chars(&sanitize_source_excerpt_text(&chapter.text)))
        .sum()
}

fn trim_narration_to_han_chars(value: &str, max_han_chars: usize) -> String {
    let mut han_chars = 0usize;
    let mut end = 0usize;
    let mut last_sentence_end = None;
    for (index, ch) in value.char_indices() {
        if is_han_char(ch) {
            if han_chars >= max_han_chars {
                break;
            }
            han_chars += 1;
        }
        end = index + ch.len_utf8();
        if matches!(ch, '。' | '？' | '?' | '！' | '!' | '；' | ';') {
            last_sentence_end = Some(end);
        }
    }
    let end = last_sentence_end.unwrap_or(end).min(value.len());
    value[..end].trim().to_string()
}

fn sanitize_generated_narration(value: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    split_narration_units(value)
        .into_iter()
        .map(|unit| unit.trim().to_string())
        .filter(|unit| !unit.is_empty())
        .filter(|unit| !is_publisher_metadata_fragment(unit))
        .filter(|unit| {
            let key = normalize_narration_unit_key(unit);
            if key.is_empty() || count_han_chars(&key) < 8 {
                return true;
            }
            seen.insert(key)
        })
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string()
}

fn split_narration_units(value: &str) -> Vec<String> {
    let mut units = Vec::new();
    let mut current = String::new();
    for ch in value.chars() {
        if ch == '\r' {
            continue;
        }
        if ch == '\n' {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                units.push(trimmed.to_string());
            }
            current.clear();
            continue;
        }
        current.push(ch);
        if matches!(
            ch,
            '\u{3002}' | '\u{FF01}' | '\u{FF1F}' | '\u{FF1B}' | '!' | '?' | ';'
        ) {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                units.push(trimmed.to_string());
            }
            current.clear();
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        units.push(trimmed.to_string());
    }
    units
}

fn is_publisher_metadata_fragment(value: &str) -> bool {
    let compact = value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    if compact.is_empty() {
        return false;
    }
    let has_book_metadata_keyword = [
        "出版社",
        "出版时间",
        "出版日期",
        "出版发行",
        "出品方",
        "isbn",
        "书号",
        "版次",
        "印次",
        "开本",
        "定价",
        "版权所有",
        "copyright",
    ]
    .iter()
    .any(|keyword| compact.contains(keyword));
    if has_book_metadata_keyword {
        return true;
    }
    let digit_count = compact.chars().filter(|ch| ch.is_ascii_digit()).count();
    if digit_count >= 10 && compact.chars().count() <= 80 {
        return true;
    }
    let chinese_number_count = compact
        .chars()
        .filter(|ch| "零〇一二三四五六七八九十两".contains(*ch))
        .count();
    chinese_number_count >= 6
        && compact.chars().count() <= 60
        && (compact.contains('年') || compact.contains('月') || compact.contains("九七八"))
}

fn normalize_narration_unit_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    *ch,
                    '\u{3002}'
                        | '\u{FF0C}'
                        | '\u{3001}'
                        | '\u{FF01}'
                        | '\u{FF1F}'
                        | '\u{FF1B}'
                        | '\u{FF1A}'
                        | '\u{300A}'
                        | '\u{300B}'
                        | '"'
                        | '\''
                        | '“'
                        | '”'
                )
        })
        .collect()
}

fn build_book_materials_prompt(book: &EpubBook, request: &BookMaterialsRequest) -> String {
    let target_min = request.target_min_chars;
    let target_max = request.target_max_chars;
    let source_packet = build_source_packet(book);
    format!(
        "You are creating a Chinese audiobook video package. Return only valid JSON with keys: videoTitle, description, tags, narration, subtitles. Do not output markdown.\n\nRole:\n- You are a senior short-video subtitle editor and a healing late-night radio copywriter.\n- Your subtitle cuts should guide reading rhythm, emotional rise and fall, and a gentle sense of breathing.\n\nNarration task:\n- Write narration as a continuous Chinese script between {target_min} and {target_max} Chinese characters.\n- This target is chosen for a 30-35 minute Chinese listening experience. Stay inside the configured range.\n- Prefer concise, warm narration. Do not add repetitive reassurance or filler.\n- Control subtitle rhythm at the source: write narration with short sentences or short clauses separated by punctuation.\n- Each narration rhythm unit should preferably be {min_unit}-{max_unit} Chinese characters. A natural sentence may be slightly longer, but avoid many 2-4 character fragments.\n- If an idea is long, rewrite it into several natural 8-12 character sentences or clauses before it reaches subtitles.\n- Never use publisher metadata to fill length. Ignore publisher names, publication dates, ISBN, book numbers, edition/printing/price/copyright page facts, and production company names unless they are part of the book's argument.\n\nSubtitle task:\n- After writing the final narration, read and understand it as a human subtitle editor.\n- Create subtitles from the final narration, not from the source excerpts.\n- subtitles must be a JSON array of Chinese subtitle lines in exact reading order.\n- subtitles must cover the whole narration. Do not summarize, omit, add, rewrite, or reorder content.\n\nConstraint:\n1. Line length: each subtitle line should usually be 8-18 Chinese characters including punctuation. A meaningful line may be slightly longer only when splitting would damage a title, phrase, or natural rhythm.\n2. Punctuation: every subtitle line must end with punctuation. Preserve natural punctuation such as ，。？！；：、《》…… Do not strip punctuation.\n3. Format: subtitles is a JSON string array. No timestamps. One subtitle line per array item.\n4. Coverage: every sentence in narration must appear in subtitles in order.\n\nSegmentation logic:\n1. Semantic integrity: prefer line breaks at punctuation. If a sentence is too long, rewrite the narration into shorter semantic clauses. Never split a word or fixed phrase, for example “白血病”, “蒲公英”, “完美”, “希望”, “你有我呢”.\n2. Reading rhythm: use natural human breathing. Most rhythm units should feel like a complete spoken breath, not a two-character beat.\n3. Visual beauty: avoid 1-5 character orphan lines unless the short line is a rare intentional emotional beat.\n4. Phrase protection: keep book titles, names, idioms, number expressions, and short emotional phrases intact, such as 《天会亮的，你有我呢》 and “三十三个四季小故事”.\n5. Soft punctuation: commas may stay inside a line when they make the rhythm better; split after a comma only when both sides are meaningful.\n6. Tiny tails: if the final fragment is too short, merge it with the previous or next line.\n\nWhat to avoid:\n- Do not mechanically cut every 14, 16, 18, or 20 characters.\n- Do not remove punctuation to make lines shorter.\n- Do not split words such as “白血病”, names, or titles into isolated fragments.\n- Do not create orphan lines such as “的书”, “完”, “美”, “三十三”, “后来”, “于是”.\n- Do not join unrelated clauses into one long line just to avoid short lines.\n- Do not mention facts like “出版社是浙江文艺出版社，果麦文化。出版时间是二零二五年六月。ISBN是……”.\n\nBad examples:\n- “白血” / “病不幸夭折”\n- “完” / “美”\n- “的书” as a separate line\n- “天会亮的” / “你有我呢” when it is one title\n- “三十三” / “个四季小故事”\n- “出版社是……” as narration filler\n- One very long sentence with no punctuation and no breathing point\n\nOutput example:\n- “今晚要一起读的是，”\n- “一平著绘的《天会亮的，你有我呢》。”\n- “先把灯光调暗一点，”\n- “把白天没有说完的话，”\n- “轻轻放在枕边。”\n- “你不必马上变好，”\n- “也不必立刻回答生活的问题。”\n\nBook title: {title}\nAuthor: {author}\nLanguage: {language}\nExtra direction: {direction}\n\nSource excerpts:\n{source_packet}",
        title = book.overview.title,
        author = book.overview.creator,
        language = request.language,
        direction = request.extra_direction.clone(),
        min_unit = NARRATION_TARGET_MIN_UNIT_HAN,
        max_unit = NARRATION_TARGET_MAX_UNIT_HAN,
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
            let text = sanitize_source_excerpt_text(&chapter.text).replace('\n', " ");
            if text.trim().is_empty() {
                continue;
            }
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
                truncate_chars(&sanitize_source_excerpt_text(&chapter.text).replace('\n', " "), 520)
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
        "Rewrite the JSON so narration is between {min_chars} and {max_chars} Chinese characters for a 30-35 minute Chinese listening experience. Current narration Chinese chars: {current_chars}. Return only valid JSON with the same keys: videoTitle, description, tags, narration, subtitles.\n\nRole:\n- You are a senior short-video subtitle editor and a healing late-night radio copywriter.\n- Rebuild narration and subtitles with rhythm, breathing, and emotional pacing, not mechanical slicing.\n\nNarration rewrite task:\n- Rewrite narration itself into short sentences and short semantic clauses.\n- Most narration rhythm units should be 8-12 Chinese characters and end with punctuation.\n- Avoid many 2-4 character fragments. If a fragment is too short, merge it into a neighboring sentence.\n- If an idea is long, rewrite it into several natural 8-12 character sentences or clauses before creating subtitles.\n- Remove repeated opening/ending filler and exact repeated sentences.\n- Never use publisher metadata to fill length. Delete publisher names, publication dates, ISBN, book numbers, edition/printing/price/copyright page facts, and production company names.\n\nSubtitle rebuild task:\n- Rebuild subtitles only after reading and understanding the final rewritten narration.\n- subtitles must cover the whole final narration in exact reading order. Do not summarize, omit, add, rewrite, or reorder content.\n\nConstraint:\n1. Each subtitle line should usually be 8-18 Chinese characters including punctuation.\n2. Every subtitle line must end with punctuation. Preserve punctuation such as ，。？！；：、《》…… Do not strip punctuation.\n3. No timestamps. subtitles must be a JSON string array.\n\nSegmentation logic:\n1. Prefer breaks at punctuation. If a sentence is too long, rewrite the narration into shorter semantic clauses.\n2. Keep natural human breathing and a slow healing rhythm.\n3. Avoid 1-5 character orphan lines unless they intentionally emphasize emotion.\n4. Keep words, titles, names, idioms, number expressions, and short emotional phrases intact, such as “白血病”, “蒲公英”, “完美”, 《天会亮的，你有我呢》, “三十三个四季小故事”.\n5. If a fragment is too short, merge it with the previous or next line.\n\nDo not:\n- Do not mechanically cut every 14, 16, 18, or 20 characters.\n- Do not remove punctuation.\n- Do not split words such as “白血病”, names, or titles into fragments.\n- Do not create orphan lines such as “的书”, “完”, “美”, “三十三”, “后来”, “于是”.\n- Do not mention facts like “出版社是浙江文艺出版社，果麦文化。出版时间是二零二五年六月。ISBN是……”.\n- Do not join unrelated clauses into one long line just to avoid short lines.\n\nOutput example:\n- “今晚要一起读的是，”\n- “一平著绘的《天会亮的，你有我呢》。”\n- “先把灯光调暗一点，”\n- “把白天没有说完的话，”\n- “轻轻放在枕边。”\n\nExisting JSON:\n{}",
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
        "Continue and expand this Chinese narration from {current_chars} to {min_chars}-{max_chars} Chinese characters. Return only the additional narration text, no markdown.\n\nRules:\n- Continue the interpretation of the book's story, themes, characters, and emotional meaning.\n- Most rhythm units should be 8-12 Chinese characters and end with punctuation.\n- Avoid many 2-4 character fragments.\n- Do not repeat the opening, closing, or any existing sentence.\n- Never use publisher metadata to fill length. Do not mention publisher names, publication dates, ISBN, book numbers, edition/printing/price/copyright page facts, or production companies.\n\nTail:\n{tail}"
    )
}

fn clean_narration_extension(value: &str) -> String {
    let trimmed = value.trim();
    if let Ok(payload) = serde_json::from_str::<AiBookMaterialsPayload>(trimmed) {
        return sanitize_generated_narration(payload.narration.trim());
    }
    sanitize_generated_narration(trimmed.trim_matches('`').trim())
}

fn narration_extension_is_repetitive(base: &str, extension: &str) -> bool {
    let extension_lines = extension
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if extension_lines.is_empty() {
        return true;
    }
    let unique_count = extension_lines
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    if extension_lines.len() >= 4 && unique_count * 2 <= extension_lines.len() {
        return true;
    }
    let base_tail = tail_chars(base, 1400);
    extension_lines
        .iter()
        .filter(|line| count_han_chars(line) >= 20 && base_tail.contains(**line))
        .count()
        >= 2
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
        "在《{title}》的章节“{chapter_title}”里，第 {index} 个关键片段把故事继续向前推进。原文片段是：{cleaned}。这一处值得放进讲述里，因为它既提供了情节信息，也显露出人物的情绪和主题方向。",
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
        .ok_or_else(|| command_error("AI response did not contain valid JSON."))?;
    let payload = serde_json::from_str::<AiBookMaterialsPayload>(&json)
        .map_err(|error| command_error(format!("AI JSON 解析失败：{error}")))?;
    if payload.video_title.trim().is_empty() {
        return Err(command_error(
            "AI operation failed.",
        ));
    }
    if payload.narration.trim().is_empty() {
        return Err(command_error("AI operation failed."));
    }
    Ok(payload)
}

fn extract_json_object(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
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
        current.push(ch);
        let hard_break = is_hard_subtitle_break(ch);
        let soft_break = is_soft_subtitle_break(ch);
        let current_len = current.chars().count();
        if hard_break || (soft_break && current_len >= SUBTITLE_SOFT_CHARS) {
            push_clean_subtitle(&mut lines, &current, SUBTITLE_MAX_CHARS);
            current.clear();
        }
    }
    push_clean_subtitle(&mut lines, &current, SUBTITLE_MAX_CHARS);
    if lines.is_empty() {
        push_subtitle_chunks(&mut lines, narration, SUBTITLE_MAX_CHARS);
    }
    finalize_subtitle_lines(merge_short_subtitle_tails(
        lines,
        SUBTITLE_MIN_TAIL_CHARS,
        SUBTITLE_MAX_CHARS,
    ))
}

fn normalize_ai_subtitles(subtitles: &[String], narration: &str) -> Option<Vec<String>> {
    if subtitles.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    for subtitle in subtitles {
        let cleaned = clean_subtitle_line(subtitle);
        if cleaned.is_empty() {
            continue;
        }
        lines.push(cleaned);
    }
    let lines = finalize_subtitle_lines(merge_short_subtitle_tails(
        lines,
        SUBTITLE_MIN_TAIL_CHARS,
        SUBTITLE_MAX_CHARS,
    ));
    if lines.len() < 8 || !subtitle_coverage_ok(&lines, narration) {
        return None;
    }
    if lines.iter().any(|line| !subtitle_line_format_ok(line)) {
        return None;
    }
    let joined = lines.join("");
    if punctuation_count(&joined) < lines.len().saturating_div(4).max(8) {
        return None;
    }
    Some(lines)
}

fn push_clean_subtitle(lines: &mut Vec<String>, cleaned: &str, max_chars: usize) {
    let cleaned = clean_subtitle_line(cleaned);
    if cleaned.is_empty() {
        return;
    }
    push_subtitle_chunks(lines, &cleaned, max_chars);
}

fn clean_subtitle_line(value: &str) -> String {
    value.trim().to_string()
}

fn finalize_subtitle_lines(lines: Vec<String>) -> Vec<String> {
    let normalized = lines
        .into_iter()
        .map(|line| clean_subtitle_line(&line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let total = normalized.len();
    normalized
        .into_iter()
        .enumerate()
        .map(|(index, line)| ensure_subtitle_line_punctuation(&line, index + 1 == total))
        .collect()
}

fn ensure_subtitle_line_punctuation(line: &str, is_last: bool) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.chars().last().is_some_and(is_subtitle_terminal_punctuation) {
        return trimmed.to_string();
    }
    let mark = if is_last { '\u{3002}' } else { '\u{FF0C}' };
    format!("{trimmed}{mark}")
}

fn subtitles_need_ai_rewrite(lines: &[String], narration: &str) -> bool {
    if !subtitle_coverage_ok(lines, narration) {
        return true;
    }
    lines.iter().any(|line| !subtitle_line_format_ok(line))
}

fn ai_subtitle_rewrite_enabled() -> bool {
    std::env::var("ABOOK_ENABLE_AI_SUBTITLE_REWRITE")
        .map(|value| value.trim() == "1" || value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

async fn rewrite_subtitles_with_ai(
    settings: &AppSettings,
    narration: &str,
    current_lines: &[String],
) -> Option<Vec<String>> {
    let prompt = format!(
        "请把下面的中文旁白重新切分为字幕行。要求：\n\
1. 只返回 JSON 字符串数组，不要 markdown，不要时间戳。\n\
2. 必须覆盖完整旁白，不能省略、改写、乱序。\n\
3. 每一行必须是一句或半句，不能把多句放到同一行。\n\
4. 每一行尽量 8 到 18 个中文字符，优先 8 到 12 个字；如果原句太长，请按语义润色成两句或多句，分别放到多行。\n\
5. 句号、问号、感叹号、分号只能出现在行尾，不能出现在一行中间。\n\
6. 每一行都必须以中文标点或常规标点结尾。\n\
7. 不要机械硬切词语，不要拆开人名、书名、固定词组，例如不要把“白血病”拆开。\n\
8. 不要大量生成两个字、四个字的碎片；少于 6 个汉字的行必须合并到上下文，除非是极少数刻意停顿。\n\
9. 不要输出出版社、出版时间、ISBN、书号、版次、定价等版权页信息。\n\n\
当前字幕仅供参考：\n{}\n\n\
完整旁白：\n{}",
        current_lines.join("\n"),
        narration
    );
    let content = call_ai(
        settings,
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是中文短视频字幕编辑，只返回 JSON 字符串数组。".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    )
    .await
    .ok()?;
    let json = extract_json_array(&content)?;
    let parsed = serde_json::from_str::<Vec<String>>(&json).ok()?;
    let lines = finalize_subtitle_lines(
        parsed
            .into_iter()
            .map(|line| clean_subtitle_line(&line))
            .filter(|line| !line.is_empty())
            .collect(),
    );
    if lines.len() < 8 || !subtitle_coverage_ok(&lines, narration) {
        return None;
    }
    if lines.iter().any(|line| !subtitle_line_format_ok(line)) {
        return None;
    }
    Some(lines)
}

fn subtitle_line_format_ok(line: &str) -> bool {
    let trimmed = line.trim();
    let han_chars = count_han_chars(trimmed);
    if trimmed.is_empty() || han_chars > SUBTITLE_MAX_CHARS {
        return false;
    }
    if han_chars < SUBTITLE_MIN_TAIL_CHARS {
        return false;
    }
    let chars = trimmed.chars().collect::<Vec<_>>();
    let Some(last) = chars.last().copied() else {
        return false;
    };
    if !is_subtitle_terminal_punctuation(last) {
        return false;
    }
    chars
        .iter()
        .take(chars.len().saturating_sub(1))
        .all(|ch| !is_sentence_terminal_punctuation(*ch))
}

fn subtitle_coverage_ok(lines: &[String], narration: &str) -> bool {
    let narration_han = count_han_chars(narration);
    if narration_han == 0 {
        return false;
    }
    let subtitle_han = count_han_chars(&lines.join(""));
    subtitle_han.saturating_mul(1000) >= narration_han.saturating_mul(995)
}

fn extract_json_array(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return Some(trimmed.to_string());
    }
    let start = trimmed.find('[')?;
    let end = trimmed.rfind(']')?;
    if end > start {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

fn merge_short_subtitle_tails(lines: Vec<String>, min_tail_chars: usize, max_chars: usize) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for line in lines {
        let line_len = line.chars().count();
        if line_len < min_tail_chars {
            if let Some(previous) = merged.last_mut() {
                let previous_len = previous.chars().count();
                if previous_len + line_len <= max_chars {
                    previous.push_str(&line);
                    continue;
                }
            }
        }
        merged.push(line);
    }
    merged
}

fn is_hard_subtitle_break(ch: char) -> bool {
    matches!(ch, '\u{3002}' | '\u{FF01}' | '\u{FF1F}' | '\u{FF1B}' | '!' | '?' | ';' | '\n' | '\r')
}

fn is_soft_subtitle_break(ch: char) -> bool {
    matches!(ch, '\u{FF0C}' | '\u{3001}' | '\u{FF1A}' | ':' | ',' | '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}')
}

fn is_subtitle_split_punctuation(ch: char) -> bool {
    is_hard_subtitle_break(ch) || is_soft_subtitle_break(ch)
}

fn is_subtitle_terminal_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '\u{3002}'
            | '\u{FF0C}'
            | '\u{3001}'
            | '\u{FF01}'
            | '\u{FF1F}'
            | '\u{FF1B}'
            | '\u{FF1A}'
            | '\u{2026}'
            | '.'
            | ','
            | '!'
            | '?'
            | ';'
            | ':'
    )
}

fn is_sentence_terminal_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '\u{3002}' | '\u{FF01}' | '\u{FF1F}' | '\u{FF1B}' | '.' | '!' | '?' | ';'
    )
}

fn punctuation_count(value: &str) -> usize {
    value
        .chars()
        .filter(|ch| is_subtitle_split_punctuation(*ch) || matches!(*ch, '\u{300A}' | '\u{300B}' | '\u{2026}' | '.'))
        .count()
}

fn best_subtitle_split(chars: &[char], start: usize, max_chars: usize) -> usize {
    let remaining = chars.len() - start;
    if remaining <= max_chars {
        return chars.len();
    }
    let mut end = start + max_chars;
    let tail_len = chars.len() - end;
    if tail_len > 0 && tail_len < SUBTITLE_MIN_TAIL_CHARS {
        end = chars.len().saturating_sub(SUBTITLE_MIN_TAIL_CHARS).max(start + 1);
    }
    let search_start = (start + SUBTITLE_SOFT_CHARS).min(end);
    for index in (search_start..end).rev() {
        if is_subtitle_split_punctuation(chars[index]) {
            return index + 1;
        }
    }
    end
}

fn push_subtitle_chunks(lines: &mut Vec<String>, text: &str, max_chars: usize) {
    let chars: Vec<char> = text.trim().chars().collect();
    if chars.len() <= max_chars {
        if !chars.is_empty() {
            lines.push(chars.iter().collect::<String>());
        }
        return;
    }
    let mut start = 0usize;
    while start < chars.len() {
        let end = best_subtitle_split(&chars, start, max_chars);
        let line = chars[start..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
        if !line.is_empty() {
            lines.push(line);
        }
        start = end;
    }
}

fn resolve_export_base_dir(data: &AppData, output_dir: &str) -> Result<PathBuf, CommandError> {
    if output_dir.is_empty() {
        let parent = data
            .db_path
            .parent()
            .ok_or_else(|| command_error("\u{65e0}\u{6cd5}\u{786e}\u{5b9a}\u{5e94}\u{7528}\u{6570}\u{636e}\u{76ee}\u{5f55}\u{3002}"))?;
        return Ok(parent.join("exports"));
    }
    Ok(PathBuf::from(output_dir))
}

fn stage_dir(root: &Path, stage: &str) -> PathBuf {
    root.join(stage)
}

fn staged_material_path(root: &Path, file_name: &str) -> PathBuf {
    let content_path = stage_dir(root, STAGE_CONTENT_DIR).join(file_name);
    if content_path.exists() {
        content_path
    } else {
        root.join(file_name)
    }
}

fn has_material_package_files(path: &Path) -> bool {
    path.join(STAGE_CONTENT_DIR).join("materials.json").exists()
        || path.join(STAGE_CONTENT_DIR).join("narration.txt").exists()
        || path.join("materials.json").exists()
        || path.join("narration.txt").exists()
}

fn write_book_materials_package(
    data: &AppData,
    output_dir: &str,
    materials: &BookMaterials,
    trace_id: &str,
) -> Result<ExportBookMaterialsResult, CommandError> {
    let base_dir = resolve_export_base_dir(data, output_dir)?;
    fs::create_dir_all(&base_dir).map_err(|error| {
        command_error(format!("Operation failed: {error}"))
    })?;

    let output_dir = base_dir;
    let content_dir = output_dir.join(STAGE_CONTENT_DIR);
    fs::create_dir_all(&content_dir).map_err(|error| {
        command_error(format!("Operation failed: {error}"))
    })?;

    let mut files = Vec::new();
    write_material_file(&content_dir, &mut files, "title.txt", &materials.video_title)?;
    write_material_file(
        &content_dir,
        &mut files,
        "description.txt",
        &materials.description,
    )?;
    write_material_file(
        &content_dir,
        &mut files,
        "tags.txt",
        &materials.tags.join(", "),
    )?;
    write_material_file(
        &content_dir,
        &mut files,
        "narration.txt",
        &materials.narration,
    )?;
    write_material_file(
        &content_dir,
        &mut files,
        "subtitles.txt",
        &materials.subtitles.join("\n"),
    )?;
    write_material_file(
        &content_dir,
        &mut files,
        "draft.srt",
        &build_srt(&materials.subtitles),
    )?;
    write_material_file(&content_dir, &mut files, "prompt.txt", &materials.prompt)?;
    write_material_file(
        &content_dir,
        &mut files,
        "overview.json",
        &serde_json::to_string_pretty(&materials.overview).unwrap_or_default(),
    )?;
    write_material_file(
        &content_dir,
        &mut files,
        "materials.json",
        &serde_json::to_string_pretty(materials).unwrap_or_default(),
    )?;
    write_material_file(
        &content_dir,
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
            content_dir.to_string_lossy()
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
        r#"Operation completed."<>|
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
                "Operation failed: {error}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_generated_narration_removes_publisher_metadata_and_duplicates() {
        let value = "今天我们一起读这本书。出版社是浙江文艺出版社，果麦文化。出版时间，是二零二五年六月。ISBN，是九七八七五三三九七九三五五。故事真正重要的，是人物如何面对失去。故事真正重要的，是人物如何面对失去。";

        let sanitized = sanitize_generated_narration(value);

        assert!(!sanitized.contains("浙江文艺出版社"));
        assert!(!sanitized.contains("果麦文化"));
        assert!(!sanitized.contains("ISBN"));
        assert!(!sanitized.contains("九七八七五三三九七九三五五"));
        assert_eq!(sanitized.matches("故事真正重要的").count(), 1);
    }

    #[test]
    fn split_subtitles_merges_tiny_fragments() {
        let lines = split_subtitles("后来，他终于明白。爱不是答案，而是继续走下去的勇气。");

        assert!(lines.iter().all(|line| count_han_chars(line) >= SUBTITLE_MIN_TAIL_CHARS));
        assert!(lines.iter().all(|line| count_han_chars(line) <= SUBTITLE_MAX_CHARS));
    }
}
