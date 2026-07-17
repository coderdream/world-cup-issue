use crate::models::{
    AiGenerateRequest, AiGenerateResult, AiTestResult, AppSettings, AppStatePayload, ChatCompletionRequest, ChatCompletionResponse,
    ChatMessage, FeishuSendRequest, FeishuSendResult, FeishuWebhookResponse, GetOperationLogsRequest, GetOperationLogsResult,
    OperationEventEntry, OperationHistoryEntry, OperationLogEntry, OperationStepEntry, QuarkStatus, RunWorkflowRequest,
    RunWorkflowResult, SkillConfigEntry, UpdateInfo, VideoCreatorDashboard,
};
use crate::operation_log::OperationLogger;
use chrono::{DateTime, Duration, Local, NaiveDateTime};
use encoding_rs::GBK;
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};

const LEGACY_DB: &str = r"D:\04_GitHub\world-cup-issue\video-creator\data\video-easy-creator.db";
const LEGACY_RUNTIME_LOG: &str = r"D:\04_GitHub\world-cup-issue\video-creator\logs\app\runtime.log";

pub struct AppData {
    settings: Mutex<AppSettings>,
    settings_path: PathBuf,
    db_path: PathBuf,
    logger: OperationLogger,
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

        let settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str::<AppSettings>(&content).ok())
            .unwrap_or_default();

        logger.info("app", "startup", "Video Creator started");

        Self {
            settings: Mutex::new(settings),
            settings_path,
            db_path,
            logger,
        }
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<(), CommandError> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| command_error(format!("Failed to create settings directory: {error}")))?;
        }
        let content = serde_json::to_string_pretty(settings).map_err(|error| command_error(format!("Failed to serialize settings: {error}")))?;
        fs::write(&self.settings_path, content).map_err(|error| command_error(format!("Failed to save settings: {error}")))?;
        self.logger.info("settings", "save", "Settings saved");
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
    data.logger.info("app", "get_app_state", "Read app state");
    Ok(AppStatePayload {
        settings: data.settings.lock().map_err(lock_error)?.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
pub fn get_settings(data: State<'_, AppData>) -> Result<AppSettings, CommandError> {
    data.logger.info("settings", "get", "Read settings");
    Ok(data.settings.lock().map_err(lock_error)?.clone())
}

#[tauri::command]
pub fn set_settings(data: State<'_, AppData>, settings: AppSettings) -> Result<AppSettings, CommandError> {
    let mut current = data.settings.lock().map_err(lock_error)?;
    *current = settings.clone();
    data.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn check_update_mock(data: State<'_, AppData>) -> UpdateInfo {
    data.logger.info("update", "check", "Check update");
    UpdateInfo {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: env!("CARGO_PKG_VERSION").to_string(),
        available: false,
        notes: "Video Creator is already up to date.".to_string(),
    }
}

#[tauri::command]
pub fn get_video_creator_dashboard(data: State<'_, AppData>) -> Result<VideoCreatorDashboard, CommandError> {
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    data.logger.info("video", "dashboard", "Read dashboard");
    Ok(build_dashboard(&settings))
}

#[tauri::command]
pub fn save_skill_configs(data: State<'_, AppData>, skills: Vec<SkillConfigEntry>) -> Result<Vec<SkillConfigEntry>, CommandError> {
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let path = Path::new(&settings.java_project_dir).join("data").join("video-creator-skills.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| command_error(format!("Failed to create skill config directory: {error}")))?;
    }
    let mut sorted = skills;
    sorted.sort_by_key(|item| item.sort_order);
    let content = serde_json::to_string_pretty(&sorted).map_err(|error| command_error(format!("Failed to serialize skill config: {error}")))?;
    fs::write(&path, content).map_err(|error| command_error(format!("Failed to save skill config: {error}")))?;
    data.logger.info("skills", "save", "Skill config saved");
    Ok(sorted)
}

#[tauri::command]
pub fn run_video_workflow(data: State<'_, AppData>, request: RunWorkflowRequest) -> Result<RunWorkflowResult, CommandError> {
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    run_video_workflow_in_background(&settings, &data.logger, request)
}

pub fn run_video_workflow_in_background(
    settings: &AppSettings,
    logger: &OperationLogger,
    request: RunWorkflowRequest,
) -> Result<RunWorkflowResult, CommandError> {
    let command = request.command.trim();
    if command.is_empty() {
        return Err(command_error("Command cannot be empty."));
    }
    let project_dir = resolve_legacy_project_dir(settings);
    if !project_dir.exists() {
        return Err(command_error(format!("Legacy Java project directory does not exist: {}", project_dir.display())));
    }
    let runtime_dir = resolve_legacy_runtime_dir(settings);
    if !runtime_dir.exists() {
        return Err(command_error(format!("Legacy Java runtime directory does not exist: {}", runtime_dir.display())));
    }

    let args = build_legacy_args(settings, &request);
    let classpath = legacy_classpath(&project_dir);
    let logger = logger.clone();
    let settings_for_background = settings.clone();
    let command_name = command.to_string();
    let args_text = args.join(" ");
    let sync_episode = request.episode.clone();
    logger.info("video", "run_workflow", &format!("Background task submitted: {args_text}"));

    thread::Builder::new()
        .name(format!("video-workflow-{command_name}"))
        .spawn(move || {
            logger.info("video", "run_workflow", &format!("Background task started: {args_text}"));
            let output = Command::new("java")
                .current_dir(&runtime_dir)
                .arg("-Dfile.encoding=UTF-8")
                .arg("-Dsun.stdout.encoding=UTF-8")
                .arg("-Dsun.stderr.encoding=UTF-8")
                .arg("-cp")
                .arg(classpath)
                .arg("com.coderdream.app.VideoEasyCreatorLauncher")
                .args(&args)
                .output();

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let exit_code = output.status.code();
                    if output.status.success() {
                        logger.info("video", "run_workflow", &format!("Background task {command_name} completed."));
                        if command_name == "prepare-sixminutes" {
                            if let Some(episode) = sync_episode.as_deref().filter(|value| !value.trim().is_empty()) {
                                match sync_jianying_draft(&settings_for_background, episode) {
                                    Ok(summary) => logger.info("video", "sync_jianying_draft", &summary),
                                    Err(error) => logger.error("video", "sync_jianying_draft", "Failed to sync Jianying draft", error),
                                }
                            }
                        }
                        if !stdout.trim().is_empty() {
                            logger.trace_info("video", "run_workflow_stdout", "Background task stdout", stdout.trim(), &command_name);
                        }
                    } else {
                        let message = format!(
                            "Background task {command_name} failed with exit code {}.",
                            exit_code.map_or_else(|| "unknown".to_string(), |value| value.to_string())
                        );
                        logger.error("video", "run_workflow", &message, stderr.trim());
                    }
                }
                Err(error) => {
                    logger.error("video", "run_workflow", &format!("Failed to start background task {command_name}."), error.to_string());
                }
            }
        })
        .map_err(|error| command_error(format!("Failed to create background task: {error}")))?;

    let message = format!("Command {command} has started in the background.");

    Ok(RunWorkflowResult {
        ok: true,
        message,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
    })
}

#[tauri::command]
pub fn open_video_creator_path(data: State<'_, AppData>, target: String) -> Result<(), CommandError> {
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let path = resolve_open_target(&settings, &target);
    if !path.exists() {
        return Err(command_error(format!("Path does not exist: {}", path.display())));
    }
    Command::new("explorer")
        .arg(&path)
        .spawn()
        .map_err(|error| command_error(format!("Failed to open path: {error}")))?;
    Ok(())
}

#[tauri::command]
pub async fn test_ai_profile(data: State<'_, AppData>) -> Result<AiTestResult, CommandError> {
    data.logger.info("ai", "test_profile", "Test AI profile");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let content = match call_ai(
        &settings,
        vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello, reply only with ok".to_string(),
        }],
    )
    .await
    {
        Ok(content) => content,
        Err(error) => {
            data.logger.error("ai", "test_profile", "AI profile test failed", &error.message);
            return Err(error);
        }
    };

    data.logger.info("ai", "test_profile", "AI profile test succeeded");
    Ok(AiTestResult {
        ok: true,
        message: "AI profile test succeeded.".to_string(),
        content: Some(content),
    })
}

