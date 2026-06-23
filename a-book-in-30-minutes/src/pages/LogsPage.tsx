import {
  ArrowDown,
  ArrowDownToLine,
  ArrowUp,
  ChevronRight,
  Filter,
  ListEnd,
  Search,
  Trash2,
  WrapText,
  X
} from "lucide-react";
import { type MouseEvent as ReactMouseEvent, type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { Panel, SectionTitle } from "@/pages/primitives";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import type { OperationLogEntry } from "@/types";

const LOG_LIMIT = 1000;
const SEARCH_HISTORY_KEY = "a-book-in-30-minutes-log-search-history";

interface ContextMenuState {
  x: number;
  y: number;
  rowId?: number;
}

interface SearchOptions {
  matchCase: boolean;
  wholeWord: boolean;
  regex: boolean;
}

export function LogsPage() {
  const currentTraceId = useAppStore((state) => state.materialsWorkbench.currentTraceId);
  const [entries, setEntries] = useState<OperationLogEntry[]>([]);
  const [clearedThroughId, setClearedThroughId] = useState(0);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [anchorId, setAnchorId] = useState<number | null>(null);
  const [softWrap, setSoftWrap] = useState(true);
  const [query, setQuery] = useState("");
  const [searchOptions, setSearchOptions] = useState<SearchOptions>({ matchCase: false, wholeWord: false, regex: false });
  const [filterMatches, setFilterMatches] = useState(false);
  const [activeMatchIndex, setActiveMatchIndex] = useState(0);
  const [searchHistory, setSearchHistory] = useState<string[]>(() => readSearchHistory());
  const [showHistory, setShowHistory] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [error, setError] = useState("");

  const logViewportRef = useRef<HTMLDivElement | null>(null);
  const rowRefs = useRef(new Map<number, HTMLDivElement>());

  useEffect(() => {
    let canceled = false;
    async function load() {
      try {
        const result = await frameworkApi.getOperationLogs({ limit: LOG_LIMIT, traceId: currentTraceId || undefined });
        if (!canceled) {
          setEntries(result.entries);
          setError("");
        }
      } catch (caught) {
        if (!canceled) setError(caught instanceof Error ? caught.message : String(caught));
      }
    }

    void load();
    const timer = window.setInterval(() => void load(), 2000);
    return () => {
      canceled = true;
      window.clearInterval(timer);
    };
  }, [currentTraceId]);

  useEffect(() => {
    const closeMenu = () => setContextMenu(null);
    window.addEventListener("click", closeMenu);
    return () => window.removeEventListener("click", closeMenu);
  }, []);

  useEffect(() => {
    function handleShortcut(event: KeyboardEvent) {
      if (!event.altKey) return;
      const key = event.key.toLowerCase();
      if (key === "c") {
        event.preventDefault();
        toggleSearchOption("matchCase");
      }
      if (key === "w") {
        event.preventDefault();
        toggleSearchOption("wholeWord");
      }
      if (key === "x") {
        event.preventDefault();
        toggleSearchOption("regex");
      }
    }
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  useEffect(() => {
    const value = query.trim();
    if (!value) return;
    const timer = window.setTimeout(() => saveSearch(value), 700);
    return () => window.clearTimeout(timer);
  }, [query, searchOptions.matchCase, searchOptions.regex, searchOptions.wholeWord]);

  const searchRegex = useMemo(() => buildSearchRegex(query, searchOptions), [query, searchOptions]);
  const searchText = query.trim();
  const baseEntries = useMemo(() => entries.filter((entry) => entry.id > clearedThroughId), [clearedThroughId, entries]);
  const visibleEntries = useMemo(() => {
    if (!filterMatches || !searchText || !searchRegex) return baseEntries;
    return baseEntries.filter((entry) => regexTest(searchRegex, formatLogEntry(entry)));
  }, [baseEntries, filterMatches, searchRegex, searchText]);
  const matches = useMemo(() => {
    if (!searchText || !searchRegex) return [];
    return visibleEntries.flatMap((entry) => {
      const text = formatLogEntry(entry);
      const found = [...text.matchAll(searchRegex)];
      return found.map((_, index) => ({ entryId: entry.id, index }));
    });
  }, [searchRegex, searchText, visibleEntries]);

  useEffect(() => {
    setActiveMatchIndex(0);
  }, [query, searchOptions.matchCase, searchOptions.regex, searchOptions.wholeWord]);

  const activeMatch = matches[activeMatchIndex] ?? null;
  const selectedEntries = visibleEntries.filter((entry) => selectedIds.has(entry.id));
  const regexInvalid = Boolean(searchText && !searchRegex);
  const displayedTraceId = currentTraceId || "";

  return (
    <div className="page logs-page">
      <Panel className="logs-panel">
        <SectionTitle icon={<ListEnd size={16} />} title="后台操作日志" inline />
        <div className="log-scope-row">
          <span>当前显示</span>
          <b>{currentTraceId ? "本次生成任务" : "本次启动后日志"}</b>
          <code>{displayedTraceId || "点击生成按钮后显示对应任务日志"}</code>
        </div>

        <div className="log-searchbar">
          <button className="log-search-icon" type="button" title="折叠搜索">
            <ChevronRight size={15} />
          </button>
          <div className="log-search-input">
            <Search size={15} />
            <input
              aria-label="搜索日志"
              onBlur={() => window.setTimeout(() => setShowHistory(false), 120)}
              onChange={(event) => setQuery(event.target.value)}
              onFocus={() => setShowHistory(true)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  saveSearch(query);
                  goToSearchMatch(event.shiftKey ? "prev" : "next");
                }
                if (event.key === "Escape") setShowHistory(false);
              }}
              placeholder="搜索日志"
              value={query}
            />
            {showHistory && searchHistory.length > 0 && (
              <div className="log-search-history">
                {searchHistory.map((item) => (
                  <button
                    key={item}
                    onMouseDown={(event) => {
                      event.preventDefault();
                      setQuery(item);
                      setShowHistory(false);
                      saveSearch(item);
                    }}
                    type="button"
                  >
                    {item}
                  </button>
                ))}
              </div>
            )}
          </div>
          <button className="log-tool-btn" type="button" title="清空搜索" onClick={() => setQuery("")}>
            <X size={14} />
          </button>
          <button
            className={searchOptions.matchCase ? "log-token-btn active" : "log-token-btn"}
            type="button"
            title="Match Case Alt+C"
            onClick={() => toggleSearchOption("matchCase")}
          >
            Cc
          </button>
          <button
            className={searchOptions.wholeWord ? "log-token-btn active" : "log-token-btn"}
            type="button"
            title="Words Alt+W"
            onClick={() => toggleSearchOption("wholeWord")}
          >
            W
          </button>
          <button
            className={searchOptions.regex ? "log-token-btn active" : "log-token-btn"}
            type="button"
            title="Regex Alt+X"
            onClick={() => toggleSearchOption("regex")}
          >
            .*
          </button>
          <span className={regexInvalid ? "log-match-count error" : "log-match-count"}>
            {regexInvalid ? "正则错误" : searchText ? `${matches.length ? activeMatchIndex + 1 : 0}/${matches.length}` : "0 results"}
          </span>
          <button className="log-tool-btn" disabled={matches.length === 0} type="button" title="上一处" onClick={() => goToSearchMatch("prev")}>
            <ArrowUp size={15} />
          </button>
          <button className="log-tool-btn" disabled={matches.length === 0} type="button" title="下一处" onClick={() => goToSearchMatch("next")}>
            <ArrowDown size={15} />
          </button>
          <button
            className={filterMatches ? "log-tool-btn active" : "log-tool-btn"}
            disabled={!searchText || regexInvalid}
            type="button"
            title="仅显示匹配"
            onClick={() => setFilterMatches((value) => !value)}
          >
            <Filter size={15} />
          </button>
        </div>

        <div className="log-console-toolbar">
          <button className="log-tool-btn" type="button" title="Up the Stack Trace" onClick={() => moveSelection("prev")}>
            <ArrowUp size={16} />
          </button>
          <button className="log-tool-btn" type="button" title="Down the Stack Trace" onClick={() => moveSelection("next")}>
            <ArrowDown size={16} />
          </button>
          <button className={softWrap ? "log-tool-btn active" : "log-tool-btn"} type="button" title="Soft-Wrap" onClick={() => setSoftWrap((value) => !value)}>
            <WrapText size={16} />
          </button>
          <button className="log-tool-btn" type="button" title="Scroll to End" onClick={() => scrollToEnd()}>
            <ArrowDownToLine size={16} />
          </button>
          <button className="log-tool-btn danger" type="button" title="Clear All" onClick={() => clearVisibleLogs()}>
            <Trash2 size={16} />
          </button>
        </div>

        {error && <p className="status error">{error}</p>}

        <div
          className={softWrap ? "log-viewport wrap" : "log-viewport"}
          onContextMenu={(event) => openContextMenu(event)}
          onKeyDown={(event) => {
            if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c") {
              event.preventDefault();
              void copySelectedLogs();
            }
          }}
          ref={logViewportRef}
          tabIndex={0}
        >
          {visibleEntries.length > 0 ? (
            visibleEntries.map((entry) => (
              <div
                className={buildLogRowClass(entry, activeMatch?.entryId === entry.id)}
                key={entry.id}
                onClick={(event) => selectEntry(entry.id, event)}
                onContextMenu={(event) => openContextMenu(event, entry.id)}
                ref={(node) => {
                  if (node) rowRefs.current.set(entry.id, node);
                  else rowRefs.current.delete(entry.id);
                }}
              >
                <span className="log-time">{entry.createdAt}</span>
                {entry.traceId && <span className="log-trace">[{entry.traceId}]</span>}
                <span className="log-module">[{entry.module}]</span>
                <span className={buildLogLevelClass(entry.level)}>{entry.level}</span>
                <span className="log-action">{entry.action}</span>
                <span className="log-message">- {renderHighlighted(entry.message)}</span>
                {entry.detail && <span className="log-detail">{renderHighlighted(entry.detail)}</span>}
              </div>
            ))
          ) : (
            <div className="log-empty">当前没有可显示的日志。</div>
          )}
        </div>

        {contextMenu && (
          <div className="log-context-menu" style={{ left: contextMenu.x, top: contextMenu.y }} onClick={(event) => event.stopPropagation()}>
            <button disabled={selectedEntries.length === 0} type="button" onClick={() => void copySelectedLogs()}>
              复制 <span>Ctrl+C</span>
            </button>
            <button className="danger" type="button" onClick={() => clearVisibleLogs()}>
              清空全部显示
            </button>
          </div>
        )}
      </Panel>
    </div>
  );

  function buildLogRowClass(entry: OperationLogEntry, activeSearchRow: boolean) {
    const selected = selectedIds.has(entry.id);
    return [
      "log-row",
      selected ? "selected" : "",
      activeSearchRow ? "active-search" : "",
      entry.level === "ERROR" ? "level-error" : entry.level === "WARN" ? "level-warn" : entry.level === "DEBUG" ? "level-debug" : ""
    ]
      .filter(Boolean)
      .join(" ");
  }

  function buildLogLevelClass(level: string) {
    if (level === "ERROR") return "log-level error";
    if (level === "WARN") return "log-level warn";
    if (level === "DEBUG") return "log-level debug";
    return "log-level";
  }

  function selectEntry(entryId: number, event: ReactMouseEvent<HTMLElement>) {
    setSelectedIds((current) => {
      if (event.shiftKey && anchorId !== null) {
        return selectRange(anchorId, entryId);
      }
      if (event.ctrlKey || event.metaKey) {
        const next = new Set(current);
        if (next.has(entryId)) next.delete(entryId);
        else next.add(entryId);
        return next;
      }
      return new Set([entryId]);
    });
    if (!event.shiftKey) setAnchorId(entryId);
  }

  function selectRange(fromId: number, toId: number) {
    const from = visibleEntries.findIndex((entry) => entry.id === fromId);
    const to = visibleEntries.findIndex((entry) => entry.id === toId);
    if (from < 0 || to < 0) return new Set([toId]);
    const [start, end] = from < to ? [from, to] : [to, from];
    return new Set(visibleEntries.slice(start, end + 1).map((entry) => entry.id));
  }

  function moveSelection(direction: "prev" | "next") {
    if (visibleEntries.length === 0) return;
    const currentId = selectedIds.size > 0 ? [...selectedIds][selectedIds.size - 1] : visibleEntries[visibleEntries.length - 1].id;
    const currentIndex = Math.max(0, visibleEntries.findIndex((entry) => entry.id === currentId));
    const nextIndex = direction === "prev" ? Math.max(0, currentIndex - 1) : Math.min(visibleEntries.length - 1, currentIndex + 1);
    const next = visibleEntries[nextIndex];
    setSelectedIds(new Set([next.id]));
    setAnchorId(next.id);
    scrollEntryIntoView(next.id);
  }

  function goToSearchMatch(direction: "prev" | "next") {
    if (matches.length === 0) return;
    const nextIndex = direction === "prev" ? (activeMatchIndex - 1 + matches.length) % matches.length : (activeMatchIndex + 1) % matches.length;
    const next = matches[nextIndex];
    setActiveMatchIndex(nextIndex);
    setSelectedIds(new Set([next.entryId]));
    setAnchorId(next.entryId);
    scrollEntryIntoView(next.entryId);
  }

  function toggleSearchOption(key: keyof SearchOptions) {
    setSearchOptions((current) => ({ ...current, [key]: !current[key] }));
  }

  function renderHighlighted(text: string) {
    if (!searchText || !searchRegex) return text;
    const segments: ReactNode[] = [];
    let lastIndex = 0;
    searchRegex.lastIndex = 0;
    for (const match of text.matchAll(searchRegex)) {
      const value = match[0];
      const index = match.index ?? 0;
      if (!value) continue;
      if (index > lastIndex) segments.push(text.slice(lastIndex, index));
      segments.push(<mark key={`${index}-${value}`}>{value}</mark>);
      lastIndex = index + value.length;
    }
    if (lastIndex < text.length) segments.push(text.slice(lastIndex));
    return segments.length > 0 ? segments : text;
  }

  function openContextMenu(event: ReactMouseEvent<HTMLElement>, rowId?: number) {
    event.preventDefault();
    event.stopPropagation();
    if (rowId && !selectedIds.has(rowId)) {
      setSelectedIds(new Set([rowId]));
      setAnchorId(rowId);
    }
    setContextMenu({ x: event.clientX, y: event.clientY, rowId });
  }

  async function copySelectedLogs() {
    const nativeSelection = window.getSelection()?.toString().trim();
    const text = nativeSelection || selectedEntries.map(formatLogEntry).join("\n");
    if (!text) return;
    await navigator.clipboard.writeText(text);
    setContextMenu(null);
  }

  function clearVisibleLogs() {
    const maxId = baseEntries.reduce((max, entry) => Math.max(max, entry.id), clearedThroughId);
    setClearedThroughId(maxId);
    setSelectedIds(new Set());
    setAnchorId(null);
    setContextMenu(null);
  }

  function scrollToEnd() {
    const node = logViewportRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }

  function scrollEntryIntoView(entryId: number) {
    window.setTimeout(() => rowRefs.current.get(entryId)?.scrollIntoView({ block: "nearest" }), 0);
  }

  function saveSearch(value: string) {
    const trimmed = value.trim();
    if (!trimmed) return;
    setSearchHistory((current) => {
      const next = [trimmed, ...current.filter((item) => item !== trimmed)].slice(0, 12);
      localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(next));
      return next;
    });
  }
}

function formatLogEntry(entry: OperationLogEntry) {
  return [
    entry.createdAt,
    entry.traceId ? `[${entry.traceId}]` : "",
    `[${entry.module}]`,
    entry.level,
    entry.action,
    "-",
    entry.message,
    entry.detail ? `: ${entry.detail}` : ""
  ].join(" ");
}

function buildSearchRegex(query: string, options: SearchOptions) {
  const trimmed = query.trim();
  if (!trimmed) return null;
  try {
    const source = options.regex ? trimmed : escapeRegex(trimmed);
    const pattern = options.wholeWord ? `\\b(?:${source})\\b` : source;
    return new RegExp(pattern, options.matchCase ? "g" : "gi");
  } catch {
    return null;
  }
}

function regexTest(regex: RegExp, value: string) {
  regex.lastIndex = 0;
  const matched = regex.test(value);
  regex.lastIndex = 0;
  return matched;
}

function escapeRegex(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function readSearchHistory() {
  try {
    const raw = localStorage.getItem(SEARCH_HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string").slice(0, 12) : [];
  } catch {
    return [];
  }
}
