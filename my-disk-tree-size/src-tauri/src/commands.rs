use crate::models::{
    merge_default_excluded_dir_names, AppSettings, AppStatePayload, DiskNode, DuplicateFile, DuplicateGroup, DuplicateSearchResult,
    GetOperationLogsRequest, GetOperationLogsResult, MoveVideoRequest, MoveVideoResult, OperationLogEntry, ScanError, ScanRequest,
    ScanResult, ScanRunSummary, ScanTreeSaveRequest, ScanTreeSaveResult, UpdateInfo, VideoClassificationRequest, VideoClassificationResult,
    VideoClassificationSuggestion,
};
use crate::operation_log::OperationLogger;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use tauri::{Manager, State};

const VIDEO_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "mov", "wmv", "flv", "ts", "m2ts", "webm", "rmvb"];
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "heic", "avif"];
const SCAN_PROGRESS_INTERVAL: Duration = Duration::from_secs(5);
const MASS_IMAGE_DIR_THRESHOLD: usize = 500;
const SUBTREE_FILE_LIMIT: u64 = 20_000;
const SUBTREE_TIME_LIMIT: Duration = Duration::from_secs(120);

pub struct AppData {
    pub settings: Mutex<AppSettings>,
    settings_path: PathBuf,
    pub db_path: PathBuf,
    pub logger: OperationLogger,
}

impl AppData {
    pub fn load(app: &tauri::AppHandle) -> Self {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let settings_path = app_data_dir.join("settings.json");
        let db_path = app_data_dir.join("app.db");
        let log_dir = app.path().app_local_data_dir().unwrap_or_else(|_| app_data_dir.clone()).join("logs");
        let mut settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str::<AppSettings>(&content).ok())
            .unwrap_or_default();
        merge_default_excluded_dir_names(&mut settings.excluded_dir_names);

