import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import clsx from "clsx";
import {
  BarChart3,
  Bot,
  ChevronDown,
  ChevronRight,
  Copy,
  Database,
  Files,
  Folder,
  FolderOpen,
  HardDrive,
  Maximize2,
  Minimize2,
  RefreshCw,
  Search,
  Settings,
  Square,
  SquareTerminal,
  Trash2,
  WrapText,
  X
} from "lucide-react";
import { diskApi } from "@/services/diskApi";
import { useAppStore } from "@/store/useAppStore";
import type { DiskNode, DuplicateGroup, OperationLogEntry, RouteKey, ScanError, ScanResult, SizeUnit, VideoClassificationSuggestion } from "@/types";

const MAX_VISIBLE_TREE_ROWS = 1200;

const unitOptions: Array<{ value: SizeUnit; label: string }> = [
  { value: "auto", label: "自动" },
  { value: "mb", label: "MB" },
  { value: "gb", label: "GB" },
  { value: "tb", label: "TB" }
];

const navItems: Array<{ key: RouteKey; label: string; icon: ReactNode }> = [
  { key: "scan", label: "空间扫描", icon: <BarChart3 size={18} /> },
  { key: "duplicates", label: "重复文件", icon: <Copy size={18} /> },
  { key: "classification", label: "AI 分类", icon: <Bot size={18} /> },
  { key: "logs", label: "操作日志", icon: <SquareTerminal size={18} /> }
];

