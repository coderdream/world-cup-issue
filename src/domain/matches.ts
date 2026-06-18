import type { Match } from "@/types";

export const MATCH_PENDING_WINDOW_MINUTES = 180;

export function byMatchTime(a: Match, b: Match) {
  return `${a.date} ${a.time}`.localeCompare(`${b.date} ${b.time}`);
}

export function getMatchStartMs(match: Match) {
  const [year, month, day] = match.date.split("-").map(Number);
  const [hour, minute] = match.time.split(":").map(Number);
  if (!year || !month || !day || Number.isNaN(hour) || Number.isNaN(minute)) return Number.POSITIVE_INFINITY;
  return Date.UTC(year, month - 1, day, hour - 8, minute, 0);
}

export function getBeijingDate(now = new Date()) {
  return new Intl.DateTimeFormat("en-CA", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    timeZone: "Asia/Shanghai"
  }).format(now);
}

export function deriveMatchStatus(match: Match, now = new Date()): Match["status"] {
  const startMs = getMatchStartMs(match);
  if (!Number.isFinite(startMs)) return match.status;

  const nowMs = now.getTime();
  const pendingMs = startMs + MATCH_PENDING_WINDOW_MINUTES * 60 * 1000;
  const hasScore = match.score.home != null && match.score.away != null;

  if (match.status === "live" && nowMs < pendingMs) return "live";
  if (hasScore) return "finished";
  if (nowMs >= startMs && nowMs < pendingMs) return "live";
  if (nowMs >= pendingMs) return "finished";
  return "scheduled";
}

export function normalizeMatch(match: Match, now = new Date()): Match {
  const status = deriveMatchStatus(match, now);
  return status === match.status ? match : { ...match, status };
}

export function normalizeMatches(items: Match[], now = new Date()) {
  return items.map((match) => normalizeMatch(match, now)).sort(byMatchTime);
}

export function getLiveMatches(items: Match[] = [], now = new Date()) {
  return normalizeMatches(items, now).filter((match) => match.status === "live");
}

export function getNextScheduledMatch(items: Match[] = [], now = new Date()) {
  const nowMs = now.getTime();
  return normalizeMatches(items, now)
    .filter((match) => match.status === "scheduled")
    .find((match) => getMatchStartMs(match) > nowMs);
}

export function getFeaturedMatch(items: Match[] = [], now = new Date()) {
  return getLiveMatches(items, now)[0] ?? getNextScheduledMatch(items, now) ?? normalizeMatches(items, now)[0];
}

export function isMatchVisibleByStatus(match: Match, status: "all" | Match["status"]) {
  return status === "all" || match.status === status;
}

export function matchSearchText(match: Match, homeName: string, awayName: string) {
  return `${homeName} ${awayName} ${match.group} ${match.stage} ${match.venue} ${match.city}`.toLowerCase();
}
