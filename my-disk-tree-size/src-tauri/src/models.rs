use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub default_path: String,
    pub size_unit: SizeUnit,
    pub include_hidden: bool,
    pub max_depth: u32,
    pub video_root: String,
    pub excluded_dir_names: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            default_path: "Z:\\".to_string(),
            size_unit: SizeUnit::Auto,
            include_hidden: true,
            max_depth: 8,
            video_root: "Z:\\Video".to_string(),
            excluded_dir_names: default_excluded_dir_names(),
        }
    }
}

pub fn default_excluded_dir_names() -> Vec<String> {
    vec![
        "node_modules".to_string(),
        ".pnpm-store".to_string(),
        ".git".to_string(),
        ".cache".to_string(),
        "target".to_string(),
        "dist".to_string(),
        "mysql-test".to_string(),
        "__pycache__".to_string(),
        ".pytest_cache".to_string(),
        ".mypy_cache".to_string(),
        ".venv".to_string(),
        "venv".to_string(),
        "bower_components".to_string(),
        ".gradle".to_string(),
        ".idea".to_string(),
        ".vscode".to_string(),
        "@eaDir".to_string(),
        "#recycle".to_string(),
        "照片".to_string(),
        "BTPanel".to_string(),
        "btpanel_data".to_string(),
        "external_data_147".to_string(),
        "external_data_148".to_string(),
        "PicAcg".to_string(),
        "mt".to_string(),
        "Apple".to_string(),
        "docker_data".to_string(),
    ]
}

pub fn merge_default_excluded_dir_names(values: &mut Vec<String>) {
    for item in default_excluded_dir_names() {
        if !values.iter().any(|value| value.eq_ignore_ascii_case(&item)) {
            values.push(item);
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
pub enum SizeUnit {
    Auto,
    B,
    Kb,
    Mb,
    Gb,
    Tb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    pub path: String,
    pub max_depth: Option<u32>,
    pub include_hidden: Option<bool>,
    pub excluded_dir_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub run_id: i64,
    pub root: DiskNode,
    pub scanned_at: String,
    pub elapsed_ms: u128,
    pub volume_info: Option<VolumeInfo>,
    pub errors: Vec<ScanError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanTreeSaveRequest {
    pub root: DiskNode,
    pub errors: Vec<ScanError>,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanTreeSaveResult {
    pub run_id: i64,
    pub scanned_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskNode {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub allocated_size: u64,
    pub file_count: u64,
    pub folder_count: u64,
    pub percent: f64,
    pub depth: u32,
    pub is_dir: bool,
    pub modified_at: Option<String>,
    pub extension: Option<String>,
    pub truncated: bool,
    pub skipped: bool,
    pub skip_reason: Option<String>,
    pub children: Vec<DiskNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRunSummary {
    pub id: i64,
    pub root_path: String,
    pub scanned_at: String,
    pub elapsed_ms: i64,
    pub total_size: u64,
    pub file_count: u64,
    pub folder_count: u64,
    pub error_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub key: String,
    pub size: u64,
    pub name: String,
    pub count: u64,
    pub wasted_size: u64,
    pub files: Vec<DuplicateFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateSearchResult {
    pub run_id: Option<i64>,
    pub groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoClassificationRequest {
    pub root_path: String,
    pub target_root: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoClassificationSuggestion {
    pub id: i64,
    pub source_path: String,
    pub file_name: String,
    pub size: u64,
    pub category: String,
    pub subcategory: String,
    pub target_path: String,
    pub confidence: f64,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoClassificationResult {
    pub suggestions: Vec<VideoClassificationSuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveVideoRequest {
    pub suggestion_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveVideoResult {
    pub ok: bool,
    pub source_path: String,
    pub target_path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOperationLogsRequest {
    pub limit: usize,
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