#[tauri::command]
pub async fn generate_ai_text(data: State<'_, AppData>, request: AiGenerateRequest) -> Result<AiGenerateResult, CommandError> {
    data.logger.info("ai", "generate_text", "Generate AI text");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let mut messages = Vec::new();
    if let Some(system_prompt) = request.system_prompt.filter(|value| !value.trim().is_empty()) {
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
            data.logger.error("ai", "generate_text", "AI text generation failed", &error.message);
            return Err(error);
        }
    };
    data.logger.info("ai", "generate_text", "AI text generation succeeded");
    Ok(AiGenerateResult {
        content,
        model: settings.ai_profile.model,
    })
}

#[tauri::command]
pub async fn test_feishu_profile(data: State<'_, AppData>) -> Result<FeishuSendResult, CommandError> {
    data.logger.info("feishu", "test_profile", "Test Feishu profile");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    let text = settings.feishu_profile.test_message.trim();
    let message = if text.is_empty() {
        "Feishu connectivity test succeeded.".to_string()
    } else {
        text.to_string()
    };

    match call_feishu(&settings, &message).await {
        Ok(result) => {
            data.logger.info("feishu", "test_profile", "Feishu profile test succeeded");
            Ok(result)
        }
        Err(error) => {
            data.logger.error("feishu", "test_profile", "Feishu profile test failed", &error.message);
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn send_feishu_message(data: State<'_, AppData>, request: FeishuSendRequest) -> Result<FeishuSendResult, CommandError> {
    data.logger.info("feishu", "send_message", "Send Feishu message");
    let settings = data.settings.lock().map_err(lock_error)?.clone();
    match call_feishu(&settings, &request.text).await {
        Ok(result) => {
            data.logger.info("feishu", "send_message", "Feishu message sent");
            Ok(result)
        }
        Err(error) => {
            data.logger.error("feishu", "send_message", "Failed to send Feishu message", &error.message);
            Err(error)
        }
    }
}

#[tauri::command]
pub fn get_operation_logs(data: State<'_, AppData>, request: GetOperationLogsRequest) -> Result<GetOperationLogsResult, CommandError> {
    let limit = request.limit.clamp(1, 1000);
    let trace_id = request.trace_id.as_deref().filter(|value| !value.trim().is_empty());
    let connection = Connection::open(&data.db_path).map_err(|error| command_error(format!("Failed to open operation log database: {error}")))?;
    let entries = if let Some(trace_id) = trace_id {
        query_operation_logs_by_trace(&connection, limit, trace_id)?
    } else {
        query_operation_logs(&connection, limit)?
    };
    Ok(GetOperationLogsResult { entries })
}

fn build_dashboard(settings: &AppSettings) -> VideoCreatorDashboard {
    let history = read_history_entries();
    let latest = history.first().cloned();
    let steps = read_step_entries(latest.as_ref().map(|item| item.id));
    let event_logs = read_event_entries(latest.as_ref().map(|item| item.id));
    let runtime_logs = tail_lines(Path::new(LEGACY_RUNTIME_LOG), 260);
    let skills = read_skill_configs(settings);
    let quark = build_quark_status(settings);
    let successful_steps = steps.iter().filter(|item| item.status.eq_ignore_ascii_case("SUCCESS")).count();
    let failed_steps = steps.iter().filter(|item| item.status.eq_ignore_ascii_case("FAILED")).count();
    let running_steps = steps.iter().filter(|item| item.status.eq_ignore_ascii_case("RUNNING")).count();
    let latest_status = latest.as_ref().map(|item| normalize_status(&item.status)).unwrap_or_else(|| "PENDING".to_string());

    VideoCreatorDashboard {
        current_task: latest.as_ref().map(|item| item.id.to_string()).unwrap_or_else(|| "-".to_string()),
        latest_episode: latest
            .as_ref()
            .map(|item| item.episode_code.clone())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| settings.default_episode.clone()),
        latest_duration_ms: latest.as_ref().map(|item| item.duration_ms).unwrap_or_default(),
        latest_step_count: steps.len(),
        latest_status,
        total_steps: steps.len().max(22),
        successful_steps,
        failed_steps,
        running_steps,
        summary: latest
            .as_ref()
            .map(|item| item.summary.clone())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Waiting for manual task execution".to_string()),
        vpn_status: detect_vpn_status(),
        runtime_log_path: LEGACY_RUNTIME_LOG.to_string(),
        recent_history: history.into_iter().take(60).collect(),
        steps,
        skills,
        event_logs,
        runtime_logs,
        quark,
    }
}

fn read_history_entries() -> Vec<OperationHistoryEntry> {
    let Ok(connection) = Connection::open(LEGACY_DB) else {
        return sample_history();
    };
    let sql = r#"
        SELECT id, operation_key, episode_code, status, current_stage, summary, started_at, finished_at, duration_ms
        FROM operation_history
        ORDER BY id DESC
        LIMIT 120
    "#;
    let Ok(mut statement) = connection.prepare(sql) else {
        return sample_history();
    };
    let Ok(rows) = statement.query_map([], |row| {
        Ok(OperationHistoryEntry {
            id: row.get(0)?,
            ability: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            episode_code: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            status: normalize_status(&row.get::<_, Option<String>>(3)?.unwrap_or_default()),
            current_stage: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            summary: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            started_at: legacy_time_to_beijing(&row.get::<_, Option<String>>(6)?.unwrap_or_default()),
            finished_at: legacy_time_to_beijing(&row.get::<_, Option<String>>(7)?.unwrap_or_default()),
            duration_ms: row.get::<_, Option<i64>>(8)?.unwrap_or_default(),
        })
    }) else {
        return sample_history();
    };
    rows.filter_map(Result::ok).collect()
}

fn read_step_entries(operation_id: Option<i64>) -> Vec<OperationStepEntry> {
    let Some(operation_id) = operation_id else {
        return sample_steps();
    };
    let Ok(connection) = Connection::open(LEGACY_DB) else {
        return sample_steps();
    };
    let sql = r#"
        SELECT step_order, step_code, step_name, status, started_at, finished_at, duration_ms, detail
        FROM operation_step
        WHERE operation_id = ?1
        ORDER BY step_order ASC, id ASC
    "#;
    let Ok(mut statement) = connection.prepare(sql) else {
        return sample_steps();
    };
    let Ok(rows) = statement.query_map([operation_id], |row| {
        Ok(OperationStepEntry {
            seq: row.get::<_, Option<i64>>(0)?.unwrap_or_default().max(0) as usize,
            code: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            status: normalize_status(&row.get::<_, Option<String>>(3)?.unwrap_or_default()),
            started_at: legacy_time_to_beijing(&row.get::<_, Option<String>>(4)?.unwrap_or_default()),
            finished_at: legacy_time_to_beijing(&row.get::<_, Option<String>>(5)?.unwrap_or_default()),
            duration_ms: row.get::<_, Option<i64>>(6)?.unwrap_or_default(),
            description: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
        })
    }) else {
        return sample_steps();
    };
    let entries: Vec<_> = rows.filter_map(Result::ok).collect();
    if entries.is_empty() { sample_steps() } else { entries }
}

fn read_event_entries(operation_id: Option<i64>) -> Vec<OperationEventEntry> {
    let Some(operation_id) = operation_id else {
        return Vec::new();
    };
    let Ok(connection) = Connection::open(LEGACY_DB) else {
        return Vec::new();
    };
    let sql = r#"
        SELECT created_at, level, stage, message
        FROM operation_event
        WHERE operation_id = ?1
        ORDER BY id ASC
        LIMIT 500
    "#;
    let Ok(mut statement) = connection.prepare(sql) else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map([operation_id], |row| {
        Ok(OperationEventEntry {
            created_at: legacy_time_to_beijing(&row.get::<_, Option<String>>(0)?.unwrap_or_default()),
            level: row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "INFO".to_string()),
            stage: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            message: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        })
    }) else {
        return Vec::new();
    };
    rows.filter_map(Result::ok).collect()
}

fn read_skill_configs(settings: &AppSettings) -> Vec<SkillConfigEntry> {
    let path = Path::new(&settings.java_project_dir).join("data").join("video-creator-skills.json");
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(skills) = serde_json::from_str::<Vec<SkillConfigEntry>>(&content) {
            return skills;
        }
    }
    default_skills()
}