        let logger = OperationLogger::new(db_path.clone(), log_dir);
        let data = Self {
            settings: Mutex::new(settings),
            settings_path,
            db_path,
            logger,
        };
        let _ = data.init_database();
        data.logger.info("app", "startup", "MyDiskTreeSize 已启动。");
        data
    }

    pub fn from_paths(settings_path: PathBuf, db_path: PathBuf, log_dir: PathBuf) -> Self {
        let mut settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str::<AppSettings>(&content).ok())
            .unwrap_or_default();
        merge_default_excluded_dir_names(&mut settings.excluded_dir_names);
        let logger = OperationLogger::new(db_path.clone(), log_dir);
        let data = Self {
            settings: Mutex::new(settings),
            settings_path,
            db_path,
            logger,
        };
        let _ = data.init_database();
        data
    }

    pub fn connection(&self) -> Result<Connection, CommandError> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent).map_err(|error| command_error(format!("创建数据库目录失败：{error}")))?;
        }
        Connection::open(&self.db_path).map_err(|error| command_error(format!("打开 SQLite 数据库失败：{error}")))
    }

    pub fn init_database(&self) -> Result<(), CommandError> {
        let conn = self.connection()?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS scan_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                root_path TEXT NOT NULL,
                scanned_at TEXT NOT NULL,
                elapsed_ms INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                allocated_size INTEGER NOT NULL,
                file_count INTEGER NOT NULL,
                folder_count INTEGER NOT NULL,
                error_count INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS scan_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL,
                parent_path TEXT,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                extension TEXT,
                is_dir INTEGER NOT NULL,
                depth INTEGER NOT NULL,
                size INTEGER NOT NULL,
                allocated_size INTEGER NOT NULL,
                file_count INTEGER NOT NULL,
                folder_count INTEGER NOT NULL,
                percent REAL NOT NULL,
                modified_at TEXT,
                truncated INTEGER NOT NULL DEFAULT 0,
                skipped INTEGER NOT NULL DEFAULT 0,
                skip_reason TEXT,
                FOREIGN KEY(run_id) REFERENCES scan_runs(id)
            );
            CREATE INDEX IF NOT EXISTS idx_scan_entries_run ON scan_entries(run_id);
            CREATE INDEX IF NOT EXISTS idx_scan_entries_path ON scan_entries(path);
            CREATE INDEX IF NOT EXISTS idx_scan_entries_duplicate ON scan_entries(run_id, is_dir, size, name);
            CREATE TABLE IF NOT EXISTS scan_errors (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                message TEXT NOT NULL,
                FOREIGN KEY(run_id) REFERENCES scan_runs(id)
            );
            CREATE TABLE IF NOT EXISTS duplicate_groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER,
                duplicate_key TEXT NOT NULL,
                size INTEGER NOT NULL,
                name TEXT NOT NULL,
                count INTEGER NOT NULL,
                wasted_size INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS video_classification_suggestions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_path TEXT NOT NULL,
                file_name TEXT NOT NULL,
                size INTEGER NOT NULL,
                category TEXT NOT NULL,
                subcategory TEXT NOT NULL,
                target_path TEXT NOT NULL,
                confidence REAL NOT NULL,
                reason TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS operate_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                level TEXT NOT NULL,
                module TEXT NOT NULL,
                action TEXT NOT NULL,
                message TEXT NOT NULL,
                detail TEXT,
                trace_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_operate_log_created_at ON operate_log(created_at);
            CREATE INDEX IF NOT EXISTS idx_operate_log_module_action ON operate_log(module, action);
            ",
        )
        .map_err(|error| command_error(format!("初始化 SQLite 表失败：{error}")))?;
        let _ = conn.execute("ALTER TABLE scan_entries ADD COLUMN truncated INTEGER NOT NULL DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE scan_entries ADD COLUMN skipped INTEGER NOT NULL DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE scan_entries ADD COLUMN skip_reason TEXT", []);
        Ok(())
    }

    fn save_settings(&self, settings: &AppSettings) -> Result<(), CommandError> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| command_error(format!("创建配置目录失败：{error}")))?;
        }
        let content = serde_json::to_string_pretty(settings).map_err(|error| command_error(format!("序列化配置失败：{error}")))?;
        fs::write(&self.settings_path, content).map_err(|error| command_error(format!("保存配置失败：{error}")))?;
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
    data.init_database()?;
    Ok(AppStatePayload {
        settings: data.settings.lock().map_err(lock_error)?.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
pub fn get_settings(data: State<'_, AppData>) -> Result<AppSettings, CommandError> {
    Ok(data.settings.lock().map_err(lock_error)?.clone())
}

#[tauri::command]
pub fn set_settings(data: State<'_, AppData>, settings: AppSettings) -> Result<AppSettings, CommandError> {
    let mut current = data.settings.lock().map_err(lock_error)?;
    *current = settings.clone();
    data.save_settings(&settings)?;
    data.logger.info("settings", "save", "配置已保存。");
    Ok(settings)
}

#[tauri::command]
pub fn check_update_mock(data: State<'_, AppData>) -> Result<UpdateInfo, CommandError> {
    let _settings = data.settings.lock().map_err(lock_error)?;
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    Ok(UpdateInfo {
        current_version: current_version.clone(),
        latest_version: current_version,
        available: false,
        notes: "当前已经是最新版本。".to_string(),
    })
}

#[tauri::command]
pub async fn scan_disk_tree_shallow(data: State<'_, AppData>, request: ScanRequest) -> Result<ScanResult, CommandError> {
    data.logger.info("scan", "shallow.start", format!("开始读取第一层：{}", request.path));
    let path = request.path.clone();
    let logger = data.logger.clone();
    let result = tauri::async_runtime::spawn_blocking(move || scan_path(request, 1, false, Some(logger)))
        .await
        .map_err(|error| command_error(format!("浅层扫描任务执行失败：{error}")))?;
    match &result {
        Ok(scan) => data.logger.info(
            "scan",
            "shallow.finish",
            format!("第一层读取完成：{}，文件 {} 个，文件夹 {} 个。", path, scan.root.file_count, scan.root.folder_count),
        ),
        Err(error) => data.logger.error("scan", "shallow.error", format!("第一层读取失败：{path}"), &error.message),
    }
    result
}

#[tauri::command]
pub async fn scan_disk_subtree(data: State<'_, AppData>, request: ScanRequest) -> Result<ScanResult, CommandError> {
    let max_depth = request.max_depth.unwrap_or(8).clamp(1, 32);
    data.logger.debug("scan", "subtree.start", format!("开始扫描子目录：{}", request.path), format!("max_depth={max_depth}"));
    let path = request.path.clone();
    let logger = data.logger.clone();
    let result = tauri::async_runtime::spawn_blocking(move || scan_path(request, max_depth, false, Some(logger)))
        .await
        .map_err(|error| command_error(format!("子目录扫描任务执行失败：{error}")))?;
    match &result {
        Ok(scan) => data.logger.info(
            "scan",
            "subtree.finish",
            format!("子目录扫描完成：{}，大小 {} 字节，文件 {} 个，文件夹 {} 个。", path, scan.root.size, scan.root.file_count, scan.root.folder_count),
        ),
        Err(error) => data.logger.error("scan", "subtree.error", format!("子目录扫描失败：{path}"), &error.message),
    }
    result
}

#[tauri::command]
pub async fn scan_disk_tree(data: State<'_, AppData>, request: ScanRequest) -> Result<ScanResult, CommandError> {
    let max_depth = request.max_depth.unwrap_or_else(|| data.settings.lock().ok().map(|s| s.max_depth).unwrap_or(8)).clamp(1, 32);
    data.logger.info("scan", "full.start", format!("开始完整扫描：{}", request.path));
    let logger = data.logger.clone();
    let scan_result = tauri::async_runtime::spawn_blocking(move || scan_path(request, max_depth, true, Some(logger)))
        .await
        .map_err(|error| command_error(format!("完整扫描任务执行失败：{error}")))?;
    let mut result = scan_result?;
    let scanned_at = Utc::now().to_rfc3339();
    let run_id = save_scan_result(&data, &result.root, &result.errors, &scanned_at, result.elapsed_ms)?;
    result.run_id = run_id;
    result.scanned_at = scanned_at;
    data.logger.info("scan", "full.finish", format!("完整扫描完成并写入 SQLite，批次 #{run_id}。"));
    Ok(result)
}

#[tauri::command]
pub fn save_scan_tree(data: State<'_, AppData>, request: ScanTreeSaveRequest) -> Result<ScanTreeSaveResult, CommandError> {
    let scanned_at = Utc::now().to_rfc3339();
    let mut root = request.root;
    let root_size = root.size.max(1);
    assign_percentages(&mut root, root_size);
    let run_id = save_scan_result(&data, &root, &request.errors, &scanned_at, request.elapsed_ms)?;
    data.logger.info(
        "scan",
        "save",
        format!("分阶段扫描结果已写入 SQLite，批次 #{run_id}，根目录：{}。", root.path),
    );
    Ok(ScanTreeSaveResult { run_id, scanned_at })
}

#[tauri::command]
pub fn list_scan_runs(data: State<'_, AppData>) -> Result<Vec<ScanRunSummary>, CommandError> {
    let conn = data.connection()?;
    let mut stmt = conn
        .prepare(
            "
            SELECT id, root_path, scanned_at, elapsed_ms, total_size, file_count, folder_count, error_count
            FROM scan_runs
            ORDER BY id DESC
            LIMIT 30
            ",
        )
        .map_err(db_error)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ScanRunSummary {
                id: row.get(0)?,
                root_path: row.get(1)?,
                scanned_at: row.get(2)?,
                elapsed_ms: row.get(3)?,
                total_size: row.get::<_, i64>(4)? as u64,
                file_count: row.get::<_, i64>(5)? as u64,
                folder_count: row.get::<_, i64>(6)? as u64,
                error_count: row.get::<_, i64>(7)? as u64,
            })
        })
        .map_err(db_error)?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(db_error)?);
    }
    Ok(result)
}

