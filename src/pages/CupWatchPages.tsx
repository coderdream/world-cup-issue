import { useEffect, useState } from "react";
import clsx from "clsx";
import {
  BarChart3,
  Bell,
  Bot,
  CheckCircle2,
  ChevronDown,
  Crown,
  Database,
  ExternalLink,
  Eye,
  KeyRound,
  Link as LinkIcon,
  RefreshCw,
  Search,
  Shield,
  SlidersHorizontal,
  Sparkles,
  Star,
  Trophy,
  Zap
} from "lucide-react";
import { APP_VERSION, bracketNodes, teams } from "@/data/worldCupData";
import { useCupStore } from "@/store/useCupStore";
import { AiModelConfigDialog } from "@/components/AiModelConfigDialog";
import { getAiModelConfig } from "@/domain/aiConfig";
import { getFeaturedMatch, getNextScheduledMatch, isMatchVisibleByStatus } from "@/domain/matches";
import { createPrediction, pickLabel } from "@/domain/predictions";
import { cupwatchApi } from "@/lib/api/cupwatch";
import type { Match, Prediction, ResultPick, StandingRow, Team } from "@/types";
import { eloOdds, formatDateTime, formatScore, getMatchStartMs, getNextMatch, getStandings, getTeam, getTodayMatches } from "@/utils/standings";
import { TeamFlag, TeamName } from "@/components/Flag";
import { checkAndInstallUpdate, type UpdateCheckResult } from "@/services/updateService";

const groupLabels = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L"];

export function OverviewPage() {
  const matches = useCupStore((state) => state.matches);
  const favorites = useCupStore((state) => state.favorites);
  const settings = useCupStore((state) => state.settings);
  const lastUpdated = useCupStore((state) => state.lastUpdated);
  const refreshMatches = useCupStore((state) => state.refreshMatches);
  const setRoute = useCupStore((state) => state.setRoute);
  const featured = getFeaturedMatch(matches) ?? getNextMatch(matches);
  const next = getNextScheduledMatch(matches) ?? getNextMatch(matches);
  const today = getTodayMatches(matches);
  const standings = getStandings("I", matches).I;
  const countdown = useCountdown(next);
  const favoriteTeams = favorites.map(getTeam);
  const openAiAnalysis = () => setRoute("ai");

  return (
    <div className="page overview-page">
      <UpdateLine time={lastUpdated} onRefresh={refreshMatches} />
      <div className="overview-hero">
        <Panel className={clsx("focus-card", `status-${featured.status}`)}>
          <div className="panel-label gold"><Bell size={15} /> {featured.status === "live" ? "正在进行" : "下一场关注"}</div>
          <div className="match-hero">
            <TeamHero teamId={featured.homeTeamId} />
            <div className="match-center">
              <div className="venue-line">{featured.group} 组 · {featured.venue}</div>
              {featured.status === "live" ? (
                <>
                  <div className="live-score">{formatLiveScore(featured)}</div>
                  <div className="date-line live-status">LIVE · 进行中</div>
                </>
              ) : (
                <>
                  <div className="vs">VS</div>
                  <div className="date-line">{formatDateTime(featured)}</div>
                </>
              )}
              <button className="ai-link" type="button" onClick={openAiAnalysis}>
                <Sparkles size={13} /> 点击查看 AI 分析 〉
              </button>
            </div>
            <TeamHero teamId={featured.awayTeamId} />
          </div>
        </Panel>
        <Panel className="countdown-card">
          <div className="panel-label gold"><Bell size={15} /> 下一场倒计时</div>
          <div className="countdown-teams">
            <TeamHero teamId={next.homeTeamId} compact />
            <span>VS</span>
            <TeamHero teamId={next.awayTeamId} compact />
          </div>
          <p>距开赛（北京时间 {formatDateTime(next)}）</p>
          <div className="countdown">{countdown}</div>
          <button className="primary-soft" type="button">悬浮比分条</button>
        </Panel>
      </div>

      <SectionTitle icon={<CalendarGlyph />} title="今日赛程 · 北京时间" action="查看全部赛程 〉" />
      <div className="today-grid">
        {today.map((match) => (
          <MatchMiniCard key={match.id} match={match} spoiler={settings?.spoilerMode} onAiAnalysis={openAiAnalysis} />
        ))}
      </div>

      <div className="two-column">
        <Panel>
          <SectionTitle icon={<Star size={16} />} title="我的关注球队" inline />
          {favoriteTeams.length === 0 ? (
            <div className="empty compact">
              <p>还没有关注球队，去「球队」页加星标～</p>
              <button className="outline-btn" type="button">＋ 添加关注</button>
            </div>
          ) : (
            <div className="favorite-list">
              {favoriteTeams.map((team) => <TeamPill key={team.id} team={team} />)}
            </div>
          )}
        </Panel>
        <Panel>
          <SectionTitle icon={<BarChart3 size={16} />} title="I 组形势" action="绿=出线区 · 金=最佳第三" inline />
          <StandingTable rows={standings} compact />
        </Panel>
      </div>
    </div>
  );
}