fn build_quark_status(settings: &AppSettings) -> QuarkStatus {
    let runtime_dir = resolve_legacy_runtime_dir(settings);
    let cookie_file = runtime_dir.join("auth").join("cookie").join("quark").join("cookies.txt");
    let updated = fs::metadata(&cookie_file)
        .and_then(|meta| meta.modified())
        .ok()
        .map(format_system_time)
        .unwrap_or_else(|| "-".to_string());
    let runtime_log = runtime_dir.join("logs").join("app").join("runtime.log");
    let logs = tail_lines(&runtime_log, 80)
        .into_iter()
        .filter(|line| line.to_lowercase().contains("quark"))
        .collect::<Vec<_>>();
    QuarkStatus {
        token_valid: if cookie_file.exists() { "待校验".to_string() } else { "否".to_string() },
        cookie_file: cookie_file.display().to_string(),
        cookie_updated_at: updated,
        root_item_count: 0,
        latest_result: "启动不自动续跑任务，请手动点击 Quark 操作。".to_string(),
        logs,
    }
}

fn format_system_time(time: SystemTime) -> String {
    let local: DateTime<Local> = DateTime::from(time);
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn build_legacy_args(settings: &AppSettings, request: &RunWorkflowRequest) -> Vec<String> {
    let mut args = vec![request.command.clone()];
    match request.command.as_str() {
        "daily-sync" => {
            let years = request.years.as_deref().unwrap_or(&settings.quark_years);
            args.extend(split_list(years));
        }
        "ppt-preview" => {
            if let Some(preview_type) = request.preview_type.as_deref().filter(|value| !value.trim().is_empty()) {
                args.push(format!("--preview-type={preview_type}"));
            }
            args.extend(split_list(request.episode.as_deref().unwrap_or(&settings.default_episode)));
        }
        "six-minutes-codex" | "six-minutes-minimax" => {
            args.push(format!("--folder={}", request.episode.as_deref().unwrap_or(&settings.default_episode)));
            if request.prepare_publish_materials == Some(false) {
                args.push("--skip-prepare".to_string());
            }
        }
        _ => {
            args.extend(split_list(request.episode.as_deref().unwrap_or(&settings.default_episode)));
            if request.prepare_publish_materials == Some(false) && request.command == "one-click" {
                args.push("--skip-prepare".to_string());
            }
        }
    }
    args
}

fn legacy_classpath(project_dir: &Path) -> String {
    [
        project_dir.join("target").join("classes"),
        project_dir.join("target").join("app-libs").join("*"),
        project_dir.join("lib").join("aspose-slides-24.5-jdk16.jar"),
    ]
    .into_iter()
    .map(|path| path.display().to_string())
    .collect::<Vec<_>>()
    .join(";")
}

fn resolve_open_target(settings: &AppSettings, target: &str) -> PathBuf {
    let base = resolve_legacy_project_dir(settings);
    let runtime = resolve_legacy_runtime_dir(settings);
    match target {
        "output" => PathBuf::from(&settings.output_dir),
        "ppt_config" => base.join("PPT_TEMPLATE.md"),
        "quark_cookie" => runtime.join("auth").join("cookie").join("quark"),
        "quark_sync" => PathBuf::from(&settings.output_dir),
        "legacy_project" => base.to_path_buf(),
        "legacy_runtime" => runtime,
        _ => PathBuf::from(target),
    }
}

fn resolve_legacy_project_dir(settings: &AppSettings) -> PathBuf {
    let configured = PathBuf::from(settings.java_project_dir.trim());
    if configured.join("target").join("classes").exists() {
        return configured;
    }
    let fallback = PathBuf::from(r"D:\04_GitHub\video-easy-creator");
    if fallback.join("target").join("classes").exists() {
        return fallback;
    }
    configured
}

fn resolve_legacy_runtime_dir(settings: &AppSettings) -> PathBuf {
    let configured = PathBuf::from(settings.java_runtime_dir.trim());
    if configured.exists() {
        return configured;
    }
    let fallback = PathBuf::from(r"D:\05_Green\VideoEasyCreator-Portable");
    if fallback.exists() {
        return fallback;
    }
    resolve_legacy_project_dir(settings)
}

fn default_skills() -> Vec<SkillConfigEntry> {
    vec![
        skill("one-click", "一键执行", "one-click", true, 10, "写入 todo、下载资源、生成脚本并继续后续流程"),
        skill("bbc-prefetch", "BBC 预下载", "bbc-prefetch", true, 20, "下载图片、音频和 PDF"),
        skill("script-text", "生成脚本", "script-text", true, 30, "从 PDF 生成脚本文本"),
        skill("question-title", "疑问句标题", "question-title", true, 35, "生成中文疑问句标题"),
        skill("six-minutes-codex", "Codex 工作流", "six-minutes-codex", true, 40, "执行 BBC 六分钟视频创作流程"),
        skill("prepare-sixminutes", "发布素材", "prepare-sixminutes", true, 60, "整理发布所需素材"),
        skill("daily-sync", "Daily 同步", "daily-sync", true, 70, "按年份同步 Daily 文件"),
    ]
}

fn skill(key: &str, title: &str, command: &str, enabled: bool, sort_order: i64, description: &str) -> SkillConfigEntry {
    SkillConfigEntry {
        key: key.to_string(),
        title: title.to_string(),
        command: command.to_string(),
        enabled,
        sort_order,
        description: description.to_string(),
    }
}

fn sample_history() -> Vec<OperationHistoryEntry> {
    vec![OperationHistoryEntry {
        id: 0,
        ability: "one-click".to_string(),
        episode_code: "260625".to_string(),
        status: "PENDING".to_string(),
        current_stage: "INIT".to_string(),
        summary: "等待手动执行任务".to_string(),
        started_at: "-".to_string(),
        finished_at: "-".to_string(),
        duration_ms: 0,
    }]
}

fn sample_steps() -> Vec<OperationStepEntry> {
    let names = [
        ("TODO", "写入 todo"),
        ("DOWNLOAD", "下载 BBC 资源"),
        ("SCRIPT", "生成脚本"),
        ("TITLE", "生成疑问句标题"),
        ("STEP01", "预处理原始脚本"),
        ("STEP02", "生成对话脚本"),
        ("STEP03", "生成问答脚本"),
        ("STEP04", "翻译对话脚本"),
        ("STEP05", "翻译问答脚本"),
        ("STEP06", "生成优化脚本"),
        ("STEP07", "合并双语脚本"),
        ("STEP08", "生成原始 SRT 字幕"),
        ("STEP09", "提取时间戳并切割音频"),
        ("STEP10", "生成最终 SRT 脚本"),
        ("STEP11", "生成最终英文字幕"),
        ("STEP12", "翻译为中文字幕"),
        ("STEP13", "合并双语字幕"),
        ("STEP14", "生成词汇表"),
        ("STEP15", "检查/生成 PPT"),
        ("STEP16", "检查/生成 PPT 截图"),
        ("STEP17", "生成描述文件"),
        ("STEP18", "整理发布素材"),
    ];
    names
        .iter()
        .enumerate()
        .map(|(index, (code, name))| OperationStepEntry {
            seq: index + 1,
            code: (*code).to_string(),
            name: (*name).to_string(),
            status: "PENDING".to_string(),
            started_at: "-".to_string(),
            finished_at: "-".to_string(),
            duration_ms: 0,
            description: "等待手动执行".to_string(),
        })
        .collect()
}

fn legacy_time_to_beijing(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "-" {
        return trimmed.to_string();
    }
    NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S")
        .map(|time| (time + Duration::hours(8)).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| trimmed.to_string())
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split([',', ';', ' '])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn tail_lines(path: &Path, limit: usize) -> Vec<String> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut lines: Vec<String> = content.lines().rev().take(limit).map(ToString::to_string).collect();
    lines.reverse();
    lines
}

fn normalize_status(status: &str) -> String {
    if status.eq_ignore_ascii_case("generating") || status.eq_ignore_ascii_case("RUNNING") {
        "PENDING".to_string()
    } else if status.trim().is_empty() {
        "PENDING".to_string()
    } else {
        status.to_ascii_uppercase()
    }
}

fn detect_vpn_status() -> String {
    match std::net::TcpStream::connect_timeout(&"127.0.0.1:1080".parse().unwrap(), std::time::Duration::from_millis(350)) {
        Ok(_) => "OK (1080)".to_string(),
        Err(_) => "not connected".to_string(),
    }
}

fn query_operation_logs(connection: &Connection, limit: usize) -> Result<Vec<OperationLogEntry>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, created_at, level, module, action, message, detail, trace_id
            FROM operate_log
            ORDER BY id DESC
            LIMIT ?1
            "#,
        )
        .map_err(|error| command_error(format!("Failed to prepare operation log query: {error}")))?;
    let rows = statement
        .query_map([limit as i64], operation_log_from_row)
        .map_err(|error| command_error(format!("Failed to query operation logs: {error}")))?;
    collect_operation_logs(rows)
}