#[tauri::command]
pub fn find_duplicate_files(data: State<'_, AppData>, run_id: Option<i64>) -> Result<DuplicateSearchResult, CommandError> {
    data.logger.info("duplicates", "search.start", "开始查找重复文件候选项。");
    let conn = data.connection()?;
    let actual_run_id = match run_id {
        Some(id) => Some(id),
        None => conn
            .query_row("SELECT id FROM scan_runs ORDER BY id DESC LIMIT 1", [], |row| row.get::<_, i64>(0))
            .optional()
            .map_err(db_error)?,
    };

    let Some(actual_run_id) = actual_run_id else {
        data.logger.warn("duplicates", "search.empty", "尚无扫描批次，无法查找重复文件。", "");
        return Ok(DuplicateSearchResult {
            run_id: None,
            groups: Vec::new(),
        });
    };

    let mut group_stmt = conn
        .prepare(
            "
            SELECT name, size, COUNT(*) AS count
            FROM scan_entries
            WHERE run_id = ?1 AND is_dir = 0 AND size > 0
            GROUP BY lower(name), size
            HAVING COUNT(*) > 1
            ORDER BY (size * (COUNT(*) - 1)) DESC
            LIMIT 200
            ",
        )
        .map_err(db_error)?;

    let mut file_stmt = conn
        .prepare(
            "
            SELECT path, name, size, modified_at
            FROM scan_entries
            WHERE run_id = ?1 AND is_dir = 0 AND lower(name) = lower(?2) AND size = ?3
            ORDER BY path
            ",
        )
        .map_err(db_error)?;

    let mapped = group_stmt
        .query_map(params![actual_run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? as u64,
                row.get::<_, i64>(2)? as u64,
            ))
        })
        .map_err(db_error)?;

    let mut groups = Vec::new();
    for item in mapped {
        let (name, size, count) = item.map_err(db_error)?;
        let mut files = Vec::new();
        let file_rows = file_stmt
            .query_map(params![actual_run_id, name, size as i64], |row| {
                Ok(DuplicateFile {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    size: row.get::<_, i64>(2)? as u64,
                    modified_at: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        for file in file_rows {
            files.push(file.map_err(db_error)?);
        }
        groups.push(DuplicateGroup {
            key: format!("{size}:{name}"),
            size,
            name,
            count,
            wasted_size: size.saturating_mul(count.saturating_sub(1)),
            files,
        });
    }

    data.logger.info("duplicates", "search.finish", format!("重复文件候选查找完成，批次 #{actual_run_id}，候选组 {} 组。", groups.len()));
    Ok(DuplicateSearchResult {
        run_id: Some(actual_run_id),
        groups,
    })
}

#[tauri::command]
pub fn classify_videos(data: State<'_, AppData>, request: VideoClassificationRequest) -> Result<VideoClassificationResult, CommandError> {
    data.logger.info("classification", "analyze.start", format!("开始分析视频目录：{}", request.root_path));
    let root = PathBuf::from(request.root_path.trim());
    if !root.exists() || !root.is_dir() {
        data.logger.warn("classification", "analyze.invalid_root", "视频扫描目录不存在。", request.root_path);
        return Err(command_error("请输入存在的视频扫描目录。"));
    }
    if request.target_root.trim().is_empty() {
        data.logger.warn("classification", "analyze.invalid_target", "分类目标根目录为空。", "");
        return Err(command_error("请输入分类目标根目录。"));
    }

    let target_root = PathBuf::from(request.target_root.trim());
    let mut files = Vec::new();
    collect_video_files(&root, request.limit.unwrap_or(300).clamp(1, 2000), &mut files)?;
    let conn = data.connection()?;
    let created_at = Utc::now().to_rfc3339();
    let mut suggestions = Vec::new();

    for path in files {
        let metadata = match fs::metadata(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let file_name = path.file_name().map(|value| value.to_string_lossy().to_string()).unwrap_or_default();
        let rule = classify_video_name(&file_name, &path);
        let target_path = target_root.join(&rule.category).join(&rule.subcategory).join(&file_name);
        conn.execute(
            "
            INSERT INTO video_classification_suggestions
                (source_path, file_name, size, category, subcategory, target_path, confidence, reason, status, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9)
            ",
            params![
                normalize_path(&path),
                file_name,
                metadata.len() as i64,
                rule.category,
                rule.subcategory,
                normalize_path(&target_path),
                rule.confidence,
                rule.reason,
                created_at,
            ],
        )
        .map_err(db_error)?;
        let id = conn.last_insert_rowid();
        suggestions.push(VideoClassificationSuggestion {
            id,
            source_path: normalize_path(&path),
            file_name,
            size: metadata.len(),
            category: rule.category,
            subcategory: rule.subcategory,
            target_path: normalize_path(&target_path),
            confidence: rule.confidence,
            reason: rule.reason,
            status: "pending".to_string(),
        });
    }

    data.logger.info("classification", "analyze.finish", format!("视频分类建议生成完成，共 {} 条。", suggestions.len()));
    Ok(VideoClassificationResult { suggestions })
}

#[tauri::command]
pub fn move_classified_video(data: State<'_, AppData>, request: MoveVideoRequest) -> Result<MoveVideoResult, CommandError> {
    data.logger.info("classification", "move.start", format!("准备移动分类建议 #{}。", request.suggestion_id));
    let conn = data.connection()?;
    let (source_path, target_path): (String, String) = conn
        .query_row(
            "SELECT source_path, target_path FROM video_classification_suggestions WHERE id = ?1",
            params![request.suggestion_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| command_error("没有找到对应的分类建议。"))?;

    let source = PathBuf::from(&source_path);
    let target = PathBuf::from(&target_path);
    if !source.exists() {
        data.logger.error("classification", "move.missing_source", "源文件不存在。", &source_path);
        return Err(command_error(format!("源文件不存在：{source_path}")));
    }
    if target.exists() {
        data.logger.warn("classification", "move.target_exists", "目标文件已存在。", &target_path);
        return Err(command_error(format!("目标文件已存在：{target_path}")));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| command_error(format!("创建目标目录失败：{error}")))?;
    }
    fs::rename(&source, &target).map_err(|error| command_error(format!("移动文件失败：{error}")))?;
    conn.execute(
        "UPDATE video_classification_suggestions SET status = 'moved' WHERE id = ?1",
        params![request.suggestion_id],
    )
    .map_err(db_error)?;

    data.logger.info("classification", "move.finish", format!("文件已移动：{} -> {}", source_path, target_path));
    Ok(MoveVideoResult {
        ok: true,
        source_path,
        target_path,
        message: "文件已移动。".to_string(),
    })
}

#[tauri::command]
pub fn get_operation_logs(data: State<'_, AppData>, request: GetOperationLogsRequest) -> Result<GetOperationLogsResult, CommandError> {
    let limit = request.limit.clamp(1, 1000);
    let conn = data.connection()?;
    let mut stmt = conn
        .prepare(
            "
            SELECT id, created_at, level, module, action, message, detail, trace_id
            FROM operate_log
            ORDER BY id DESC
            LIMIT ?1
            ",
        )
        .map_err(|error| command_error(format!("准备操作日志查询失败：{error}")))?;
    let rows = stmt
        .query_map([limit as i64], operation_log_from_row)
        .map_err(|error| command_error(format!("查询操作日志失败：{error}")))?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|error| command_error(format!("读取操作日志行失败：{error}")))?);
    }
    entries.reverse();
    Ok(GetOperationLogsResult { entries })
}

pub fn scan_path_with_logging(request: ScanRequest, max_depth: u32, save_ready: bool, logger: OperationLogger) -> Result<ScanResult, CommandError> {
    scan_path(request, max_depth, save_ready, Some(logger))
}

fn scan_path(request: ScanRequest, max_depth: u32, save_ready: bool, logger: Option<OperationLogger>) -> Result<ScanResult, CommandError> {
    let path_text = request.path.trim();
    if path_text.is_empty() {
        return Err(command_error("请输入要扫描的目录路径。"));
    }
    let path = PathBuf::from(path_text);
    if !path.exists() {
        return Err(command_error(format!("路径不存在：{path_text}")));
    }
    if !path.is_dir() {
        return Err(command_error(format!("请选择文件夹路径：{path_text}")));
    }

    let include_hidden = request.include_hidden.unwrap_or(true);
    let excluded_dir_names = normalize_excluded_dir_names(request.excluded_dir_names.unwrap_or_default());
    let started = Instant::now();
    let mut errors = Vec::new();
    let mut progress = ScanProgress::new(logger);
    let mut root = scan_dir(&path, 0, max_depth, include_hidden, &excluded_dir_names, &mut errors, &mut progress);
    progress.finish(&path);
    let root_size = root.size.max(1);
    assign_percentages(&mut root, root_size);
    let scanned_at = Utc::now().to_rfc3339();
    Ok(ScanResult {
        run_id: if save_ready { -1 } else { 0 },
        root,
        scanned_at,
        elapsed_ms: started.elapsed().as_millis(),
        volume_info: get_volume_info(&path),
        errors,
    })
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

pub fn save_scan_result(data: &AppData, root: &DiskNode, errors: &[ScanError], scanned_at: &str, elapsed_ms: u128) -> Result<i64, CommandError> {
    let mut conn = data.connection()?;
    let tx = conn.transaction().map_err(db_error)?;
    tx.execute(
        "
        INSERT INTO scan_runs(root_path, scanned_at, elapsed_ms, total_size, allocated_size, file_count, folder_count, error_count)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ",
        params![
            root.path,
            scanned_at,
            elapsed_ms as i64,
            root.size as i64,
            root.allocated_size as i64,
            root.file_count as i64,
            root.folder_count as i64,
            errors.len() as i64
        ],
    )
    .map_err(db_error)?;
    let run_id = tx.last_insert_rowid();
    insert_node(&tx, run_id, root, None)?;
    for error in errors {
        tx.execute(
            "INSERT INTO scan_errors(run_id, path, message) VALUES (?1, ?2, ?3)",
            params![run_id, error.path, error.message],
        )
        .map_err(db_error)?;
    }
    tx.commit().map_err(db_error)?;
    Ok(run_id)
}

fn insert_node(conn: &Connection, run_id: i64, node: &DiskNode, parent_path: Option<&str>) -> Result<(), CommandError> {
    conn.execute(
        "
        INSERT INTO scan_entries
            (run_id, parent_path, path, name, extension, is_dir, depth, size, allocated_size, file_count, folder_count, percent, modified_at, truncated, skipped, skip_reason)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ",
        params![
            run_id,
            parent_path,
            node.path,
            node.name,
            node.extension,
            if node.is_dir { 1 } else { 0 },
            node.depth as i64,
            node.size as i64,
            node.allocated_size as i64,
            node.file_count as i64,
            node.folder_count as i64,
            node.percent,
            node.modified_at,
            if node.truncated { 1 } else { 0 },
            if node.skipped { 1 } else { 0 },
            node.skip_reason,
        ],
    )
    .map_err(db_error)?;
    for child in &node.children {
        insert_node(conn, run_id, child, Some(&node.path))?;
    }
    Ok(())
}

struct ScanProgress {
    logger: Option<OperationLogger>,
    started: Instant,
    last_emit: Instant,
    visited_dirs: u64,
    visited_files: u64,
    visited_bytes: u64,
    skipped_dirs: u64,
}

impl ScanProgress {
    fn new(logger: Option<OperationLogger>) -> Self {
        let now = Instant::now();
        Self {
            logger,
            started: now,
            last_emit: now,
            visited_dirs: 0,
            visited_files: 0,
            visited_bytes: 0,
            skipped_dirs: 0,
        }
    }

    fn visit_dir(&mut self, path: &Path, depth: u32) {
        self.visited_dirs += 1;
        self.emit_if_due(path, depth, false);
    }

    fn visit_file(&mut self, path: &Path, depth: u32, size: u64) {
        self.visited_files += 1;
        self.visited_bytes = self.visited_bytes.saturating_add(size);
        self.emit_if_due(path, depth, false);
    }

    fn skip_dir(&mut self, path: &Path, reason: &str) {
        self.skipped_dirs += 1;
        if let Some(logger) = &self.logger {
            logger.scan_progress(
                "skip.dir",
                format!("跳过目录：{}", normalize_path(path)),
                format!("reason={reason}, skipped_dirs={}", self.skipped_dirs),
            );
        }
    }

    fn finish(&mut self, path: &Path) {
        self.emit(path, 0, true);
    }

    fn emit_if_due(&mut self, path: &Path, depth: u32, force: bool) {
        if force || self.last_emit.elapsed() >= SCAN_PROGRESS_INTERVAL {
            self.emit(path, depth, force);
        }
    }

    fn emit(&mut self, path: &Path, depth: u32, finished: bool) {
        let Some(logger) = &self.logger else {
            return;
        };
        self.last_emit = Instant::now();
        let action = if finished { "progress.finish" } else { "progress" };
        let message = if finished {
            format!("扫描阶段完成：{}", normalize_path(path))
        } else {
            format!("仍在扫描：{}", normalize_path(path))
        };
        let detail = format!(
            "depth={}, elapsed={}s, dirs={}, files={}, size={}, skipped_dirs={}",
            depth,
            self.started.elapsed().as_secs(),
            self.visited_dirs,
            self.visited_files,
            self.visited_bytes,
            self.skipped_dirs
        );
        logger.scan_progress(action, message, detail);
    }
}

fn scan_dir(
    path: &Path,
    depth: u32,
    max_depth: u32,
    include_hidden: bool,
    excluded_dir_names: &[String],
    errors: &mut Vec<ScanError>,
    progress: &mut ScanProgress,
) -> DiskNode {
    let subtree_started = Instant::now();
    progress.visit_dir(path, depth);
    let mut node = new_node(path, depth, true, 0);
    if depth >= max_depth {
        node.truncated = true;
        node.skip_reason = Some("达到扫描深度上限".to_string());
        progress.skip_dir(path, "达到扫描深度上限");
        return node;
    }
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(ScanError {
                path: normalize_path(path),
                message: error.to_string(),
            });
            return node;
        }
    };

    let entries: Vec<_> = entries.collect();
    if is_mass_image_dir(path, &entries) {
        node.skipped = true;
        node.skip_reason = Some(format!("疑似大量图片目录，直接图片文件超过 {MASS_IMAGE_DIR_THRESHOLD} 个"));
        progress.skip_dir(path, "疑似大量图片目录");
        return node;
    }

    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(ScanError {
                    path: normalize_path(path),
                    message: error.to_string(),
                });
                continue;
            }
        };
        let entry_path = entry.path();
        if !include_hidden && is_hidden(&entry_path) {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                errors.push(ScanError {
                    path: normalize_path(&entry_path),
                    message: error.to_string(),
                });
                continue;
            }
        };

        if metadata.is_dir() {
            if should_exclude_dir(&entry_path, excluded_dir_names) {
                progress.skip_dir(&entry_path, "目录名匹配排除规则");
                let mut child = new_node(&entry_path, depth + 1, true, 0);
                child.skipped = true;
                child.skip_reason = Some("目录名匹配排除规则".to_string());
                node.folder_count += 1;
                node.children.push(child);
                continue;
            }
            node.folder_count += 1;
            let child = scan_dir(&entry_path, depth + 1, max_depth, include_hidden, excluded_dir_names, errors, progress);
            node.size = node.size.saturating_add(child.size);
            node.allocated_size = node.allocated_size.saturating_add(child.allocated_size);
            node.file_count = node.file_count.saturating_add(child.file_count);
            node.folder_count = node.folder_count.saturating_add(child.folder_count);
            node.children.push(child);
            if mark_subtree_limit_if_needed(&mut node, progress) {
                break;
            }
            if mark_subtree_time_limit_if_needed(&mut node, progress, subtree_started) {
                break;
            }
        } else if metadata.is_file() {
            progress.visit_file(&entry_path, depth + 1, metadata.len());
            let mut child = new_node(&entry_path, depth + 1, false, metadata.len());
            child.allocated_size = allocated_size(metadata.len());
            child.file_count = 1;
            node.size = node.size.saturating_add(child.size);
            node.allocated_size = node.allocated_size.saturating_add(child.allocated_size);
            node.file_count += 1;
            node.children.push(child);
            if mark_subtree_limit_if_needed(&mut node, progress) {
                break;
            }
            if mark_subtree_time_limit_if_needed(&mut node, progress, subtree_started) {
                break;
            }
        }
    }

    node.children.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    node
}