export default function App() {
  const hydrate = useAppStore((state) => state.hydrate);
  const route = useAppStore((state) => state.route);
  const setRoute = useAppStore((state) => state.setRoute);
  const settings = useAppStore((state) => state.settings);
  const version = useAppStore((state) => state.version);
  const updateSettings = useAppStore((state) => state.updateSettings);
  const [path, setPath] = useState(settings.defaultPath);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [isScanning, setIsScanning] = useState(false);
  const [status, setStatus] = useState("准备扫描。");
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  useEffect(() => {
    setPath(settings.defaultPath);
  }, [settings.defaultPath]);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    let canceled = false;

    async function bindWindowState() {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const appWindow = getCurrentWindow();
        if (!canceled) setIsMaximized(await appWindow.isMaximized());
        dispose = await appWindow.onResized(async () => setIsMaximized(await appWindow.isMaximized()));
      } catch {
        // Browser preview has no native window API.
      }
    }

    void bindWindowState();
    return () => {
      canceled = true;
      dispose?.();
    };
  }, []);

  async function scan(nextPath = path) {
    const cleanPath = nextPath.trim();
    if (!cleanPath) {
      setStatus("请输入要扫描的路径。");
      return;
    }

    const startedAt = performance.now();
    setIsScanning(true);
    setStatus(`正在读取第一层：${cleanPath}`);

    try {
      const shallow = await diskApi.scanDiskTreeShallow({
        path: cleanPath,
        maxDepth: 1,
        includeHidden: settings.includeHidden,
        excludedDirNames: settings.excludedDirNames
      });
      let root = recalculateTree(shallow.root);
      let errors: ScanError[] = [...shallow.errors];
      const firstLevelDirs = root.children.filter((child) => child.isDir);
      const baseResult = { ...shallow, root, runId: 0 };

      setResult(baseResult);
      setExpanded(new Set([root.id]));
      await updateSettings({ defaultPath: cleanPath });

      for (let index = 0; index < firstLevelDirs.length; index += 1) {
        const dir = firstLevelDirs[index];
        setStatus(`正在扫描 ${index + 1}/${firstLevelDirs.length}：${dir.path}`);
        const subtree = await diskApi.scanDiskSubtree({
          path: dir.path,
          maxDepth: Math.max(1, settings.maxDepth - 1),
          includeHidden: settings.includeHidden,
          excludedDirNames: settings.excludedDirNames
        });
        errors = errors.concat(subtree.errors);
        root = replaceNode(root, dir.path, subtree.root);
        root = recalculateTree(root);
        setResult({
          runId: 0,
          root,
          scannedAt: new Date().toISOString(),
          elapsedMs: Math.round(performance.now() - startedAt),
          volumeInfo: shallow.volumeInfo,
          errors
        });
        setExpanded((current) => new Set([...current, root.id, dir.path]));
        await yieldToUi();
      }

      setStatus("扫描完成，正在写入 SQLite。");
      root = recalculateTree(root);
      const saved = await diskApi.saveScanTree({
        root,
        errors,
        elapsedMs: Math.round(performance.now() - startedAt)
      });
      setResult({
        runId: saved.runId,
        root,
        scannedAt: saved.scannedAt,
        elapsedMs: Math.round(performance.now() - startedAt),
        volumeInfo: shallow.volumeInfo,
        errors
      });
      setStatus(`扫描完成并写入 SQLite，批次 #${saved.runId}，读取问题 ${errors.length} 个。`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setIsScanning(false);
    }
  }

  const sqliteStatus = result?.runId ? `扫描批次 #${result.runId} 已保存到 SQLite` : "扫描完成后保存到 SQLite";

  return (
    <div className="app-shell">
      <header className="titlebar" onMouseDown={(event) => void startWindowDrag(event)}>
        <div className="brand">
          <HardDrive size={18} />
          <span>MyDiskTreeSize</span>
          <small>v{version}</small>
        </div>
        <div className="window-actions">
          <button type="button" title="最小化" onClick={() => void minimizeWindow()}>
            <Minimize2 size={14} />
          </button>
          <button type="button" title={isMaximized ? "还原" : "最大化"} onClick={() => void toggleMaximizeWindow()}>
            {isMaximized ? <Square size={13} /> : <Maximize2 size={14} />}
          </button>
          <button className="danger" type="button" title="关闭" onClick={() => void closeWindow()}>
            <X size={14} />
          </button>
        </div>
      </header>

      <div className="body-shell">
        <aside className="sidebar">
          <div className="sidebar-section">功能</div>
          {navItems.map((item) => (
            <button className={clsx("nav-item", route === item.key && "active")} key={item.key} type="button" onClick={() => setRoute(item.key)}>
              {item.icon}
              <span>{item.label}</span>
            </button>
          ))}
          <div className={clsx("sidebar-footer", result?.runId && "saved")}>
            <Database size={15} />
            <span>{sqliteStatus}</span>
          </div>
        </aside>

        <main className="main-area">
          {route === "scan" && (
            <ScanPage
              path={path}
              setPath={setPath}
              result={result}
              expanded={expanded}
              setExpanded={setExpanded}
              settings={settings}
              updateSettings={updateSettings}
              isScanning={isScanning}
              scan={scan}
            />
          )}
          {route === "duplicates" && <DuplicatesPage />}
          {route === "classification" && <ClassificationPage />}
          {route === "logs" && <LogsPage />}
        </main>
      </div>

      <footer className="statusbar">
        <span>{status}</span>
        {result ? <span>{result.runId ? `扫描时间：${new Date(result.scannedAt).toLocaleString()}` : "扫描中，尚未写入 SQLite"}</span> : <span>等待任务</span>}
      </footer>
    </div>
  );
}