fn query_operation_logs_by_trace(connection: &Connection, limit: usize, trace_id: &str) -> Result<Vec<OperationLogEntry>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, created_at, level, module, action, message, detail, trace_id
            FROM operate_log
            WHERE trace_id = ?1
            ORDER BY id DESC
            LIMIT ?2
            "#,
        )
        .map_err(|error| command_error(format!("Failed to prepare trace log query: {error}")))?;
    let rows = statement
        .query_map((trace_id, limit as i64), operation_log_from_row)
        .map_err(|error| command_error(format!("Failed to query trace logs: {error}")))?;
    collect_operation_logs(rows)
}

fn collect_operation_logs<F>(rows: rusqlite::MappedRows<'_, F>) -> Result<Vec<OperationLogEntry>, CommandError>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<OperationLogEntry>,
{
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|error| command_error(format!("Failed to read operation log row: {error}")))?);
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

async fn call_ai(settings: &AppSettings, messages: Vec<ChatMessage>) -> Result<String, CommandError> {
    let profile = &settings.ai_profile;
    if profile.api_key.trim().is_empty() {
        return Err(command_error("Please set AI API Key first."));
    }
    if profile.base_url.trim().is_empty() {
        return Err(command_error("Please set AI Base URL first."));
    }
    if profile.model.trim().is_empty() {
        return Err(command_error("Please set AI model first."));
    }

    let base = profile.base_url.trim().trim_end_matches('/');
    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    };

    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(profile.api_key.trim())
        .json(&ChatCompletionRequest {
            model: profile.model.clone(),
            messages,
            stream: false,
        })
        .send()
        .await
        .map_err(|error| command_error(format!("AI request failed: {error}")))?
        .error_for_status()
        .map_err(|error| command_error(format!("AI service returned an error: {error}")))?
        .json::<ChatCompletionResponse>()
        .await
        .map_err(|error| command_error(format!("Failed to parse AI response: {error}")))?;

    response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| command_error("AI service returned no content."))
}