fn normalize_excluded_dir_names(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().trim_matches('\\').trim_matches('/').to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn should_exclude_dir(path: &Path, excluded_dir_names: &[String]) -> bool {
    if excluded_dir_names.is_empty() {
        return false;
    }
    path.file_name()
        .map(|name| excluded_dir_names.contains(&name.to_string_lossy().to_lowercase()))
        .unwrap_or(false)
}

fn is_mass_image_dir(path: &Path, entries: &[Result<fs::DirEntry, std::io::Error>]) -> bool {
    if path.parent().is_none() {
        return false;
    }
    let mut image_count = 0usize;
    for entry in entries.iter().flatten() {
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }
        let Some(extension) = entry_path.extension().map(|value| value.to_string_lossy().to_lowercase()) else {
            continue;
        };
        if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
            image_count += 1;
            if image_count >= MASS_IMAGE_DIR_THRESHOLD {
                return true;
            }
        }
    }
    false
}

fn mark_subtree_limit_if_needed(node: &mut DiskNode, progress: &mut ScanProgress) -> bool {
    if node.depth == 0 {
        return false;
    }
    if node.file_count < SUBTREE_FILE_LIMIT || node.truncated {
        return false;
    }
    node.truncated = true;
    node.skip_reason = Some(format!("达到单目录扫描量上限，已扫描 {SUBTREE_FILE_LIMIT} 个文件"));
    progress.skip_dir(Path::new(&node.path), "达到单目录扫描量上限");
    true
}

