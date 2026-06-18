import { matches, teamById, teams } from "@/data/worldCupData";
import { getBeijingDate, getMatchStartMs, getNextScheduledMatch, normalizeMatches } from "@/domain/matches";
import type { Match, ResultPick, StandingRow, Team } from "@/types";

export function getTeam(id: string): Team {
  const team = teamById.get(id);
  if (team) return team;
  return {
    id,
    code: id.toUpperCase(),
    nameZh: id.toUpperCase(),
    nameEn: id.toUpperCase(),
    group: "",
    flag: "□",
    elo: 1700
  };
}

export function formatScore(match: Match, spoiler = false) {
  if (match.status === "scheduled") return "vs";
  if (spoiler && match.status === "finished") return "··";
  return `${match.score.home ?? 0} - ${match.score.away ?? 0}`;
}

export function formatDateTime(match: Match) {
  return `${match.date} ${match.time}`;
}

export { getMatchStartMs } from "@/domain/matches";

export function resultOf(match: Match): ResultPick | null {
  if (match.status !== "finished" || match.score.home == null || match.score.away == null) return null;
  if (match.score.home > match.score.away) return "home";
  if (match.score.home < match.score.away) return "away";
  return "draw";
}

export function getNextMatch(items: Match[] = matches, now = new Date()) {
  return getNextScheduledMatch(items, now) ?? normalizeMatches(items, now).at(-1) ?? matches[matches.length - 1];
}

export function getTodayMatches(items: Match[] = matches, now = new Date()) {
  const currentDate = getBeijingDate(now);
  const normalized = normalizeMatches(items, now);
  const next = getNextMatch(normalized, now);
  const date = normalized.some((match) => match.date === currentDate) ? currentDate : next.date;
  const sameDate = normalized.filter((match) => match.date === date);
  return sameDate.length ? sameDate.slice(0, 5) : items.slice(12, 17);
}

export function getStandings(group?: string, items: Match[] = matches, teamList: Team[] = teams): Record<string, StandingRow[]> {
  const groups = teamList.reduce<Record<string, StandingRow[]>>((acc, team) => {
    acc[team.group] ??= [];
    acc[team.group].push({
      teamId: team.id,
      rank: 0,
      played: 0,
      wins: 0,
      draws: 0,
      losses: 0,
      goalDiff: 0,
      points: 0
    });
    return acc;
  }, {});

  for (const match of items.filter((item) => item.status === "finished")) {
    const rows = groups[match.group];
    if (!rows || match.score.home == null || match.score.away == null) continue;
    const home = rows.find((row) => row.teamId === match.homeTeamId);
    const away = rows.find((row) => row.teamId === match.awayTeamId);
    if (!home || !away) continue;
    home.played += 1;
    away.played += 1;
    home.goalDiff += match.score.home - match.score.away;
    away.goalDiff += match.score.away - match.score.home;
    if (match.score.home > match.score.away) {
      home.wins += 1;
      away.losses += 1;
      home.points += 3;
    } else if (match.score.home < match.score.away) {
      away.wins += 1;
      home.losses += 1;
      away.points += 3;
    } else {
      home.draws += 1;
      away.draws += 1;
      home.points += 1;
      away.points += 1;
    }
  }

  for (const rows of Object.values(groups)) {
    rows.sort((a, b) => b.points - a.points || b.goalDiff - a.goalDiff || getTeam(a.teamId).nameZh.localeCompare(getTeam(b.teamId).nameZh));
    rows.forEach((row, index) => {
      row.rank = index + 1;
    });
  }

  if (group) return { [group]: groups[group] ?? [] };
  return groups;
}

export function eloOdds(homeId: string, awayId: string) {
  const home = getTeam(homeId);
  const away = getTeam(awayId);
  const expectedHome = 1 / (1 + 10 ** ((away.elo - home.elo) / 400));
  const draw = 0.11;
  const homeWin = Math.max(0.08, expectedHome * (1 - draw));
  const awayWin = Math.max(0.08, (1 - expectedHome) * (1 - draw));
  const total = homeWin + draw + awayWin;
  return {
    home: Math.round((homeWin / total) * 100),
    draw: Math.round((draw / total) * 100),
    away: Math.round((awayWin / total) * 100)
  };
}
