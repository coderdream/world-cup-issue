import { defaultSettings } from "@/store/defaults";
import type {
  AppSettings,
  AppStatePayload,
  DuplicateSearchResult,
  GetOperationLogsRequest,
  GetOperationLogsResult,
  MoveVideoResult,
  ScanRequest,
  ScanResult,
  ScanRunSummary,
  ScanTreeSaveRequest,
  ScanTreeSaveResult,
  UpdateInfo,
  VideoClassificationRequest,
  VideoClassificationResult
} from "@/types";

async function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<T>(command, args);
  } catch {
    return browserFallback<T>(command, args);
  }
}

function browserFallback<T>(command: string, args?: Record<string, unknown>): T {
  const settings = loadSettings();
  if (command === "get_app_state") {
    return { settings, version: "0.1.20" } as T;
  }
  if (command === "get_settings") {
    return settings as T;
  }
  if (command === "set_settings") {
    const next = args?.settings as AppSettings;
    localStorage.setItem("my-disk-tree-size.settings", JSON.stringify(next));
    return next as T;
  }
  if (command === "check_update_mock") {
    return {
      currentVersion: "0.1.20",
      latestVersion: "0.1.20",
      available: false,
      notes: "当前已经是最新版本。"
    } as T;
  }
  if (command === "scan_disk_tree_shallow" || command === "scan_disk_subtree" || command === "scan_disk_tree") {
    const request = args?.request as ScanRequest;
    return makePreviewScan(request?.path ?? settings.defaultPath) as T;
  }
  if (command === "save_scan_tree") {
    return { runId: 1, scannedAt: new Date().toISOString() } as T;
  }
  if (command === "list_scan_runs") {
    return [
      {
        id: 1,
        rootPath: settings.defaultPath,
        scannedAt: new Date().toISOString(),
        elapsedMs: 1280,
        totalSize: 20_042_956_900_000,
        fileCount: 439_913,
        folderCount: 68_151,
        errorCount: 0
      }
    ] as T;
  }
  if (command === "find_duplicate_files") {
    return makePreviewDuplicates() as T;
  }
  if (command === "classify_videos") {
    const request = args?.request as VideoClassificationRequest;
    return makePreviewClassification(request?.targetRoot ?? settings.videoRoot) as T;
  }
  if (command === "move_classified_video") {
    return {
      ok: true,
      sourcePath: "preview-source.mkv",
      targetPath: "preview-target.mkv",
      message: "预览环境不会移动文件。"
    } as T;
  }
  if (command === "get_operation_logs") {
    const request = args?.request as GetOperationLogsRequest | undefined;
    return makePreviewLogs(request?.limit ?? 200) as T;
  }
  throw new Error(`Unsupported browser fallback command: ${command}`);
}

function loadSettings(): AppSettings {
  const raw = localStorage.getItem("my-disk-tree-size.settings");
  if (!raw) return defaultSettings;
  try {
    return { ...defaultSettings, ...JSON.parse(raw) };
  } catch {
    return defaultSettings;
  }
}

function makePreviewScan(path: string): ScanResult {
  const rootSize = 20_042_956_900_000;
  const children = [
    node("qb", `${path}\\qb`, 15_292_769_400_000, 90_779, 4_797, 76.2, 1),
    node("Disk_D", `${path}\\Disk_D`, 2_487_127_700_000, 49_500, 10_676, 12.4, 1, [
      node("Video", `${path}\\Disk_D\\Video`, 1_581_329_600_000, 11_891, 557, 63.6, 2),
      node("mt", `${path}\\Disk_D\\mt`, 452_545_600_000, 3_747, 402, 18.2, 2)
    ]),
    node("node_modules", `${path}\\Projects\\node_modules`, 0, 0, 0, 0, 2, [], true, "目录名匹配排除规则"),
    node("Archive", `${path}\\Archive`, 677_589_600_000, 43, 0, 3.4, 1),
    node("Projects", `${path}\\Projects`, 54_933_100_000, 146, 4, 0.3, 1)
  ];

  return {
    runId: 0,
    root: {
      id: path,
      name: path,
      path,
      size: rootSize,
      allocatedSize: rootSize,
      fileCount: 439_913,
      folderCount: 68_151,
      percent: 100,
      depth: 0,
      isDir: true,
      truncated: false,
      skipped: false,
      children
    },
    scannedAt: new Date().toISOString(),
    elapsedMs: 1280,
    volumeInfo: {
      totalBytes: 22_973_110_886_400,
      freeBytes: 1_121_181_188_096,
      availableBytes: 1_121_181_188_096
    },
    errors: []
  };
}