fn mark_subtree_time_limit_if_needed(node: &mut DiskNode, progress: &mut ScanProgress, started: Instant) -> bool {
    if node.depth == 0 || node.truncated || started.elapsed() < SUBTREE_TIME_LIMIT {
        return false;
    }
    node.truncated = true;
    node.skip_reason = Some(format!("达到单目录扫描耗时上限，已扫描 {} 秒", SUBTREE_TIME_LIMIT.as_secs()));
    progress.skip_dir(Path::new(&node.path), "达到单目录扫描耗时上限");
    true
}

fn new_node(path: &Path, depth: u32, is_dir: bool, size: u64) -> DiskNode {
    let extension = if is_dir {
        None
    } else {
        path.extension().map(|value| value.to_string_lossy().to_lowercase())
    };
    DiskNode {
        id: normalize_path(path),
        name: display_name(path),
        path: normalize_path(path),
        size,
        allocated_size: if is_dir { 0 } else { allocated_size(size) },
        file_count: 0,
        folder_count: 0,
        percent: 0.0,
        depth,
        is_dir,
        modified_at: fs::metadata(path).ok().and_then(|metadata| metadata.modified().ok()).map(format_system_time),
        extension,
        truncated: false,
        skipped: false,
        skip_reason: None,
        children: Vec::new(),
    }
}