export function FloatingScorebar() {
  const matches = useCupStore((state) => state.matches);
  const next = getNextMatch(matches);
  const home = getTeam(next.homeTeamId);
  const away = getTeam(next.awayTeamId);

  return (
    <div className="floating-scorebar">
      <div className="scorebar-brand"><Trophy size={16} /> 杯况</div>
      <div className="scorebar-match">
        <TeamFlag team={home} size="sm" /><b>{home.nameZh}</b>
        <strong>VS</strong>
        <TeamFlag team={away} size="sm" /><b>{away.nameZh}</b>
      </div>
      <div className="scorebar-time">{formatDateTime(next)}</div>
    </div>
  );
}

export function SchedulePage() {
  const matches = useCupStore((state) => state.matches);
  const settings = useCupStore((state) => state.settings);
  const lastUpdated = useCupStore((state) => state.lastUpdated);
  const refreshMatches = useCupStore((state) => state.refreshMatches);
  const [group, setGroup] = useState("全部小组");
  const [stage, setStage] = useState("全部阶段");
  const [query, setQuery] = useState("");
  const filtered = matches.filter((match) => {
    const home = getTeam(match.homeTeamId);
    const away = getTeam(match.awayTeamId);
    const groupMatch = group === "全部小组" || `${match.group} 组` === group;
    const stageMatch = stage === "全部阶段" || match.stage === stage || (stage === "淘汰赛" && match.stage !== "小组赛");
    const text = `${home.nameZh}${home.nameEn}${away.nameZh}${away.nameEn}${match.venue}`.toLowerCase();
    return groupMatch && stageMatch && text.includes(query.toLowerCase());
  });

  return (
    <div className="page">
      <FilterBar>
        <Field label="小组">
          <select value={group} onChange={(event) => setGroup(event.target.value)}>
            <option>全部小组</option>
            {groupLabels.map((item) => <option key={item}>{item} 组</option>)}
          </select>
        </Field>
        <Field label="阶段">
          <select value={stage} onChange={(event) => setStage(event.target.value)}>
            <option>全部阶段</option>
            <option>小组赛</option>
            <option>32 强</option>
            <option>16 强</option>
            <option>1/4 决赛</option>
            <option>半决赛</option>
            <option>季军赛</option>
            <option>决赛</option>
            <option>淘汰赛</option>
          </select>
        </Field>
        <Field label="关键词">
          <div className="input-with-icon">
            <Search size={16} />
            <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索球队 / 场馆..." />
          </div>
        </Field>
        <div className="filter-spacer" />
        <InlineRefresh time={lastUpdated} onRefresh={refreshMatches} />
        <button className="outline-btn small" type="button"><Eye size={14} /> 防剧透</button>
      </FilterBar>

      <Panel className="schedule-table-panel">
        <table className="schedule-table">
          <thead>
            <tr>
              <th>北京时间</th>
              <th>阶段</th>
              <th>主队</th>
              <th>比分</th>
              <th>客队</th>
              <th>场馆</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((match) => (
              <tr className={clsx(match.status === "live" && "live-row")} key={match.id}>
                <td><b>{match.time}</b><small>{match.date}</small></td>
                <td><span className="blue-tag">{match.group} 组</span></td>
                <td><TeamName teamId={match.homeTeamId} /></td>
                <td className="score-cell"><b>{formatScore(match, settings?.spoilerMode)}</b><small>{statusText(match.status)}</small></td>
                <td><TeamName teamId={match.awayTeamId} /></td>
                <td className="venue">{match.venue}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <div className="table-foot">共 {matches.length} 场 · 当前显示 {filtered.length} 场 <span>全程北京时间 UTC+8 · 与 FIFA 无关</span></div>
      </Panel>
    </div>
  );
}

export function ScoresPage() {
  const matches = useCupStore((state) => state.matches);
  const settings = useCupStore((state) => state.settings);
  const lastUpdated = useCupStore((state) => state.lastUpdated);
  const refreshMatches = useCupStore((state) => state.refreshMatches);
  const setRoute = useCupStore((state) => state.setRoute);
  const [tab, setTab] = useState<"all" | "live" | "finished" | "scheduled">("all");
  const visible = matches.filter((match) => isMatchVisibleByStatus(match, tab));
  const stats = {
    all: matches.length,
    live: matches.filter((match) => match.status === "live").length,
    finished: matches.filter((match) => match.status === "finished").length,
    scheduled: matches.filter((match) => match.status === "scheduled").length
  };
  const openAiAnalysis = () => setRoute("ai");

  return (
    <div className="page">
      <div className="score-tabs">
        {[
          ["all", `全部 ${stats.all}`],
          ["live", `进行中 ${stats.live}`],
          ["finished", `已结束 ${stats.finished}`],
          ["scheduled", `未开始 ${stats.scheduled}`]
        ].map(([key, label]) => (
          <button className={clsx(tab === key && "active")} key={key} onClick={() => setTab(key as typeof tab)} type="button">{label}</button>
        ))}
        <div className="filter-spacer" />
        <InlineRefresh time={lastUpdated} onRefresh={refreshMatches} />
      </div>

      {visible.length === 0 ? (
        <EmptyState text="该状态下暂无比赛" subtext="数据可能有延迟，仅供参考 · 纯资讯，与 FIFA 无关" />
      ) : (
        <div className="score-grid">
          {visible.map((match) => <ScoreCard key={match.id} match={match} spoiler={settings?.spoilerMode} onAiAnalysis={openAiAnalysis} />)}
        </div>
      )}
    </div>
  );
}

export function StandingsPage() {
  const matches = useCupStore((state) => state.matches);
  const lastUpdated = useCupStore((state) => state.lastUpdated);
  const refreshMatches = useCupStore((state) => state.refreshMatches);
  const settings = useCupStore((state) => state.settings);
  const rowsByGroup = getStandings(undefined, matches);

  return (
    <div className="page">
      <div className="jump-row">
        <span>跳转小组:</span>
        {groupLabels.map((group) => <a href={`#group-${group}`} key={group}>{group}</a>)}
        <div className="filter-spacer" />
        <span className="legend green-dot">小组前二出线</span>
        <span className="legend gold-dot">最佳第三</span>
        <InlineRefresh time={lastUpdated} onRefresh={refreshMatches} />
      </div>
      <div className="standings-grid">
        {groupLabels.map((group) => (
          <Panel key={group} id={`group-${group}`} className="standing-card">
            <SectionTitle icon={<span className="group-chip">{group}</span>} title={`${group} 组`} inline />
            <StandingTable rows={rowsByGroup[group]} spoiler={settings?.spoilerMode} />
          </Panel>
        ))}
      </div>
      <p className="fineprint">积分规则：胜 3 平 1 负 0；同分依次比净胜球、进球数。最佳第三精确排名以官方为准。</p>
    </div>
  );
}

export function BracketPage() {
  const lastUpdated = useCupStore((state) => state.lastUpdated);
  const refreshMatches = useCupStore((state) => state.refreshMatches);
  const columns = [
    { key: "32", label: "32 强" },
    { key: "16", label: "16 强" },
    { key: "quarter", label: "1/4 决赛" },
    { key: "semi", label: "半决赛" },
    { key: "third", label: "季军赛" },
    { key: "final", label: "决赛" }
  ] as const;

  return (
    <div className="page bracket-page">
      <div className="page-action-row">
        <SectionTitle icon={<Trophy size={16} />} title="淘汰赛对阵图" action="小组赛结束后自动填入晋级球队" />
        <InlineRefresh time={lastUpdated} onRefresh={refreshMatches} />
      </div>
      <div className="bracket-board">
        {columns.map((column) => (
          <div className={`bracket-column bracket-${column.key}`} key={column.key}>
            <div className="bracket-title">{column.label}</div>
            {bracketNodes.filter((node) => node.round === column.key).map((node) => (
              <div className={clsx("bracket-node", node.round === "final" && "final-node")} key={node.id}>
                <span>{node.slotA}</span><b>?</b>
                <span>{node.slotB}</span><b>?</b>
                {node.venue && <small>{node.venue}</small>}
              </div>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

export function AiAnalysisPage() {
  const matches = useCupStore((state) => state.matches);
  const settings = useCupStore((state) => state.settings);
  const updateSettings = useCupStore((state) => state.updateSettings);
  const next = getNextMatch(matches);
  const odds = eloOdds(next.homeTeamId, next.awayTeamId);
  const home = getTeam(next.homeTeamId);
  const away = getTeam(next.awayTeamId);
  const [configOpen, setConfigOpen] = useState(false);
  const [aiGenerating, setAiGenerating] = useState(false);
  const [aiEvaluation, setAiEvaluation] = useState("");
  const [aiError, setAiError] = useState("");
  const aiConfig = getAiModelConfig(settings);
  const aiReady = Boolean(aiConfig.apiKey.trim() && aiConfig.baseUrl.trim() && aiConfig.model.trim());

  return (
    <div className="page ai-page">
      <div className="notice green"><Shield size={16} /> 以下为基于公开数据的赛事资讯分析，仅供观察参考，不构成任何投注 / 竞猜建议。</div>
      <Panel className="ai-hero">
        <div className="panel-label gold"><Sparkles size={16} /> 赛前 AI 洞察</div>
        <div className="match-select">{home.nameZh} vs {away.nameZh} · {formatDateTime(next)} <ChevronDown size={15} /></div>
        <div className="match-hero center">
          <TeamHero teamId={home.id} />
          <div className="vs">VS</div>
          <TeamHero teamId={away.id} />
        </div>
      </Panel>

      <div className="ai-grid">
        <Panel>
          <SectionTitle icon={<Sparkles size={16} />} title="胜负概率测算（资讯参考）" action="基于 Elo 评分模型" inline />
          <div className="elo-row"><span>{home.nameZh}实力评分 · Elo</span><b>{home.elo}</b><div /><b className="blue">{away.elo}</b></div>
          <Probability label={`${home.code} ${home.nameZh}胜`} value={odds.home} color="green" />
          <Probability label="平局" value={odds.draw} color="gray" />
          <Probability label={`${away.code} ${away.nameZh}胜`} value={odds.away} color="blue" />
          <p className="fineprint">概率为统计模型估算，比赛存在偶然性，仅供观察参考，非投注建议。</p>
        </Panel>
        <Panel>
          <SectionTitle icon={<Zap size={16} />} title="近期状态" inline />
          <TeamStatus team={home} />
          <TeamStatus team={away} />
        </Panel>
      </div>

      <Panel>
        <SectionTitle
          icon={<Bot size={16} />}
          title="AI 评估意见"
          action={(
            <div className="ai-section-actions">
              <button className="outline-btn small" type="button" onClick={() => void generateEvaluation()} disabled={aiGenerating}>
                <RefreshCw className={clsx(aiGenerating && "spin")} size={14} /> {aiEvaluation ? "重新生成" : "生成评估"}
              </button>
              <button className="model-pill small" type="button" onClick={() => setConfigOpen(true)}>
                <SlidersHorizontal size={14} /> 模型：{aiConfig.model || "未配置"}
              </button>
            </div>
          )}
          inline
        />
        <p className="muted">上方为 Elo 模型的确定性概率测算；点击按钮，让你配置的 AI 结合本场数据生成一段自然语言解读。</p>
        {aiEvaluation && <div className="ai-generated">{aiEvaluation}</div>}
        {aiError && <p className="setting-status warning">{aiError}</p>}
        <div className="ai-result-actions">
          <button className="primary-btn" type="button" onClick={() => void generateEvaluation()} disabled={aiGenerating}>
            <Sparkles className={clsx(aiGenerating && "spin")} size={16} /> {aiGenerating ? "生成中" : aiReady ? "生成 AI 评估" : "配置并生成 AI 评估"}
          </button>
          <span className="fineprint">本内容由 AI 生成，仅供参考，不构成投注 / 竞猜建议。</span>
        </div>
        <p className="fineprint">需先配置自己的 AI 模型；API Key 仅本地保存，请优先使用合规模型做赛事资讯解读。</p>
      </Panel>

      <Panel>
        <SectionTitle icon={<Bell size={16} />} title="趣味预测" action="独立娱乐 · 纯虚拟荣誉 · 与 AI 分析无关" inline />
        <div className="prediction-buttons">
          <button type="button">{home.nameZh}胜</button>
          <button type="button">平局</button>
          <button type="button">{away.nameZh}胜</button>
        </div>
      </Panel>

      <Panel>
        <SectionTitle icon={<BarChart3 size={16} />} title="数据对比" inline />
        {["已赛场次", "积分", "场均进球", "场均失球"].map((label) => (
          <div className="data-compare" key={label}><b>0</b><span>{label}</span><b className="blue">0</b></div>
        ))}
      </Panel>

      <Panel>
        <SectionTitle
          icon={<Sparkles size={16} />}
          title="AI 追问（仅资讯解读）"
          action={<button className="link-action" type="button" onClick={() => setConfigOpen(true)}>配置模型</button>}
          inline
        />
        <div className={clsx("warning-card", aiReady && "success")}>
          <b>{aiReady ? "AI 模型已配置" : "尚未配置 AI 模型"}</b>
          <span>{aiReady ? `${aiConfig.name} · ${aiConfig.model}` : "AI 追问需要你自己的 LLM API Key（支持 DeepSeek / Kimi / 智谱 / OpenAI / Claude 等）。"}</span>
          <button className="primary-btn small" type="button" onClick={() => setConfigOpen(true)}>去配置</button>
        </div>
        <div className="prompt-row">
          {["两队近期状态如何？", "数据上谁更占优，为什么？", "两队进攻和防守谁更强？", "解释一下这个胜负概率", "这场比赛的看点在哪？"].map((item) => (
            <button key={item} type="button">{item}</button>
          ))}
        </div>
        <textarea placeholder="问问球队近期状态、战术特点等资讯类问题...（不提供任何投注建议）" />
        <button className="primary-btn send-btn" type="button">提交</button>
      </Panel>
      {settings && (
        <AiModelConfigDialog
          open={configOpen}
          settings={settings}
          onClose={() => setConfigOpen(false)}
          onSave={updateSettings}
        />
      )}
    </div>
  );

  async function generateEvaluation() {
    if (!aiReady) {
      setConfigOpen(true);
      return;
    }
    if (aiGenerating) return;
    setAiGenerating(true);
    setAiError("");
    try {
      const result = await cupwatchApi.generateAiEvaluation({
        config: aiConfig,
        context: {
          matchId: next.id,
          homeTeam: home.nameZh,
          awayTeam: away.nameZh,
          kickoff: formatDateTime(next),
          venue: next.venue,
          status: next.status,
          score: formatLiveScore(next),
          oddsHome: odds.home,
          oddsDraw: odds.draw,
          oddsAway: odds.away
        }
      });
      if (result.ok) {
        setAiEvaluation(result.content);
      } else {
        setAiError(result.message || "AI 评估生成失败");
      }
    } catch (error) {
      setAiError(error instanceof Error ? error.message : "AI 评估生成失败");
    } finally {
      setAiGenerating(false);
    }
  }
}

export function PredictionsPage() {
  const predictions = useCupStore((state) => state.predictions);
  const matches = useCupStore((state) => state.matches);
  const savePrediction = useCupStore((state) => state.savePrediction);
  const next = getNextMatch(matches);
  const home = getTeam(next.homeTeamId);
  const away = getTeam(next.awayTeamId);
  const odds = eloOdds(next.homeTeamId, next.awayTeamId);

  const choose = (pick: ResultPick) => {
    void savePrediction(createPrediction(next, pick));
  };

  return (
    <div className="page predictions-page">
      <div className="notice gold"><CheckCircle2 size={16} /> 免费趣味预测：猜着玩、自娱自乐，无任何金钱 / 实物奖励，积分与徽章为虚拟荣誉，不可兑现 / 兑换 / 转让。</div>
      <div className="prediction-top">
        <Panel className="record-card">
          <SectionTitle icon={<CheckCircle2 size={16} />} title="我的战绩" action="虚拟称号 · 民间预测家" inline />
          <div className="record-rate">0%</div>
          <span>预测命中率</span>
          <div className="record-stats">
            <div><b>{predictions.length}</b><span>已猜</span></div>
            <div><b>0</b><span>猜对</span></div>
            <div><b>0</b><span>最长连胜</span></div>
          </div>
          <div className="ai-hit"><Sparkles size={14} /> AI 命中率（同口径回测）<b>{odds.home}%</b></div>
        </Panel>
        <Panel className="guess-card">
          <SectionTitle icon={<Trophy size={16} />} title="来猜这场（开赛前可改）" action="批量预测　换一场 〉" inline />
          <div className="guess-main">
            <small>I 组 · 北京时间 {formatDateTime(next)}</small><span>未开赛</span>
            <div className="match-hero center">
              <TeamHero teamId={home.id} compact />
              <div className="vs">VS</div>
              <TeamHero teamId={away.id} compact />
            </div>
          </div>
          <div className="odds-strip"><Sparkles size={14} /> AI 测算（Elo 模型：资讯参考，非建议）<span>{home.nameZh}胜 {odds.home}%</span><span>平 {odds.draw}%</span><span>{away.nameZh}胜 {odds.away}%</span></div>
          <div className="prediction-buttons">
            <button onClick={() => choose("home")} type="button">{home.nameZh}胜</button>
            <button onClick={() => choose("draw")} type="button">平局</button>
            <button onClick={() => choose("away")} type="button">{away.nameZh}胜</button>
          </div>
          <p className="fineprint">比赛结束后自动公布答案（结果取自公开赛事数据），纯属娱乐。</p>
        </Panel>
      </div>
      <SectionTitle icon={<Trophy size={16} />} title="待揭晓的预测" />
      {predictions.length ? <PredictionList predictions={predictions} /> : <EmptyState text="还没有进行中的预测，去上面猜一场吧～" />}
      <SectionTitle icon={<RefreshCw size={16} />} title="已揭晓的预测" />
      <EmptyState text="还没有揭晓的预测，去上面猜一场吧～" />
    </div>
  );
}

export function TeamsPage() {
  const [query, setQuery] = useState("");
  const favorites = useCupStore((state) => state.favorites);
  const toggleFavorite = useCupStore((state) => state.toggleFavorite);
  const filtered = teams.filter((team) => `${team.nameZh}${team.nameEn}${team.code}`.toLowerCase().includes(query.toLowerCase()));

  return (
    <div className="page">
      <div className="teams-head">
        <h2>48 支参赛球队</h2>
        <div className="input-with-icon search-box">
          <Search size={16} />
          <input placeholder="搜索球队..." value={query} onChange={(event) => setQuery(event.target.value)} />
        </div>
      </div>
      <div className="teams-grid">
        {filtered.map((team) => (
          <button className="team-card" key={team.id} onClick={() => void toggleFavorite(team.id)} type="button">
            <Star className={clsx(favorites.includes(team.id) && "favorited")} size={18} />
            <TeamFlag team={team} size="lg" />
            <b>{team.nameZh}</b>
            <small>{team.group} 组</small>
          </button>
        ))}
      </div>
    </div>
  );
}

export function SettingsPage() {
  const settings = useCupStore((state) => state.settings);
  const license = useCupStore((state) => state.license);
  const updateSettings = useCupStore((state) => state.updateSettings);
  const refreshMatches = useCupStore((state) => state.refreshMatches);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [tokenDraft, setTokenDraft] = useState(settings?.footballDataToken ?? "");
  const [tokenSaving, setTokenSaving] = useState(false);
  const [tokenTesting, setTokenTesting] = useState(false);
  const [tokenStatus, setTokenStatus] = useState("");

  useEffect(() => {
    setTokenDraft(settings?.footballDataToken ?? "");
  }, [settings?.footballDataToken]);

  if (!settings) return null;

  return (
    <div className="page settings-page">
      <Panel className="settings-panel">
        <SectionTitle icon={<Shield size={16} />} title="桌面体验" action="网页/手机做不到的功能" inline />
        <SettingRow title="置顶悬浮比分条" desc="看直播时第一屏弹出角落显示比分" action={<button className="outline-btn" type="button">开/关</button>} />
        <SettingRow title="全局热键" desc="任意界面一键呼出赛程（自定义改键后续支持）" action={<kbd>Ctrl + Alt + C</kbd>} />
        <SettingRow title="开机自启「盯盘模式」" desc="世界杯期间随系统启动并最小化托盘" action={<Switch checked={settings.launchOnBoot} onChange={(value) => void updateSettings({ launchOnBoot: value })} />} />
      </Panel>

      <Panel className="settings-panel">
        <SectionTitle icon={<Bell size={16} />} title="开赛提醒" inline />
        <SettingRow title="开赛系统通知" desc="比赛临近开球时弹出系统通知（纯赛程提醒，无投注）" action={<Switch checked={settings.notificationsEnabled} onChange={(value) => void updateSettings({ notificationsEnabled: value })} />} />
        <SettingRow
          title="提前提醒时间"
          desc="开球前多少分钟通知"
          action={<><input className="number-input" type="number" min={1} max={120} value={settings.reminderMinutes} onChange={(event) => void updateSettings({ reminderMinutes: Number(event.target.value) })} /><span className="muted">分钟</span></>}
        />
      </Panel>

      <Panel className="settings-panel">
        <SectionTitle icon={<Shield size={16} />} title="授权管理" inline />
        <div className="license-row"><span>当前状态</span><b><em>试用中</em> 剩余 {license?.remainingDays ?? 30} 天</b></div>
        <div className="license-row"><span>试用到期</span><b>{license?.expiresAt}</b></div>
        <div className="settings-actions">
          <button className="primary-btn" type="button"><Crown size={16} /> 查看订阅方案</button>
          <button className="outline-btn" type="button"><KeyRound size={16} /> 输入激活码</button>
          <button className="outline-btn" type="button"><RefreshCw size={16} /> 刷新状态</button>
          <button className="danger-btn" type="button">清除缓存</button>
        </div>
      </Panel>

      <Panel className="settings-panel">
        <SectionTitle icon={<Database size={16} />} title="数据源与防剧透" inline />
        <p className="muted">赛程/比分来自公开数据源：<b>openfootball</b>（CC0 公共领域，离线兜底）与 <b>football-data.org</b>（免费层，赛事代码 WC，增量比分）。仅使用公开授权数据，不抓取 CCTV / 咪咕等持权转播方。</p>
        <SettingRow
          title="football-data API Token（可选，但预测揭晓 / LIVE 比分需要它）"
          desc="免费 token 需每位用户自行注册（各用户的配额），仅本地保存。"
          action={
            <div className="token-row">
              <input type="password" value={tokenDraft} onChange={(event) => setTokenDraft(event.target.value)} placeholder="API Token" />
              <button className="primary-btn" type="button" onClick={() => void saveFootballDataToken()} disabled={tokenSaving}>
                {tokenSaving ? "保存中" : "保存"}
              </button>
              <button className="outline-btn" type="button" onClick={() => void testFootballDataToken()} disabled={tokenTesting || tokenSaving}>
                <RefreshCw className={clsx(tokenTesting && "spin")} size={14} /> {tokenTesting ? "测试中" : "测试Token"}
              </button>
            </div>
          }
        />
        {tokenStatus && <p className={clsx("setting-status", tokenStatus.includes("失败") && "warning")}>{tokenStatus}</p>}
        <SettingRow title="防剧透模式" desc="顶部「眼睛」图标一键隐藏全局比分，适合还没看回放时使用（偏好本地记忆）。" action={<Switch checked={settings.spoilerMode} onChange={() => void updateSettings({ spoilerMode: !settings.spoilerMode })} />} />
      </Panel>

      <Panel className="settings-panel">
        <SectionTitle icon={<InfoGlyph />} title="关于与更新" inline />
        <SettingRow
          title="软件更新"
          desc={`当前版本 v${APP_VERSION}`}
          action={<button className="outline-btn" type="button" onClick={() => void runUpdateCheck()} disabled={updateChecking}><RefreshCw className={clsx(updateChecking && "spin")} size={16} /> {updateChecking ? "检查中" : "检查更新"}</button>}
        />
        {updateResult && <p className={clsx("update-status", updateResult.status === "manual" && "warning", updateResult.status === "installed" && "success")}>{updateResult.message}</p>}
        <p className="fineprint">WorldCupIssue（世界杯组手）复刻杯况 CupWatch 的桌面观赛体验，是独立第三方资讯工具，仅展示赛程/比分/积分等公开事实数据。</p>
      </Panel>
    </div>
  );

  async function runUpdateCheck() {
    if (updateChecking) return;
    setUpdateChecking(true);
    setUpdateResult(null);
    try {
      setUpdateResult(await checkAndInstallUpdate());
    } finally {
      setUpdateChecking(false);
    }
  }

  async function saveFootballDataToken() {
    if (tokenSaving) return;
    setTokenSaving(true);
    setTokenStatus("");
    try {
      await updateSettings({ footballDataToken: tokenDraft.trim() });
      await refreshMatches();
      setTokenStatus("已保存并刷新数据");
    } catch (error) {
      setTokenStatus(error instanceof Error ? error.message : "保存失败");
    } finally {
      setTokenSaving(false);
    }
  }

  async function testFootballDataToken() {
    if (tokenTesting || tokenSaving) return;
    setTokenTesting(true);
    setTokenStatus("");
    let shouldRefresh = false;
    try {
      const token = tokenDraft.trim();
      await updateSettings({ footballDataToken: token });
      const result = await cupwatchApi.testFootballDataToken(token);
      setTokenStatus(result.message);
      shouldRefresh = result.ok;
    } catch (error) {
      setTokenStatus(error instanceof Error ? error.message : "Token 测试失败");
    } finally {
      setTokenTesting(false);
    }
    if (shouldRefresh) {
      void refreshMatches().catch((error) => {
        setTokenStatus(error instanceof Error ? `Token 可用，但刷新数据失败：${error.message}` : "Token 可用，但刷新数据失败");
      });
    }
  }
}

export function AboutPage() {
  return (
    <div className="page about-page">
      <h2>关于</h2>
      <p>WorldCupIssue（世界杯组手） · 2026 世界杯北京时间赛程与比分（纯资讯）</p>
      <Panel className="notice-card"><Shield size={20} /> WorldCupIssue 是独立第三方资讯工具，仅展示赛程、比分、积分等公开事实数据，与 FIFA 及官方转播机构无关。本软件不含、也绝不提供任何投注、下注、赔率、盘口或博彩导流功能。</Panel>
      <Panel>
        <h3>开发者 & 社区</h3>
        <div className="profile-row"><b>抓哇师</b><span>Java 全栈 AI 架构师</span><span>Agent 架构师</span></div>
        <p className="muted">专注 Java 后端 + 前端全栈工程实践，深耕大模型应用与 Agent 架构落地。</p>
        <table className="contact-table">
          <tbody>
            <tr><td>B 站</td><td>https://space.bilibili.com/520725002 <LinkIcon size={14} /></td></tr>
            <tr><td>知识星球</td><td>编号 91839984</td></tr>
            <tr><td>QQ 交流群</td><td>1082276867</td></tr>
            <tr><td>联系作者</td><td>QQ 770492966</td></tr>
          </tbody>
        </table>
      </Panel>
      <Panel>
        <h3>产品矩阵 · AI 桌面全家桶 <span>同一作者出品 · 点击看详情</span></h3>
        {[
          ["智码 AICoder", "AI 编程助手桌面管理客户端，多会话 / 多账号 / Token 统计"],
          ["Sigil · AI 凭据金库", "AI 凭据金库 · MCP 代理 —— AI 拿不到你的密钥"],
          ["Reeve · 服务器庄园总管", "SSH 服务器管理 + 安全 AI 接入"],
          ["RuoYi-Plus-UniApp", "业内首个适配 Claude Code 的企业级全栈框架"],
          ["灵动桌面应用开发框架", "面向 AI 时代的 Tauri 桌面应用快速开发框架"],
          ["AI 全能工作站", "一句话路由到 60+ 专业模块"]
        ].map(([title, desc]) => (
          <div className="product-row" key={title}><span className="product-icon">⌁</span><b>{title}</b><small>{desc}</small><ExternalLink size={14} /></div>
        ))}
      </Panel>
      <Panel className="terms-row">〉 用户协议与免责声明 <span>v1.1（2026-06-14 生效）</span></Panel>
    </div>
  );
}

function Panel({ children, className, id }: { children: React.ReactNode; className?: string; id?: string }) {
  return <section id={id} className={clsx("panel", className)}>{children}</section>;
}

function SectionTitle({ icon, title, action, inline = false }: { icon: React.ReactNode; title: string; action?: React.ReactNode; inline?: boolean }) {
  return (
    <div className={clsx("section-title", inline && "inline")}>
      <div>{icon}<h2>{title}</h2></div>
      {action && <span>{action}</span>}
    </div>
  );
}

function UpdateLine({ time, onRefresh }: { time: string | null; onRefresh: () => Promise<void> }) {
  const [refreshing, setRefreshing] = useState(false);

  const refresh = async () => {
    if (refreshing) return;
    setRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setRefreshing(false);
    }
  };

  return (
    <div className="update-line">
      <button className="refresh-inline-btn" type="button" onClick={() => void refresh()} disabled={refreshing} title="刷新公开赛事数据">
        <RefreshCw className={clsx(refreshing && "spin")} size={14} />
      </button>
      <span>上次更新 {time ?? "未更新"}</span>
    </div>
  );
}

function InlineRefresh({ time, onRefresh }: { time: string | null; onRefresh: () => Promise<void> }) {
  const [refreshing, setRefreshing] = useState(false);

  const refresh = async () => {
    if (refreshing) return;
    setRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setRefreshing(false);
    }
  };

  return (
    <button className="inline-refresh" type="button" onClick={() => void refresh()} disabled={refreshing} title="立即拉取最新数据">
      <RefreshCw className={clsx(refreshing && "spin")} size={14} /> 上次更新 {time ?? "未更新"}
    </button>
  );
}

function useCountdown(match: Match) {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [match.id]);

  const remaining = Math.max(0, Math.floor((getMatchStartMs(match) - now) / 1000));
  const hours = Math.floor(remaining / 3600);
  const minutes = Math.floor((remaining % 3600) / 60);
  const seconds = remaining % 60;
  return `${padTime(hours)}:${padTime(minutes)}:${padTime(seconds)}`;
}

function padTime(value: number) {
  return value.toString().padStart(2, "0");
}

function TeamHero({ teamId, compact = false }: { teamId: string; compact?: boolean }) {
  const team = getTeam(teamId);
  return (
    <div className={clsx("team-hero", compact && "compact")}>
      <TeamFlag team={team} size={compact ? "md" : "lg"} />
      <b>{team.nameZh}</b>
    </div>
  );
}

function MatchMiniCard({ match, spoiler, onAiAnalysis }: { match: Match; spoiler?: boolean; onAiAnalysis: () => void }) {
  return (
    <Panel className={clsx("match-mini-card", `status-${match.status}`)}>
      <div className="card-meta">{match.group} 组 · {match.venue}<span>{statusText(match.status)}</span></div>
      <div className="score-line"><TeamName teamId={match.homeTeamId} /><b>{match.score.home ?? "-"}</b></div>
      <div className="score-line"><TeamName teamId={match.awayTeamId} /><b>{spoiler && match.status === "finished" ? "·" : match.score.away ?? "-"}</b></div>
      <button className="ai-link" type="button" onClick={onAiAnalysis}><Sparkles size={13} /> AI 分析 〉</button>
    </Panel>
  );
}

function ScoreCard({ match, spoiler, onAiAnalysis }: { match: Match; spoiler?: boolean; onAiAnalysis: () => void }) {
  return (
    <Panel className={clsx("score-card", `status-${match.status}`)}>
      <div className="card-meta">{match.group} 组 · {match.venue}<span>{statusText(match.status)}</span></div>
      <div className="score-line large"><TeamName teamId={match.homeTeamId} /><b>{formatScoreValue(match.score.home, spoiler && match.status === "finished")}</b></div>
      <div className="score-line large"><TeamName teamId={match.awayTeamId} /><b>{formatScoreValue(match.score.away, spoiler && match.status === "finished")}</b></div>
      <button className="ai-link" type="button" onClick={onAiAnalysis}><Sparkles size={13} /> AI 分析 〉</button>
    </Panel>
  );
}

function formatScoreValue(value: number | null, spoiler?: boolean) {
  if (spoiler) return "·";
  return value ?? "-";
}

function formatLiveScore(match: Match) {
  return `${formatScoreValue(match.score.home)} - ${formatScoreValue(match.score.away)}`;
}

function StandingTable({ rows, compact = false }: { rows: StandingRow[]; spoiler?: boolean; compact?: boolean }) {
  return (
    <table className={clsx("standing-table", compact && "compact")}>
      <thead>
        <tr><th>#</th><th>球队</th><th>赛</th><th>胜</th><th>平</th><th>负</th><th>净</th><th>分</th></tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.teamId} className={clsx(row.rank <= 2 && "qualified", row.rank === 2 && "third-candidate")}>
            <td>{row.rank}</td>
            <td><TeamName teamId={row.teamId} /></td>
            <td>{row.played}</td>
            <td>{row.wins}</td>
            <td>{row.draws}</td>
            <td>{row.losses}</td>
            <td className={row.goalDiff > 0 ? "positive" : row.goalDiff < 0 ? "negative" : ""}>{row.goalDiff > 0 ? `+${row.goalDiff}` : row.goalDiff}</td>
            <td><b>{row.points}</b></td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function statusText(status: Match["status"]) {
  if (status === "finished") return "完场";
  if (status === "live") return "LIVE";
  return "未开赛";
}

function Probability({ label, value, color }: { label: string; value: number; color: "green" | "gray" | "blue" }) {
  return (
    <div className="probability-row">
      <span>{label}</span>
      <div><i className={color} style={{ width: `${value}%` }} /></div>
      <b>{value}%</b>
    </div>
  );
}

function TeamStatus({ team }: { team: Team }) {
  return <div className="team-status"><TeamFlag team={team} size="sm" /><b>{team.nameZh}</b><small>暂无</small></div>;
}

function EmptyState({ text, subtext }: { text: string; subtext?: string }) {
  return (
    <Panel className="empty-state">
      <div className="empty-icon">▰</div>
      <p>{text}</p>
      {subtext && <span>{subtext}</span>}
    </Panel>
  );
}

function PredictionList({ predictions }: { predictions: Prediction[] }) {
  return (
    <Panel>
      {predictions.map((prediction) => {
        const match = useCupStore.getState().matches.find((item) => item.id === prediction.matchId);
        if (!match) return null;
        return <div className="prediction-row" key={prediction.matchId}>{getTeam(match.homeTeamId).nameZh} vs {getTeam(match.awayTeamId).nameZh}<b>{pickLabel(prediction.pick)}</b></div>;
      })}
    </Panel>
  );
}

function FilterBar({ children }: { children: React.ReactNode }) {
  return <div className="filter-bar">{children}</div>;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="field"><span>{label}</span>{children}</label>;
}

function SettingRow({ title, desc, action }: { title: string; desc: string; action: React.ReactNode }) {
  return <div className="setting-row"><div><b>{title}</b><span>{desc}</span></div><div className="setting-action">{action}</div></div>;
}

function Switch({ checked, onChange }: { checked: boolean; onChange: (checked: boolean) => void }) {
  return <button className={clsx("switch", checked && "checked")} onClick={() => onChange(!checked)} type="button"><span /></button>;
}

function TeamPill({ team }: { team: Team }) {
  return <div className="team-pill"><TeamFlag team={team} size="md" /><b>{team.nameZh}</b><small>{team.group} 组</small></div>;
}

function CalendarGlyph() {
  return <span className="glyph">▣</span>;
}

function InfoGlyph() {
  return <span className="glyph">ⓘ</span>;
}
