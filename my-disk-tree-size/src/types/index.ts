export type SizeUnit = "auto" | "b" | "kb" | "mb" | "gb" | "tb";
export type RouteKey = "scan" | "duplicates" | "classification" | "logs";

export interface AppSettings {
  theme: "dark";
  defaultPath: string;
  sizeUnit: SizeUnit;
  includeHidden: boolean;
  maxDepth: number;
  videoRoot: string;
  excludedDirNames: string[];
}

export interface AppStatePayload {
  settings: AppSettings;
  version: string;
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  available: boolean;
  notes: string;
}

export interface ScanRequest {
  path: string;
  maxDepth?: number;
  includeHidden?: boolean;
  excludedDirNames?: string[];
}

export interface ScanResult {
  runId: number;
  root: DiskNode;
  scannedAt: string;
  elapsedMs: number;
  volumeInfo?: VolumeInfo;
  errors: ScanError[];
}

export interface VolumeInfo {
  totalBytes: number;
  freeBytes: number;
  availableBytes: number;
}

export interface ScanTreeSaveRequest {
  root: DiskNode;
  errors: ScanError[];
  elapsedMs: number;
}

export interface ScanTreeSaveResult {
  runId: number;
  scannedAt: string;
}

export interface DiskNode {
  id: string;
  name: string;
  path: string;
  size: number;
  allocatedSize: number;
  fileCount: number;
  folderCount: number;
  percent: number;
  depth: number;
  isDir: boolean;
  modifiedAt?: string;
  extension?: string;
  truncated?: boolean;
  skipped?: boolean;
  skipReason?: string;
  children: DiskNode[];
}

export interface ScanError {
  path: string;
  message: string;
}

export interface ScanRunSummary {
  id: number;
  rootPath: string;
  scannedAt: string;
  elapsedMs: number;
  totalSize: number;
  fileCount: number;
  folderCount: number;
  errorCount: number;
}

export interface DuplicateFile {
  path: string;
  name: string;
  size: number;
  modifiedAt?: string;
}

export interface DuplicateGroup {
  key: string;
  size: number;
  name: string;
  count: number;
  wastedSize: number;
  files: DuplicateFile[];
}

export interface DuplicateSearchResult {
  runId?: number;
  groups: DuplicateGroup[];
}

export interface VideoClassificationRequest {
  rootPath: string;
  targetRoot: string;
  limit?: number;
}

export interface VideoClassificationSuggestion {
  id: number;
  sourcePath: string;
  fileName: string;
  size: number;
  category: string;
  subcategory: string;
  targetPath: string;
  confidence: number;
  reason: string;
  status: string;
}

export interface VideoClassificationResult {
  suggestions: VideoClassificationSuggestion[];
}

export interface MoveVideoResult {
  ok: boolean;
  sourcePath: string;
  targetPath: string;
  message: string;
}

export interface GetOperationLogsRequest {
  limit: number;
}

export interface OperationLogEntry {
  id: number;
  createdAt: string;
  level: string;
  module: string;
  action: string;
  message: string;
  detail?: string | null;
  traceId?: string | null;
}

export interface GetOperationLogsResult {
  entries: OperationLogEntry[];
}