fn assign_percentages(node: &mut DiskNode, base_size: u64) {
    node.percent = if base_size == 0 {
        0.0
    } else {
        (node.size as f64 / base_size as f64 * 1000.0).round() / 10.0
    };
    let child_base = node.size.max(1);
    for child in &mut node.children {
        assign_percentages(child, child_base);
    }
}

struct ClassificationRule {
    category: String,
    subcategory: String,
    confidence: f64,
    reason: String,
}

fn classify_video_name(file_name: &str, path: &Path) -> ClassificationRule {
    let text = format!("{} {}", file_name.to_lowercase(), normalize_path(path).to_lowercase());
    let has_episode = text.contains("s01e") || text.contains("s02e") || text.contains("season") || (text.contains("第") && text.contains("集"));
    let has_course = contains_any(&text, &["course", "教程", "课程", "lesson", "python", "java", "blender", "photoshop", "premiere", "机器学习", "大模型"]);
    let has_documentary = contains_any(&text, &["documentary", "docu", "bbc", "nhk", "national.geographic", "纪录片", "自然", "历史", "宇宙"]);
    let has_variety = contains_any(&text, &["综艺", "脱口秀", "show", "variety"]);
    let has_anime = contains_any(&text, &["anime", "动画", "番剧", "japan", "01_japan"]);

    if has_course {
        return ClassificationRule {
            category: "学习视频".to_string(),
            subcategory: if contains_any(&text, &["ai", "大模型", "机器学习", "数据"]) { "AI与数据".to_string() } else { "其他课程".to_string() },
            confidence: 0.86,
            reason: "文件名或路径包含课程、教程、编程、AI 等学习视频关键词。".to_string(),
        };
    }
    if has_documentary {
        return ClassificationRule {
            category: "纪录片".to_string(),
            subcategory: if contains_any(&text, &["bbc", "nhk", "national"]) { "BBC-NHK-国家地理".to_string() } else { "未整理".to_string() },
            confidence: 0.82,
            reason: "文件名或路径包含 documentary、BBC、NHK、纪录片等关键词。".to_string(),
        };
    }
    if has_variety {
        return ClassificationRule {
            category: "综艺".to_string(),
            subcategory: "未整理".to_string(),
            confidence: 0.78,
            reason: "文件名或路径包含综艺、脱口秀或 show 等关键词。".to_string(),
        };
    }
    if has_anime {
        return ClassificationRule {
            category: "动画".to_string(),
            subcategory: "日本番剧".to_string(),
            confidence: 0.76,
            reason: "文件名或路径包含动画、番剧或 Japan 等关键词。".to_string(),
        };
    }
    if has_episode {
        return ClassificationRule {
            category: "电视剧".to_string(),
            subcategory: if contains_any(&text, &["korea", "韩剧"]) { "韩剧".to_string() } else if contains_any(&text, &["japan", "日剧"]) { "日剧".to_string() } else { "其他地区".to_string() },
            confidence: 0.72,
            reason: "文件名呈现剧集或季集编号特征。".to_string(),
        };
    }
    ClassificationRule {
        category: "电影".to_string(),
        subcategory: if contains_any(&text, &["china", "chinese", "国语", "粤语", "香港"]) {
            "华语".to_string()
        } else if contains_any(&text, &["japan", "korea", "india", "thai"]) {
            "亚洲".to_string()
        } else {
            "未整理".to_string()
        },
        confidence: 0.62,
        reason: "未命中特定剧集、课程、纪录片或综艺特征，先按电影待整理处理。".to_string(),
    }
}