async fn call_feishu(settings: &AppSettings, text: &str) -> Result<FeishuSendResult, CommandError> {
    let profile = &settings.feishu_profile;
    let webhook_url = profile.webhook_url.trim();
    if webhook_url.is_empty() {
        return Err(command_error("Please set Feishu Webhook URL first."));
    }
    if !webhook_url.starts_with("http://") && !webhook_url.starts_with("https://") {
        return Err(command_error("Feishu Webhook URL must start with http:// or https://."));
    }

    let message = text.trim();
    if message.is_empty() {
        return Err(command_error("Please enter a Feishu message first."));
    }

    let title = profile.title.trim();
    let content = if title.is_empty() {
        message.to_string()
    } else {
        format!("[{title}]\n{message}")
    };

    let response = reqwest::Client::new()
        .post(webhook_url)
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Accept", "application/json")
        .header("User-Agent", "VideoCreator/0.1")
        .json(&serde_json::json!({
            "msg_type": "text",
            "content": {
                "text": content,
            }
        }))
        .send()
        .await
        .map_err(|error| command_error(format!("Feishu request failed: {error}")))?
        .error_for_status()
        .map_err(|error| command_error(format!("Feishu service returned an error: {error}")))?;

    let body = response
        .json::<FeishuWebhookResponse>()
        .await
        .map_err(|error| command_error(format!("Failed to parse Feishu response: {error}")))?;

    let code = body.code.unwrap_or(-1);
    if code != 0 {
        let msg = body.msg.unwrap_or_else(|| "unknown error".to_string());
        return Err(command_error(format!("Feishu bot returned error: code={code}, msg={msg}")));
    }

    Ok(FeishuSendResult {
        ok: true,
        message: "Feishu connectivity test succeeded; message sent.".to_string(),
    })
}

#[derive(Debug, Clone)]
struct SrtCue {
    start_us: i64,
    duration_us: i64,
    text: String,
}