function node(
  name: string,
  path: string,
  size: number,
  fileCount: number,
  folderCount: number,
  percent: number,
  depth: number,
  children: any[] = [],
  skipped = false,
  skipReason?: string
) {
  return {
    id: path,
    name,
    path,
    size,
    allocatedSize: size,
    fileCount,
    folderCount,
    percent,
    depth,
    isDir: true,
    truncated: false,
    skipped,
    skipReason,
    children
  };
}

function makePreviewDuplicates(): DuplicateSearchResult {
  return {
    runId: 1,
    groups: [
      {
        key: "11881700000:Foods.That.Cure.Disease.2018.mkv",
        size: 11_881_700_000,
        name: "Foods.That.Cure.Disease.2018.mkv",
        count: 2,
        wastedSize: 11_881_700_000,
        files: [
          { path: "Z:\\Video\\纪录片\\Foods.That.Cure.Disease.2018.mkv", name: "Foods.That.Cure.Disease.2018.mkv", size: 11_881_700_000 },
          { path: "Z:\\downloads\\Foods.That.Cure.Disease.2018.mkv", name: "Foods.That.Cure.Disease.2018.mkv", size: 11_881_700_000 }
        ]
      }
    ]
  };
}

function makePreviewClassification(targetRoot: string): VideoClassificationResult {
  return {
    suggestions: [
      {
        id: 1,
        sourcePath: "Z:\\downloads\\Planet.Earth.II.S01E01.mkv",
        fileName: "Planet.Earth.II.S01E01.mkv",
        size: 8_600_000_000,
        category: "纪录片",
        subcategory: "未整理",
        targetPath: `${targetRoot}\\纪录片\\未整理\\Planet.Earth.II.S01E01.mkv`,
        confidence: 0.82,
        reason: "文件名包含自然纪录片特征，建议进入纪录片库。",
        status: "pending"
      }
    ]
  };
}

function makePreviewLogs(limit: number): GetOperationLogsResult {
  const now = new Date().toLocaleString("zh-CN", { hour12: false });
  const entries = [
    {
      id: 1,
      createdAt: now,
      level: "INFO",
      module: "scan",
      action: "shallow.start",
      message: "开始读取第一层：Z:\\",
      detail: null,
      traceId: null
    },
    {
      id: 2,
      createdAt: now,
      level: "INFO",
      module: "scan",
      action: "skip.dir",
      message: "跳过目录：Z:\\Projects\\node_modules",
      detail: "reason=目录名匹配排除规则, skipped_dirs=1",
      traceId: null
    },
    {
      id: 3,
      createdAt: now,
      level: "INFO",
      module: "scan",
      action: "save",
      message: "分阶段扫描结果已写入 SQLite，批次 #1，根目录：Z:\\。",
      detail: null,
      traceId: null
    }
  ];
  return { entries: entries.slice(-limit) };
}

export const diskApi = {
  getAppState: () => invokeTauri<AppStatePayload>("get_app_state"),
  getSettings: () => invokeTauri<AppSettings>("get_settings"),
  setSettings: (settings: AppSettings) => invokeTauri<AppSettings>("set_settings", { settings }),
  checkUpdate: () => invokeTauri<UpdateInfo>("check_update_mock"),
  scanDiskTree: (request: ScanRequest) => invokeTauri<ScanResult>("scan_disk_tree", { request }),
  scanDiskTreeShallow: (request: ScanRequest) => invokeTauri<ScanResult>("scan_disk_tree_shallow", { request }),
  scanDiskSubtree: (request: ScanRequest) => invokeTauri<ScanResult>("scan_disk_subtree", { request }),
  saveScanTree: (request: ScanTreeSaveRequest) => invokeTauri<ScanTreeSaveResult>("save_scan_tree", { request }),
  listScanRuns: () => invokeTauri<ScanRunSummary[]>("list_scan_runs"),
  findDuplicateFiles: (runId?: number) => invokeTauri<DuplicateSearchResult>("find_duplicate_files", { runId }),
  classifyVideos: (request: VideoClassificationRequest) => invokeTauri<VideoClassificationResult>("classify_videos", { request }),
  moveClassifiedVideo: (suggestionId: number) => invokeTauri<MoveVideoResult>("move_classified_video", { request: { suggestionId } }),
  getOperationLogs: (request: GetOperationLogsRequest) => invokeTauri<GetOperationLogsResult>("get_operation_logs", { request })
};