function ScanPage({
  path,
  setPath,
  result,
  expanded,
  setExpanded,
  settings,
  updateSettings,
  isScanning,
  scan
}: any) {
  const allRows = useMemo(() => (result ? flattenTree(result.root, expanded) : []), [expanded, result]);
  const rows = useMemo(() => allRows.slice(0, MAX_VISIBLE_TREE_ROWS), [allRows]);
  const scannedSize = result?.root.size ?? 0;
  const totalCapacity = result?.volumeInfo?.totalBytes ?? 0;
  const freeBytes = result?.volumeInfo?.freeBytes ?? 0;

  function toggle(id: string) {
    setExpanded((current: Set<string>) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  return (
    <section className="scan-page">
      <section className="ribbon">
        <div className="ribbon-group">
          <button className="tool-button primary" type="button" onClick={() => void scan()} disabled={isScanning} title="扫描当前路径">
            <Search size={18} />
            <span>扫描</span>
          </button>
          <button className="tool-button" type="button" onClick={() => void scan()} disabled={isScanning || !result} title="重新扫描">
            <RefreshCw className={clsx(isScanning && "spin")} size={18} />
            <span>刷新</span>
          </button>
        </div>
        <div className="ribbon-group wide">
          <label className="path-field">
            <Folder size={16} />
            <input value={path} onChange={(event) => setPath(event.target.value)} placeholder="例如 Z:\\ 或 \\\\100.85.139.99\\docker\\" />
          </label>
        </div>
        <div className="ribbon-group">
          <label className="select-field">
            <Database size={16} />
            <select value={settings.sizeUnit} onChange={(event) => void updateSettings({ sizeUnit: event.target.value as SizeUnit })}>
              {unitOptions.map((item) => (
                <option key={item.value} value={item.value}>{item.label}</option>
              ))}
            </select>
          </label>
          <label className="check-field">
            <input type="checkbox" checked={settings.includeHidden} onChange={(event) => void updateSettings({ includeHidden: event.target.checked })} />
            <span>隐藏项</span>
          </label>
          <label className="depth-field" title="扫描深度">
            <Settings size={15} />
            <input min={1} max={32} type="number" value={settings.maxDepth} onChange={(event) => void updateSettings({ maxDepth: Number(event.target.value) || 1 })} />
          </label>
          <label className="exclude-field" title="按目录名排除，使用英文逗号分隔">
            <span>排除</span>
            <input
              value={(settings.excludedDirNames ?? []).join(", ")}
              onChange={(event) => void updateSettings({ excludedDirNames: splitExcludedDirNames(event.target.value) })}
            />
          </label>
        </div>
      </section>

      <section className="workspace">
        <section className="summary-strip">
          <Metric label="总容量" value={totalCapacity ? formatSize(totalCapacity, settings.sizeUnit) : "等待读取"} />
          <Metric label="可用空间" value={totalCapacity ? formatSize(freeBytes, settings.sizeUnit) : "等待读取"} />
          <Metric label="已扫描" value={formatSize(scannedSize, settings.sizeUnit)} />
          <Metric label="文件/文件夹" value={`${formatNumber(result?.root.fileCount ?? 0)} / ${formatNumber(result?.root.folderCount ?? 0)}`} />
          <Metric label="读取问题" value={formatNumber(result?.errors.length ?? 0)} tone={result?.errors.length ? "warn" : "normal"} />
        </section>

        <TreeTable rows={rows} totalRows={allRows.length} expanded={expanded} unit={settings.sizeUnit} onToggle={toggle} />
      </section>
    </section>
  );
}

function LogsPage() {
  const [entries, setEntries] = useState<OperationLogEntry[]>([]);
  const [query, setQuery] = useState("");
  const [softWrap, setSoftWrap] = useState(true);
  const [clearedThroughId, setClearedThroughId] = useState(0);
  const [status, setStatus] = useState("正在读取操作日志。");

  useEffect(() => {
    let canceled = false;
    async function load() {
      try {
        const result = await diskApi.getOperationLogs({ limit: 1000 });
        if (!canceled) {
          setEntries(result.entries);
          setStatus(`已加载 ${result.entries.length} 条日志。`);
        }
      } catch (error) {
        if (!canceled) setStatus(error instanceof Error ? error.message : String(error));
      }
    }
    void load();
    const timer = window.setInterval(() => void load(), 2000);
    return () => {
      canceled = true;
      window.clearInterval(timer);
    };
  }, []);

  const visibleEntries = useMemo(() => {
    const base = entries.filter((entry) => entry.id > clearedThroughId);
    const text = query.trim().toLowerCase();
    if (!text) return base;
    return base.filter((entry) => formatLogEntry(entry).toLowerCase().includes(text));
  }, [clearedThroughId, entries, query]);

  function clearVisibleLogs() {
    const maxId = entries.reduce((max, entry) => Math.max(max, entry.id), clearedThroughId);
    setClearedThroughId(maxId);
  }

  return (
    <section className="logs-page">
      <div className="tool-header">
        <div>
          <h2>操作日志</h2>
          <p>查看扫描、SQLite 保存、重复文件查找和 AI 分类过程，日志同时写入 SQLite 与本地文本文件。</p>
        </div>
        <button className="action-button" type="button" onClick={clearVisibleLogs}>
          <Trash2 size={16} />
          清空显示
        </button>
      </div>

      <div className="log-toolbar">
        <label className="log-search-field">
          <Search size={15} />
          <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索日志" />
        </label>
        <button className={clsx("log-tool-btn", softWrap && "active")} type="button" title="自动换行" onClick={() => setSoftWrap((value) => !value)}>
          <WrapText size={16} />
        </button>
        <span>{status}</span>
      </div>

      <div className={clsx("log-viewport", softWrap && "wrap")}>
        {visibleEntries.length > 0 ? (
          visibleEntries.map((entry) => (
            <div className={clsx("log-row", `level-${entry.level.toLowerCase()}`)} key={entry.id}>
              <span className="log-time">{entry.createdAt}</span>
              <span className="log-module">[{entry.module}]</span>
              <span className="log-level">{entry.level}</span>
              <span className="log-action">{entry.action}</span>
              <span className="log-message">- {entry.message}</span>
              {entry.detail && <span className="log-detail"> {entry.detail}</span>}
            </div>
          ))
        ) : (
          <div className="log-empty">当前没有可显示的日志。</div>
        )}
      </div>
    </section>
  );
}

function DuplicatesPage() {
  const settings = useAppStore((state) => state.settings);
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
  const [status, setStatus] = useState("使用最近一次扫描明细查找重复文件。");
  const [loading, setLoading] = useState(false);

  async function search() {
    setLoading(true);
    try {
      const result = await diskApi.findDuplicateFiles();
      setGroups(result.groups);
      setStatus(`已从扫描批次 #${result.runId ?? "-"} 找到 ${result.groups.length} 组候选重复文件。`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="tool-page">
      <div className="tool-header">
        <div>
          <h2>重复文件</h2>
          <p>按文件名和大小从 SQLite 明细中查找候选重复项，删除前仍需人工确认内容质量。</p>
        </div>
        <button className="action-button" type="button" onClick={() => void search()} disabled={loading}>
          <Files size={16} />
          查找重复文件
        </button>
      </div>
      <p className="page-status">{status}</p>
      <div className="list-panel">
        {groups.map((group) => (
          <div className="duplicate-card" key={group.key}>
            <div className="duplicate-card-head">
              <b>{group.name}</b>
              <span>{group.count} 个文件，可能浪费 {formatSize(group.wastedSize, settings.sizeUnit)}</span>
            </div>
            {group.files.map((file) => (
              <div className="file-line" key={file.path}>
                <span>{formatSize(file.size, settings.sizeUnit)}</span>
                <code>{file.path}</code>
              </div>
            ))}
          </div>
        ))}
      </div>
    </section>
  );
}

function ClassificationPage() {
  const settings = useAppStore((state) => state.settings);
  const updateSettings = useAppStore((state) => state.updateSettings);
  const [rootPath, setRootPath] = useState(settings.defaultPath);
  const [targetRoot, setTargetRoot] = useState(settings.videoRoot);
  const [suggestions, setSuggestions] = useState<VideoClassificationSuggestion[]>([]);
  const [status, setStatus] = useState("参考 NAS 视频库分类方案生成建议，确认后再移动文件。");
  const [loading, setLoading] = useState(false);

  async function analyze() {
    setLoading(true);
    try {
      const result = await diskApi.classifyVideos({ rootPath, targetRoot, limit: 500 });
      setSuggestions(result.suggestions);
      await updateSettings({ videoRoot: targetRoot });
      setStatus(`生成 ${result.suggestions.length} 条分类建议。`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setLoading(false);
    }
  }

  async function move(id: number) {
    try {
      const result = await diskApi.moveClassifiedVideo(id);
      setSuggestions((items) => items.map((item) => (item.id === id ? { ...item, status: "moved" } : item)));
      setStatus(result.message);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <section className="tool-page">
      <div className="tool-header">
        <div>
          <h2>AI 视频分类</h2>
          <p>按电影、电视剧、纪录片、学习视频、综艺、动画等结构生成目标路径，移动前逐条确认。</p>
        </div>
        <button className="action-button" type="button" onClick={() => void analyze()} disabled={loading}>
          <Bot size={16} />
          智能分析
        </button>
      </div>
      <div className="form-grid">
        <label>
          <span>待整理目录</span>
          <input value={rootPath} onChange={(event) => setRootPath(event.target.value)} />
        </label>
        <label>
          <span>分类目标根目录</span>
          <input value={targetRoot} onChange={(event) => setTargetRoot(event.target.value)} />
        </label>
      </div>
      <p className="page-status">{status}</p>
      <div className="list-panel">
        {suggestions.map((item) => (
          <div className="classification-card" key={item.id}>
            <div>
              <b>{item.fileName}</b>
              <p>{item.reason}</p>
              <code>{item.sourcePath}</code>
              <code>{item.targetPath}</code>
            </div>
            <div className="classification-side">
              <span>{item.category} / {item.subcategory}</span>
              <span>{Math.round(item.confidence * 100)}%</span>
              <button type="button" disabled={item.status === "moved"} onClick={() => void move(item.id)}>
                {item.status === "moved" ? "已移动" : "确认移动"}
              </button>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

function Metric({ label, value, tone = "normal" }: { label: string; value: string; tone?: "normal" | "warn" }) {
  return (
    <div className={clsx("metric", tone === "warn" && "warn")}>
      <span>{label}</span>
      <b>{value}</b>
    </div>
  );
}

function TreeTable({
  rows,
  totalRows,
  expanded,
  unit,
  onToggle
}: {
  rows: Array<{ node: DiskNode; level: number }>;
  totalRows: number;
  expanded: Set<string>;
  unit: SizeUnit;
  onToggle: (id: string) => void;
}) {
  return (
    <section className="table-panel">
      <div className="table-header">
        <div>名称</div>
        <div>大小</div>
        <div>已分配</div>
        <div>文件</div>
        <div>文件夹</div>
        <div>占上层百分比</div>
      </div>
      <div className="table-body">
        {rows.length > 0 ? (
          <>
            {rows.map((row) => <TreeRow key={row.node.id} row={row} expanded={expanded.has(row.node.id)} unit={unit} onToggle={onToggle} />)}
            {totalRows > rows.length && (
              <div className="row-limit-notice">
                当前显示前 {formatNumber(rows.length)} 行，共 {formatNumber(totalRows)} 行。为避免窗口未响应，已限制单次渲染行数。
              </div>
            )}
          </>
        ) : (
          <div className="empty-state">
            <BarChart3 size={34} />
            <b>输入路径后开始扫描</b>
            <span>支持本地盘符、映射盘和 UNC 网络路径。</span>
          </div>
        )}
      </div>
    </section>
  );
}

function formatLogEntry(entry: OperationLogEntry) {
  return `${entry.createdAt} [${entry.module}] ${entry.level} ${entry.action} - ${entry.message}${entry.detail ? ` ${entry.detail}` : ""}`;
}

function splitExcludedDirNames(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function TreeRow({ row, expanded, unit, onToggle }: { row: { node: DiskNode; level: number }; expanded: boolean; unit: SizeUnit; onToggle: (id: string) => void }) {
  const { node, level } = row;
  const hasChildren = node.children.length > 0;
  return (
    <div className="tree-row" title={node.skipReason ? `${node.path}\n${node.skipReason}` : node.path}>
      <div className="name-cell" style={{ paddingLeft: 10 + level * 18 }}>
        <button className="expand-button" type="button" disabled={!hasChildren} onClick={() => onToggle(node.id)}>
          {hasChildren ? expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} /> : <span />}
        </button>
        {node.isDir ? expanded ? <FolderOpen className="folder-icon" size={17} /> : <Folder className="folder-icon" size={17} /> : <Files className="file-icon" size={16} />}
        <span className="node-size">{formatSize(node.size, unit)}</span>
        <span className="node-name">{node.name}</span>
        {node.skipped && <span className="skipped-badge">已跳过</span>}
        {node.truncated && <span className="truncated-badge">已截断</span>}
      </div>
      <div>{formatSize(node.size, unit)}</div>
      <div>{formatSize(node.allocatedSize, unit)}</div>
      <div>{formatNumber(node.fileCount)}</div>
      <div>{formatNumber(node.folderCount)}</div>
      <div className="percent-cell">
        <div className="bar-track">
          <div className="bar-fill" style={{ width: `${Math.min(100, Math.max(0, node.percent))}%` }} />
          <span>{node.percent.toFixed(1)} %</span>
        </div>
      </div>
    </div>
  );
}

function replaceNode(root: DiskNode, path: string, replacement: DiskNode): DiskNode {
  if (root.path === path) return replacement;
  return {
    ...root,
    children: root.children.map((child) => replaceNode(child, path, replacement))
  };
}

function recalculateTree(node: DiskNode): DiskNode {
  if (!node.isDir) {
    return { ...node, fileCount: 1, folderCount: 0, percent: node.percent ?? 0 };
  }
  const children = node.children.map(recalculateTree).sort((a, b) => b.size - a.size || a.name.localeCompare(b.name));
  const size = children.reduce((sum, child) => sum + child.size, 0);
  const allocatedSize = children.reduce((sum, child) => sum + child.allocatedSize, 0);
  const directFiles = children.filter((child) => !child.isDir).length;
  const fileCount = children.reduce((sum, child) => sum + (child.isDir ? child.fileCount : 1), 0);
  const folderCount = children.filter((child) => child.isDir).length + children.filter((child) => child.isDir).reduce((sum, child) => sum + child.folderCount, 0);
  const next = {
    ...node,
    size,
    allocatedSize,
    fileCount: Math.max(fileCount, directFiles),
    folderCount,
    children
  };
  return assignPercents(next, Math.max(1, next.size));
}

function assignPercents(node: DiskNode, baseSize: number): DiskNode {
  const percent = baseSize > 0 ? Math.round((node.size / baseSize) * 1000) / 10 : 0;
  const childBase = Math.max(1, node.size);
  return {
    ...node,
    percent,
    children: node.children.map((child) => assignPercents(child, childBase))
  };
}

function flattenTree(root: DiskNode, expanded: Set<string>) {
  const rows: Array<{ node: DiskNode; level: number }> = [];
  const walk = (node: DiskNode, level: number) => {
    rows.push({ node, level });
    if (!expanded.has(node.id)) return;
    for (const child of node.children) walk(child, level + 1);
  };
  walk(root, 0);
  return rows;
}

function yieldToUi() {
  return new Promise<void>((resolve) => window.setTimeout(resolve, 0));
}

function formatNumber(value: number) {
  return new Intl.NumberFormat("zh-CN").format(value);
}

function formatSize(bytes: number, unit: SizeUnit) {
  const units: Array<{ key: SizeUnit; label: string; value: number }> = [
    { key: "tb", label: "TB", value: 1024 ** 4 },
    { key: "gb", label: "GB", value: 1024 ** 3 },
    { key: "mb", label: "MB", value: 1024 ** 2 },
    { key: "kb", label: "KB", value: 1024 },
    { key: "b", label: "B", value: 1 }
  ];
  const selected = unit === "auto" ? units.find((item) => bytes >= item.value) ?? units[units.length - 1] : units.find((item) => item.key === unit)!;
  const value = selected.value === 1 ? bytes : bytes / selected.value;
  const digits = selected.value >= 1024 ** 3 ? 2 : selected.value >= 1024 ** 2 ? 1 : 0;
  return `${new Intl.NumberFormat("zh-CN", { maximumFractionDigits: digits }).format(value)} ${selected.label}`;
}

async function startWindowDrag(event: React.MouseEvent<HTMLElement>) {
  if ((event.target as HTMLElement).closest("button, input, select, textarea, a")) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().startDragging();
  } catch {
    // Browser preview has no native window API.
  }
}

async function minimizeWindow() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  } catch {
    // Browser preview has no native window API.
  }
}

async function toggleMaximizeWindow() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const appWindow = getCurrentWindow();
    if (await appWindow.isMaximized()) await appWindow.unmaximize();
    else await appWindow.maximize();
  } catch {
    // Browser preview has no native window API.
  }
}

async function closeWindow() {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  } catch {
    // Browser preview has no native window API.
  }
}