fn sync_jianying_draft(settings: &AppSettings, episode: &str) -> Result<String, String> {
    let publish_dir = Path::new(r"D:\0000\video\0001_SixMinutes_Draft");
    let draft_dir = resolve_jianying_draft_dir(settings, episode)?;
    if !draft_dir.exists() {
        return Err(format!("剪映草稿目录不存在：{}", draft_dir.display()));
    }

    let cover_path = publish_dir.join("cover.png");
    let eng_srt_path = publish_dir.join("eng.srt");
    let chn_srt_path = publish_dir.join("chn.srt");
    for path in [&cover_path, &eng_srt_path, &chn_srt_path] {
        if !path.exists() {
            return Err(format!("发布素材缺失：{}", path.display()));
        }
    }

    repair_mojibake_srt(&chn_srt_path)?;
    sync_draft_cover(&draft_dir, &cover_path)?;
    sync_draft_subtitles(&draft_dir, &eng_srt_path, &chn_srt_path)?;
    sync_draft_media_library(&draft_dir, publish_dir)?;
    Ok(format!("已同步剪映草稿封面和字幕：{}", draft_dir.display()))
}

fn resolve_jianying_draft_dir(settings: &AppSettings, episode: &str) -> Result<PathBuf, String> {
    let configured = PathBuf::from(settings.jianying_draft_dir.trim());
    if configured.exists() {
        return Ok(configured);
    }

    let root = Path::new(r"D:\03_Software\JianyingPro Drafts");
    if !root.exists() {
        return Err(format!("剪映草稿根目录不存在：{}", root.display()));
    }

    let mut candidates = Vec::new();
    let trimmed = episode.trim();
    if trimmed.len() >= 4 {
        candidates.push(root.join(format!("六分钟英语_{}", &trimmed[..4])));
    }
    candidates.push(root.join("六分钟英语_2606"));

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    fs::read_dir(root)
        .map_err(|error| format!("读取剪映草稿目录失败：{error}"))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().map(|file_type| file_type.is_dir()).unwrap_or(false)
                && entry.file_name().to_string_lossy().starts_with("六分钟英语_")
        })
        .max_by_key(|entry| entry.metadata().and_then(|metadata| metadata.modified()).ok())
        .map(|entry| entry.path())
        .ok_or_else(|| "没有找到 六分钟英语_* 剪映草稿".to_string())
}

fn sync_draft_cover(draft_dir: &Path, cover_path: &Path) -> Result<(), String> {
    let content_path = draft_dir.join("draft_content.json");
    let meta_path = draft_dir.join("draft_meta_info.json");
    let content = read_json_file(&content_path)?;
    let cover_resource = content
        .pointer("/materials/drafts/0/draft/materials/videos/0/path")
        .and_then(Value::as_str)
        .and_then(|value| value.split("##/").nth(1))
        .map(|relative| draft_dir.join(relative.replace('/', "\\")));

    if let Some(path) = cover_resource {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(cover_path, &path).map_err(|error| format!("覆盖草稿内部封面失败：{error}"))?;
    }

    for name in ["draft_cover.jpg", "draft_local_cover.jpg"] {
        let target = draft_dir.join(name);
        if target.exists() {
            fs::copy(cover_path, target).map_err(|error| format!("覆盖草稿封面缩略图失败：{error}"))?;
        }
    }

    if meta_path.exists() {
        let mut meta = read_json_file(&meta_path)?;
        meta["draft_cover"] = Value::String("draft_cover.jpg".to_string());
        write_json_file(&meta_path, &meta)?;
    }
    Ok(())
}

fn sync_draft_subtitles(draft_dir: &Path, eng_srt_path: &Path, chn_srt_path: &Path) -> Result<(), String> {
    let content_path = draft_dir.join("draft_content.json");
    let mut draft = read_json_file(&content_path)?;
    let english = parse_srt_file(eng_srt_path)?;
    let chinese = parse_srt_file(chn_srt_path)?;
    if english.is_empty() || chinese.is_empty() {
        return Err("字幕文件为空，无法同步到剪映草稿".to_string());
    }

    let tracks = draft
        .get_mut("tracks")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "draft_content.json 缺少 tracks".to_string())?;
    let text_track_indexes: Vec<usize> = tracks
        .iter()
        .enumerate()
        .filter_map(|(index, track)| (track.get("type").and_then(Value::as_str) == Some("text")).then_some(index))
        .collect();
    if text_track_indexes.len() < 2 {
        return Err("剪映草稿缺少双语字幕轨".to_string());
    }

    let mut text_ids = Vec::new();
    sync_text_track(&mut tracks[text_track_indexes[0]], &english, &mut text_ids)?;
    sync_text_track(&mut tracks[text_track_indexes[1]], &chinese, &mut text_ids)?;

    let texts = draft
        .pointer_mut("/materials/texts")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "draft_content.json 缺少 materials.texts".to_string())?;
    for cue in english.iter().chain(chinese.iter()) {
        let id = text_ids.remove(0);
        if let Some(material) = texts.iter_mut().find(|item| item.get("id").and_then(Value::as_str) == Some(id.as_str())) {
            update_text_material(material, &cue.text)?;
        }
    }

    let duration = english
        .iter()
        .chain(chinese.iter())
        .map(|cue| cue.start_us + cue.duration_us)
        .max()
        .unwrap_or(0);
    if duration > 0 {
        draft["duration"] = Value::from(duration);
        draft["tm_duration"] = Value::from(duration / 1000);
    }

    sync_draft_video_images(&mut draft, Path::new(r"D:\0000\video\0001_SixMinutes_Draft"))?;
    write_json_file(&content_path, &draft)
}