fn collect_video_files(path: &Path, limit: u32, files: &mut Vec<PathBuf>) -> Result<(), CommandError> {
    if files.len() >= limit as usize {
        return Ok(());
    }
    let entries = fs::read_dir(path).map_err(|error| command_error(format!("读取目录失败：{error}")))?;
    for entry in entries {
        if files.len() >= limit as usize {
            break;
        }
        let entry = entry.map_err(|error| command_error(format!("读取目录项失败：{error}")))?;
        let entry_path = entry.path();
        let metadata = entry.metadata().map_err(|error| command_error(format!("读取文件信息失败：{error}")))?;
        if metadata.is_dir() {
            collect_video_files(&entry_path, limit, files)?;
        } else if metadata.is_file() && is_video_file(&entry_path) {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn is_video_file(path: &Path) -> bool {
    path.extension()
        .map(|value| VIDEO_EXTENSIONS.contains(&value.to_string_lossy().to_lowercase().as_str()))
        .unwrap_or(false)
}

fn contains_any(text: &str, values: &[&str]) -> bool {
    values.iter().any(|value| text.contains(&value.to_lowercase()))
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| normalize_path(path))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

fn allocated_size(size: u64) -> u64 {
    const CLUSTER_SIZE: u64 = 4096;
    if size == 0 {
        0
    } else {
        ((size + CLUSTER_SIZE - 1) / CLUSTER_SIZE) * CLUSTER_SIZE
    }
}

fn format_system_time(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339()
}

#[cfg(windows)]
fn get_volume_info(path: &Path) -> Option<crate::models::VolumeInfo> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let mut target = path.to_path_buf();
    while !target.exists() {
        target.pop();
    }
    let mut wide: Vec<u16> = target.as_os_str().encode_wide().collect();
    if !wide.ends_with(&[b'\\' as u16]) {
        wide.push(b'\\' as u16);
    }
    wide.push(0);

    let mut available = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut total, &mut free) };
    if ok == 0 {
        None
    } else {
        Some(crate::models::VolumeInfo {
            total_bytes: total,
            free_bytes: free,
            available_bytes: available,
        })
    }
}

#[cfg(not(windows))]
fn get_volume_info(_path: &Path) -> Option<crate::models::VolumeInfo> {
    None
}

#[cfg(windows)]
fn is_hidden(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    fs::metadata(path)
        .map(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .map(|name| name.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
}

fn command_error(message: impl Into<String>) -> CommandError {
    CommandError {
        message: message.into(),
    }
}

fn db_error(error: rusqlite::Error) -> CommandError {
    command_error(format!("SQLite 操作失败：{error}"))
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> CommandError {
    CommandError {
        message: error.to_string(),
    }
}
