use crate::models::{
    AiGenerateRequest, AiGenerateResult, AiTestResult, AppSettings, AppStatePayload, ChatCompletionRequest, ChatCompletionResponse,
    ChatMessage, FeishuSendRequest, FeishuSendResult, FeishuWebhookResponse, GetOperationLogsRequest, GetOperationLogsResult,
    OperationEventEntry, OperationHistoryEntry, OperationLogEntry, OperationStepEntry, QuarkStatus, RunWorkflowRequest,
    RunWorkflowResult, SkillConfigEntry, UpdateInfo, VideoCreatorDashboard,
};
use crate::operation_log::OperationLogger;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use tauri::{Manager, State};

const LEGACY_DB: &str = r"D:\04_GitHub\video-easy-creator\data\video-easy-creator.db";
const LEGACY_RUNTIME_LOG: &str = r"D:\04_GitHub\video-easy-creator\logs\app\runtime.log";

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
    let project_dir = PathBuf::from(&settings.java_project_dir);
    if !project_dir.exists() {
        return Err(command_error(format!("Legacy Java project directory does not exist: {}", project_dir.display())));
    }

    let args = build_legacy_args(settings, &request);
    let classpath = legacy_classpath(&project_dir);
    let logger = logger.clone();
    let command_name = command.to_string();
    let args_text = args.join(" ");
    logger.info("video", "run_workflow", &format!("Background task submitted: {args_text}"));

    thread::Builder::new()
        .name(format!("video-workflow-{command_name}"))
        .spawn(move || {
            logger.info("video", "run_workflow", &format!("Background task started: {args_text}"));
            let output = Command::new("java")
                .current_dir(&project_dir)
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
            started_at: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
            finished_at: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
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
        SELECT step_order, step_code, step_name, status, started_at, finished_at, duration_ms, description
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
            started_at: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            finished_at: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
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
            created_at: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
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
    let cookie_file = Path::new(&settings.java_project_dir).join("auth").join("cookie").join("quark").join("cookies.txt");
    let updated = fs::metadata(&cookie_file)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| format!("Unix {}", duration.as_secs()))
        .unwrap_or_else(|| "-".to_string());
    let logs = tail_lines(Path::new(LEGACY_RUNTIME_LOG), 80)
        .into_iter()
        .filter(|line| line.to_lowercase().contains("quark"))
        .collect::<Vec<_>>();
    QuarkStatus {
        token_valid: if cookie_file.exists() { "pending check".to_string() } else { "no".to_string() },
        cookie_file: cookie_file.display().to_string(),
        cookie_updated_at: updated,
        root_item_count: 0,
        latest_result: "Startup does not auto-resume tasks. Trigger Quark actions manually.".to_string(),
        logs,
    }
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
    let base = Path::new(&settings.java_project_dir);
    match target {
        "output" => PathBuf::from(&settings.output_dir),
        "ppt_config" => base.join("PPT_TEMPLATE.md"),
        "quark_cookie" => base.join("auth").join("cookie").join("quark"),
        "quark_sync" => base.join("data"),
        "legacy_project" => base.to_path_buf(),
        _ => PathBuf::from(target),
    }
}

fn default_skills() -> Vec<SkillConfigEntry> {
    vec![
        skill("one-click", "One click", "one-click", true, 10, "Write todo, download resources, generate script, and continue workflow"),
        skill("bbc-prefetch", "BBC prefetch", "bbc-prefetch", true, 20, "Download images, audio, and PDF"),
        skill("script-text", "Generate Script", "script-text", true, 30, "Generate script text from PDF"),
        skill("question-title", "Question title", "question-title", true, 35, "Generate short Chinese question title"),
        skill("six-minutes-codex", "Codex workflow", "six-minutes-codex", true, 40, "Run BBC six-minute creation workflow"),
        skill("prepare-sixminutes", "Publish assets", "prepare-sixminutes", true, 60, "Prepare publish assets"),
        skill("daily-sync", "Daily sync", "daily-sync", true, 70, "Sync Daily files by year"),
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
        summary: "Waiting for manual task execution".to_string(),
        started_at: "-".to_string(),
        finished_at: "-".to_string(),
        duration_ms: 0,
    }]
}

fn sample_steps() -> Vec<OperationStepEntry> {
    let names = [
        ("TODO", "Write todo"),
        ("DOWNLOAD", "Download BBC resources"),
        ("SCRIPT", "Generate Script"),
        ("TITLE", "Generate question title"),
        ("STEP01", "Preprocess raw script"),
        ("STEP02", "Generate dialog script"),
        ("STEP03", "Generate question script"),
        ("STEP04", "Translate dialog script"),
        ("STEP05", "Translate question script"),
        ("STEP06", "Generate optimized dialog script"),
        ("STEP07", "Merge bilingual script"),
        ("STEP08", "Generate raw SRT subtitles"),
        ("STEP09", "Extract timestamps and split audio"),
        ("STEP10", "Generate final SRT script"),
        ("STEP11", "Generate final English subtitles"),
        ("STEP12", "Translate Chinese subtitles"),
        ("STEP13", "Merge bilingual subtitles"),
        ("STEP14", "Generate vocabulary list"),
        ("STEP15", "Check generated PPT"),
        ("STEP16", "Check generated PPT screenshots"),
        ("STEP17", "Generate description file"),
        ("STEP18", "Prepare publish assets"),
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
            description: (*name).to_string(),
        })
        .collect()
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