fn sync_draft_video_images(draft: &mut Value, publish_dir: &Path) -> Result<(), String> {
    let draft_duration = draft.get("duration").and_then(Value::as_i64).unwrap_or(0);
    let target_names = list_snapshot_names(publish_dir)?;
    if target_names.is_empty() {
        return Ok(());
    }

    let videos = draft
        .pointer_mut("/materials/videos")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "draft_content.json 缺少 materials.videos".to_string())?;
    let video_template = videos
        .first()
        .cloned()
        .ok_or_else(|| "剪映草稿缺少图片素材模板".to_string())?;

    while videos.len() < target_names.len() {
        videos.push(video_template.clone());
    }
    videos.truncate(target_names.len());

    let mut material_ids = Vec::with_capacity(target_names.len());
    for (index, name) in target_names.iter().enumerate() {
        let material = &mut videos[index];
        let id = stable_jianying_image_id(name);
        material["id"] = Value::String(id.clone());
        material["material_name"] = Value::String(name.clone());
        material["name"] = Value::String(name.clone());
        material["path"] = Value::String(format!("D:/0000/video/0001_SixMinutes_Draft/{name}"));
        material["media_path"] = Value::String(String::new());
        material["type"] = Value::String("photo".to_string());
        material["width"] = Value::from(1920);
        material["height"] = Value::from(1080);
        material_ids.push(id);
    }

    let tracks = draft
        .get_mut("tracks")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "draft_content.json 缺少 tracks".to_string())?;
    let video_track = tracks
        .iter_mut()
        .find(|track| track.get("type").and_then(Value::as_str) == Some("video"))
        .ok_or_else(|| "剪映草稿缺少视频轨".to_string())?;
    let segments = video_track
        .get_mut("segments")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "视频轨缺少 segments".to_string())?;
    let segment_template = segments
        .last()
        .cloned()
        .ok_or_else(|| "视频轨缺少图片片段模板".to_string())?;
    let original_len = segments.len();
    let original_end = segments
        .last()
        .and_then(|segment| segment.get("target_timerange"))
        .and_then(|range| Some(range.get("start")?.as_i64()? + range.get("duration")?.as_i64()?))
        .unwrap_or(draft_duration);

    while segments.len() < target_names.len() {
        segments.push(segment_template.clone());
    }
    segments.truncate(target_names.len());

    if target_names.len() > original_len && original_len > 0 {
        let split_start = segments[original_len - 1]
            .get("target_timerange")
            .and_then(|range| range.get("start"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let split_count = (target_names.len() - original_len + 1) as i64;
        let split_duration = ((original_end - split_start) / split_count).max(1);
        for index in (original_len - 1)..target_names.len() {
            segments[index]["target_timerange"]["start"] = Value::from(split_start + ((index - original_len + 1) as i64 * split_duration));
            let duration = if index == target_names.len() - 1 {
                (original_end - segments[index]["target_timerange"]["start"].as_i64().unwrap_or(split_start)).max(1)
            } else {
                split_duration
            };
            segments[index]["target_timerange"]["duration"] = Value::from(duration);
        }
    }

    for (index, segment) in segments.iter_mut().enumerate() {
        segment["id"] = Value::String(stable_jianying_segment_id(&target_names[index]));
        segment["material_id"] = Value::String(material_ids[index].clone());
        segment["visible"] = Value::Bool(true);
    }

    Ok(())
}

fn stable_jianying_image_id(name: &str) -> String {
    let suffix = name
        .trim_end_matches(".png")
        .replace("snapshot_", "SNAPSHOT_")
        .replace("cover", "COVER")
        .replace('-', "_");
    format!("VIDEO_CREATOR_IMAGE_{suffix}")
}

fn stable_jianying_segment_id(name: &str) -> String {
    let suffix = name
        .trim_end_matches(".png")
        .replace("snapshot_", "SNAPSHOT_")
        .replace("cover", "COVER")
        .replace('-', "_");
    format!("VIDEO_CREATOR_IMAGE_SEGMENT_{suffix}")
}

fn sync_draft_media_library(draft_dir: &Path, publish_dir: &Path) -> Result<(), String> {
    let meta_path = draft_dir.join("draft_meta_info.json");
    if !meta_path.exists() {
        return Ok(());
    }

    let mut meta = read_json_file(&meta_path)?;
    let materials = meta
        .get_mut("draft_materials")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "draft_meta_info.json 缺少 draft_materials".to_string())?;
    let photo_group = materials
        .iter_mut()
        .find(|group| group.get("type").and_then(Value::as_i64) == Some(0))
        .ok_or_else(|| "draft_meta_info.json 缺少本地媒体素材组".to_string())?;
    let values = photo_group
        .get_mut("value")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "draft_meta_info.json 本地媒体素材组缺少 value".to_string())?;

    let mut image_names = vec!["cover.png".to_string()];
    image_names.extend(list_snapshot_names(publish_dir)?);
    let existing = values.clone();
    let mut next_values = Vec::new();

    for item in existing.iter().filter(|item| item.get("metetype").and_then(Value::as_str) != Some("photo")) {
        next_values.push(item.clone());
    }

    let now = current_unix_seconds();
    for name in image_names {
        let source = publish_dir.join(&name);
        if !source.exists() {
            continue;
        }
        let existing_item = existing.iter().find(|item| item.get("extra_info").and_then(Value::as_str) == Some(name.as_str()));
        let id = stable_jianying_image_id(&name);
        let mut item = existing_item.cloned().unwrap_or_else(|| serde_json::json!({}));
        item["create_time"] = Value::from(now);
        item["duration"] = Value::from(5_000_000);
        item["extra_info"] = Value::String(name.clone());
        item["file_Path"] = Value::String(format!("D:/0000/video/0001_SixMinutes_Draft/{name}"));
        item["height"] = Value::from(1080);
        item["id"] = Value::String(id);
        item["import_time"] = Value::from(now);
        item["import_time_ms"] = Value::from(now * 1_000_000);
        item["item_source"] = Value::from(1);
        item["md5"] = Value::String(String::new());
        item["metetype"] = Value::String("photo".to_string());
        item["roughcut_time_range"] = serde_json::json!({ "duration": -1, "start": -1 });
        item["sub_time_range"] = serde_json::json!({ "duration": -1, "start": -1 });
        item["type"] = Value::from(0);
        item["width"] = Value::from(1920);
        next_values.push(item);
    }

    *values = next_values;
    write_json_file(&meta_path, &meta)
}

