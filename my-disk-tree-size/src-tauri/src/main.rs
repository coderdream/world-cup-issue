use my_disk_tree_size_lib::commands::{save_scan_result, scan_path_with_logging, AppData};
use my_disk_tree_size_lib::models::ScanRequest;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|value| value == "--scan-cli").unwrap_or(false) {
        if let Err(error) = run_scan_cli(args.get(2).cloned().unwrap_or_else(|| "Z:\\".to_string())) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    my_disk_tree_size_lib::run();
}

fn run_scan_cli(path: String) -> Result<(), String> {
    let base_dir = std::env::var_os("MY_DISK_TREE_SIZE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
                .join("com.mydisktreesize.desktop")
        });
    let local_dir = std::env::var_os("MY_DISK_TREE_SIZE_LOCAL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| base_dir.clone())
                .join("com.mydisktreesize.desktop")
        });
    let data = AppData::from_paths(base_dir.join("settings.json"), base_dir.join("app.db"), local_dir.join("logs"));
    data.logger.info("scan", "cli.start", format!("命令行端到端扫描开始：{path}"));
    let started = std::time::Instant::now();
    let settings = data.settings.lock().map(|settings| settings.clone()).unwrap_or_default();
    let max_depth = settings.max_depth.clamp(1, 32);
    let request = ScanRequest {
        path: path.clone(),
        max_depth: Some(max_depth),
        include_hidden: Some(true),
        excluded_dir_names: Some(settings.excluded_dir_names.clone()),
    };
    data.logger.info(
        "scan",
        "cli.options",
        format!("命令行扫描参数：max_depth={max_depth}，排除目录={}", settings.excluded_dir_names.join(", ")),
    );
    let result = scan_path_with_logging(request, max_depth, true, data.logger.clone()).map_err(|error| error.message)?;
    let scanned_at = chrono::Utc::now().to_rfc3339();
    let run_id = save_scan_result(&data, &result.root, &result.errors, &scanned_at, started.elapsed().as_millis()).map_err(|error| error.message)?;
    data.logger.info(
        "scan",
        "cli.finish",
        format!(
            "命令行端到端扫描完成并写入 SQLite，批次 #{run_id}，大小 {} 字节，文件 {} 个，文件夹 {} 个，读取问题 {} 个。",
            result.root.size,
            result.root.file_count,
            result.root.folder_count,
            result.errors.len()
        ),
    );
    println!(
        "run_id={run_id}; root={}; size={}; files={}; folders={}; errors={}; db={}",
        result.root.path,
        result.root.size,
        result.root.file_count,
        result.root.folder_count,
        result.errors.len(),
        data.db_path.display()
    );
    Ok(())
}
