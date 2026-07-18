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
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { Panel, SectionTitle } from "@/pages/primitives";
import { frameworkApi } from "@/services/frameworkApi";
import { useAppStore } from "@/store/useAppStore";
import { useDashboard } from "@/pages/useDashboard";

interface SearchOptions {
  matchCase: boolean;
  wholeWord: boolean;
  regex: boolean;
}

export function QuarkPage() {
  const settings = useAppStore((state) => state.settings);
  const [years, setYears] = useState(settings.quarkYears);
  const [message, setMessage] = useState("");
  const [busyCommand, setBusyCommand] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [searchOptions, setSearchOptions] = useState<SearchOptions>({ matchCase: false, wholeWord: false, regex: false });
  const [filterMatches, setFilterMatches] = useState(false);
  const [activeMatchIndex, setActiveMatchIndex] = useState(0);
  const [softWrap, setSoftWrap] = useState(true);
  const [clearedThrough, setClearedThrough] = useState(0);
  const [lastLoadedAt, setLastLoadedAt] = useState("");
  const { dashboard, refresh } = useDashboard();
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const lineRefs = useRef(new Map<number, HTMLDivElement>());
  const stickToBottomRef = useRef(true);

  useEffect(() => {
    const timer = window.setInterval(() => void refresh(), 2000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const logs = dashboard?.quark.logs ?? [];

  useEffect(() => {
    if (!dashboard) return;
    stickToBottomRef.current = isNearBottom(viewportRef.current);
    setLastLoadedAt(new Date().toLocaleTimeString());
  }, [dashboard]);

  useEffect(() => {
    if (stickToBottomRef.current) scrollToEnd();
  }, [logs.length]);

  useEffect(() => {
    setActiveMatchIndex(0);
  }, [query, searchOptions.matchCase, searchOptions.regex, searchOptions.wholeWord]);

  const searchText = query.trim();
  const searchRegex = useMemo(() => buildSearchRegex(query, searchOptions), [query, searchOptions]);
  const sessionLogs = useMemo(() => logs.slice(clearedThrough), [clearedThrough, logs]);
  const visibleLogs = useMemo(() => {
    if (!filterMatches || !searchText || !searchRegex) return sessionLogs;
    return sessionLogs.filter((line) => regexTest(searchRegex, line));
  }, [filterMatches, searchRegex, searchText, sessionLogs]);
  const matches = useMemo(() => {
    if (!searchText || !searchRegex) return [];
    return visibleLogs.flatMap((line, lineIndex) => [...line.matchAll(searchRegex)].map(() => lineIndex));
  }, [searchRegex, searchText, visibleLogs]);
  const regexInvalid = Boolean(searchText && !searchRegex);

  async function run(command: string, label: string, useInputYears = false) {
    setBusyCommand(command);
    setMessage(`正在执行：${label}`);
    try {
      const result = await frameworkApi.runVideoWorkflow({ command, years: useInputYears ? years : undefined });
      setMessage(result.message);
      await refresh();
    } catch (err) {
      setMessage(err instanceof Error ? err.message : String(err));
    } finally {
      setBusyCommand(null);
    }
  }

  function goToMatch(direction: "prev" | "next") {
    if (matches.length === 0) return;
    const nextIndex = direction === "prev"
      ? (activeMatchIndex - 1 + matches.length) % matches.length
      : (activeMatchIndex + 1) % matches.length;
    setActiveMatchIndex(nextIndex);
    window.setTimeout(() => lineRefs.current.get(matches[nextIndex])?.scrollIntoView({ block: "nearest" }), 0);
  }

  function clearCurrentView() {
    setClearedThrough(logs.length);
    setActiveMatchIndex(0);
  }

  function scrollToEnd() {
    const node = viewportRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }

  return (
    <section className="studio-page quark-page">
      <div className="quark-grid">
        <div className="studio-panel">
          <h2>Quark 状态</h2>
          <dl className="quark-status">
            <dt>Token有效性</dt><dd>{dashboard?.quark.tokenValid ?? "-"}</dd>
            <dt>Cookie文件</dt><dd>{dashboard?.quark.cookieFile ?? "-"}</dd>
            <dt>Cookie更新时间</dt><dd>{dashboard?.quark.cookieUpdatedAt ?? "-"}</dd>
            <dt>根目录返回数量</dt><dd>{dashboard?.quark.rootItemCount ?? 0}</dd>
            <dt>最近校验结果</dt><dd>{dashboard?.quark.latestResult ?? "-"}</dd>
          </dl>
        </div>
        <div className="studio-panel">
          <h2>同步操作</h2>
          <label className="quark-year-row"><span>同步年份</span><input value={years} onChange={(event) => setYears(event.target.value)} /></label>
          <div className="quark-actions">
            <button type="button" disabled={busyCommand !== null} onClick={() => void run("quark-check", "校验 Token")}>校验 Token</button>
            <button className="primary-action" type="button" disabled={busyCommand !== null} onClick={() => void run("quark-refresh", "获取新 Token")}>获取新 Token</button>
            <button type="button" disabled={busyCommand !== null} onClick={() => void run("quark-browser", "打开 Quark 浏览器")}>打开 Quark 浏览器</button>
            <button type="button" disabled={busyCommand !== null} onClick={() => void run("daily-sync", "同步默认年份")}>同步默认年份</button>
            <button className="primary-action" type="button" disabled={busyCommand !== null} onClick={() => void run("daily-sync", "同步输入年份", true)}>同步输入年份</button>
            <button type="button" onClick={() => void frameworkApi.openVideoCreatorPath("quark_cookie")}>打开 Cookie 目录</button>
            <button type="button" onClick={() => void frameworkApi.openVideoCreatorPath("quark_sync")}>打开同步目录</button>
          </div>
          <ol className="quark-help">
            <li>校验 Token 会读取 Cookie 并请求 Quark 根目录。</li>
            <li>获取新 Token 会从 Quark 专用浏览器导出 Cookie。</li>
            <li>打开 Quark 浏览器后，请完成登录，再点击获取新 Token。</li>
            <li>同步默认年份使用配置中的年份；同步输入年份使用当前输入框。</li>
          </ol>
          {message && <p className="run-message">{message}</p>}
        </div>
      </div>

      <Panel className="logs-panel quark-logs-panel">
        <SectionTitle icon={<ListEnd size={16} />} title="Quark 日志" inline />
        <div className="log-scope-row">
          <span>当前显示</span>
          <b>本次启动后日志</b>
          <code>点击同步后持续追加本次 Quark 运行日志</code>
          <span>最后刷新</span>
          <b>{lastLoadedAt || "-"}</b>
          <span>日志条数</span>
          <b>{String(visibleLogs.length)}</b>
        </div>

        <div className="log-searchbar">
          <button className="log-search-icon" type="button" title="搜索日志"><ChevronRight size={15} /></button>
          <div className="log-search-input">
            <Search size={15} />
            <input aria-label="搜索 Quark 日志" placeholder="搜索日志" value={query} onChange={(event) => setQuery(event.target.value)} />
          </div>
          <button className="log-tool-btn" type="button" title="清空搜索" onClick={() => setQuery("")}><X size={14} /></button>
          <button className={searchOptions.matchCase ? "log-token-btn active" : "log-token-btn"} type="button" onClick={() => setSearchOptions((value) => ({ ...value, matchCase: !value.matchCase }))}>Cc</button>
          <button className={searchOptions.wholeWord ? "log-token-btn active" : "log-token-btn"} type="button" onClick={() => setSearchOptions((value) => ({ ...value, wholeWord: !value.wholeWord }))}>W</button>
          <button className={searchOptions.regex ? "log-token-btn active" : "log-token-btn"} type="button" onClick={() => setSearchOptions((value) => ({ ...value, regex: !value.regex }))}>.*</button>
          <span className={regexInvalid ? "log-match-count error" : "log-match-count"}>{regexInvalid ? "正则错误" : searchText ? `${matches.length ? activeMatchIndex + 1 : 0}/${matches.length}` : "0 results"}</span>
          <button className="log-tool-btn" disabled={matches.length === 0} type="button" title="上一处" onClick={() => goToMatch("prev")}><ArrowUp size={15} /></button>
          <button className="log-tool-btn" disabled={matches.length === 0} type="button" title="下一处" onClick={() => goToMatch("next")}><ArrowDown size={15} /></button>
          <button className={filterMatches ? "log-tool-btn active" : "log-tool-btn"} disabled={!searchText || regexInvalid} type="button" title="仅显示匹配" onClick={() => setFilterMatches((value) => !value)}><Filter size={15} /></button>
        </div>

        <div className="log-console-toolbar">
          <button className={softWrap ? "log-tool-btn active" : "log-tool-btn"} type="button" title="自动换行" onClick={() => setSoftWrap((value) => !value)}><WrapText size={16} /></button>
          <button className="log-tool-btn" type="button" title="滚动到底部" onClick={scrollToEnd}><ArrowDownToLine size={16} /></button>
          <button className="log-tool-btn danger" type="button" title="清空当前显示" onClick={clearCurrentView}><Trash2 size={16} /></button>
        </div>

        <div className={softWrap ? "log-viewport wrap quark-log-viewport" : "log-viewport quark-log-viewport"} ref={viewportRef} tabIndex={0}>
          {visibleLogs.length > 0 ? visibleLogs.map((line, index) => (
            <div
              className={matches[activeMatchIndex] === index ? "log-row active-search" : "log-row"}
              key={`${clearedThrough}-${index}-${line}`}
              ref={(node) => {
                if (node) lineRefs.current.set(index, node);
                else lineRefs.current.delete(index);
              }}
            >
              {renderHighlighted(line, searchText, searchRegex)}
            </div>
          )) : <div className="log-empty">本次启动后暂无 Quark 日志。</div>}
        </div>
      </Panel>
    </section>
  );
}

function buildSearchRegex(query: string, options: SearchOptions) {
  const trimmed = query.trim();
  if (!trimmed) return null;
  try {
    const source = options.regex ? trimmed : escapeRegex(trimmed);
    return new RegExp(options.wholeWord ? `\\b(?:${source})\\b` : source, options.matchCase ? "g" : "gi");
  } catch {
    return null;
  }
}

function regexTest(regex: RegExp, value: string) {
  regex.lastIndex = 0;
  const result = regex.test(value);
  regex.lastIndex = 0;
  return result;
}

function renderHighlighted(text: string, query: string, regex: RegExp | null) {
  if (!query || !regex) return text;
  const parts: ReactNode[] = [];
  let lastIndex = 0;
  regex.lastIndex = 0;
  for (const match of text.matchAll(regex)) {
    const value = match[0];
    const index = match.index ?? 0;
    if (!value) continue;
    if (index > lastIndex) parts.push(text.slice(lastIndex, index));
    parts.push(<mark key={`${index}-${value}`}>{value}</mark>);
    lastIndex = index + value.length;
  }
  if (lastIndex < text.length) parts.push(text.slice(lastIndex));
  return parts.length > 0 ? parts : text;
}

function escapeRegex(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function isNearBottom(node: HTMLDivElement | null) {
  if (!node) return true;
  return node.scrollHeight - node.scrollTop - node.clientHeight < 48;
}