fn list_snapshot_names(publish_dir: &Path) -> Result<Vec<String>, String> {
    let mut snapshots = fs::read_dir(publish_dir)
        .map_err(|error| format!("读取发布素材目录失败：{error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("snapshot_") && name.ends_with(".png"))
                .unwrap_or(false)
        })
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()).map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    snapshots.sort();
    Ok(snapshots)
}

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn sync_text_track(track: &mut Value, cues: &[SrtCue], text_ids: &mut Vec<String>) -> Result<(), String> {
    let segments = track
        .get_mut("segments")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "字幕轨缺少 segments".to_string())?;
    if segments.len() < cues.len() {
        return Err(format!("字幕轨片段不足：草稿 {} 条，SRT {} 条", segments.len(), cues.len()));
    }
    segments.truncate(cues.len());
    for (segment, cue) in segments.iter_mut().zip(cues.iter()) {
        let material_id = segment
            .get("material_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "字幕片段缺少 material_id".to_string())?
            .to_string();
        segment["target_timerange"]["start"] = Value::from(cue.start_us);
        segment["target_timerange"]["duration"] = Value::from(cue.duration_us);
        text_ids.push(material_id);
    }
    Ok(())
}

fn update_text_material(material: &mut Value, text: &str) -> Result<(), String> {
    let content = material
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| "字幕素材缺少 content".to_string())?;
    let mut content_json: Value = serde_json::from_str(content).map_err(|error| format!("解析字幕素材 content 失败：{error}"))?;
    content_json["text"] = Value::String(text.to_string());
    if let Some(styles) = content_json.get_mut("styles").and_then(Value::as_array_mut) {
        for style in styles {
            style["range"] = Value::Array(vec![Value::from(0), Value::from(text.chars().count() as i64)]);
        }
    }
    material["content"] = Value::String(serde_json::to_string(&content_json).map_err(|error| error.to_string())?);
    Ok(())
}

fn parse_srt_file(path: &Path) -> Result<Vec<SrtCue>, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("读取字幕失败 {}：{error}", path.display()))?;
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let mut cues = Vec::new();
    for block in normalized.split("\n\n") {
        let mut lines = block.lines().filter(|line| !line.trim().is_empty());
        let first = match lines.next() {
            Some(value) => value.trim(),
            None => continue,
        };
        let timing = if first.contains("-->") {
            first
        } else {
            match lines.next() {
                Some(value) => value.trim(),
                None => continue,
            }
        };
        let Some((start, end)) = timing.split_once("-->") else {
            continue;
        };
        let start_us = parse_srt_time(start.trim())?;
        let end_us = parse_srt_time(end.trim())?;
        let text = lines.collect::<Vec<_>>().join("\n").trim().to_string();
        if !text.is_empty() && end_us > start_us {
            cues.push(SrtCue {
                start_us,
                duration_us: end_us - start_us,
                text,
            });
        }
    }
    Ok(cues)
}

fn parse_srt_time(value: &str) -> Result<i64, String> {
    let clean = value.split_whitespace().next().unwrap_or(value);
    let parts: Vec<&str> = clean.split([':', ',']).collect();
    if parts.len() != 4 {
        return Err(format!("无法解析字幕时间：{value}"));
    }
    let hours = parts[0].parse::<i64>().map_err(|error| error.to_string())?;
    let minutes = parts[1].parse::<i64>().map_err(|error| error.to_string())?;
    let seconds = parts[2].parse::<i64>().map_err(|error| error.to_string())?;
    let millis = parts[3].parse::<i64>().map_err(|error| error.to_string())?;
    Ok(((hours * 3600 + minutes * 60 + seconds) * 1000 + millis) * 1000)
}

fn repair_mojibake_srt(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("读取中文字幕失败：{error}"))?;
    if !looks_mojibake(&content) {
        return Ok(());
    }
    let mut repaired = String::with_capacity(content.len());
    for ch in content.chars() {
        if ch.is_ascii() {
            repaired.push(ch);
            continue;
        }
        let mut source = [0u8; 4];
        let encoded = ch.encode_utf8(&mut source);
        let (decoded, _, had_errors) = GBK.decode(encoded.as_bytes());
        if had_errors {
            repaired.push(ch);
        } else {
            repaired.push_str(&decoded);
        }
    }
    if looks_mojibake(&repaired) {
        return Err("中文字幕疑似乱码，自动修复后仍未通过检查".to_string());
    }
    fs::write(path, repaired).map_err(|error| format!("写回修复后的中文字幕失败：{error}"))
}

fn looks_mojibake(value: &str) -> bool {
    ["鎴", "锛", "銆", "�", "鐨", "涓"].iter().any(|token| value.contains(token))
}

fn read_json_file(path: &Path) -> Result<Value, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("读取 JSON 失败 {}：{error}", path.display()))?;
    serde_json::from_str(&content).map_err(|error| format!("解析 JSON 失败 {}：{error}", path.display()))
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    let backup = path.with_extension(format!(
        "{}.bak",
        path.extension().and_then(|extension| extension.to_str()).unwrap_or("json")
    ));
    if path.exists() {
        fs::copy(path, backup).map_err(|error| format!("备份 JSON 失败 {}：{error}", path.display()))?;
    }
    let content = serde_json::to_string(value).map_err(|error| format!("序列化 JSON 失败：{error}"))?;
    fs::write(path, content).map_err(|error| format!("写入 JSON 失败 {}：{error}", path.display()))
}

fn command_error(message: impl Into<String>) -> CommandError {
    CommandError {
        message: message.into(),
    }
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> CommandError {
    CommandError {
        message: error.to_string(),
    }
}
