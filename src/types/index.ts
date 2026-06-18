export type RouteKey =
  | "overview"
  | "schedule"
  | "scores"
  | "standings"
  | "bracket"
  | "ai"
  | "predictions"
  | "teams"
  | "settings"
  | "about";

export type MatchStatus = "finished" | "live" | "scheduled";
export type ResultPick = "home" | "draw" | "away";

export interface Team {
  id: string;
  code: string;
  nameZh: string;
  nameEn: string;
  group: string;
  flag: string;
  elo: number;
}

export interface Score {
  home: number | null;
  away: number | null;
}

export interface Match {
  id: string;
  group: string;
  stage: string;
  date: string;
  time: string;
  utcOffset: string;
  homeTeamId: string;
  awayTeamId: string;
  score: Score;
  status: MatchStatus;
  venue: string;
  city: string;
  updatedAt?: string;
}

export interface StandingRow {
  teamId: string;
  rank: number;
  played: number;
  wins: number;
  draws: number;
  losses: number;
  goalDiff: number;
  points: number;
}

export interface BracketNode {
  id: string;
  round: "32" | "16" | "quarter" | "semi" | "third" | "final";
  slotA: string;
  slotB: string;
  venue?: string;
  winner?: string;
}

export interface Prediction {
  matchId: string;
  pick: ResultPick;
  lockedAt: string;
  resolved: boolean;
  correct?: boolean;
}

export interface AppSettings {
  spoilerMode: boolean;
  scorebarEnabled: boolean;
  launchOnBoot: boolean;
  notificationsEnabled: boolean;
  reminderMinutes: number;
  footballDataToken: string;
  aiProvider: string;
  aiApiKey: string;
  aiBaseUrl: string;
  aiModel: string;
  aiProfileName: string;
}

export interface LicenseState {
  status: "trial" | "active" | "expired";
  remainingDays: number;
  expiresAt: string;
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  available: boolean;
  notes: string;
}

export interface AppStatePayload {
  teams: Team[];
  matches: Match[];
  settings: AppSettings;
  predictions: Prediction[];
  license: LicenseState;
  lastUpdated: string | null;
}

export interface RefreshMatchesResult {
  matches: Match[];
  lastUpdated: string | null;
}

export interface AiModelConfig {
  provider: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  name: string;
}

export interface AiProfileShare {
  data: {
    apiKey: string;
    baseURL: string;
    model: string;
    name: string;
    provider: "openai_compatible";
  };
  kind: "ai.profile";
  v: 1;
}

export interface ConnectivityTestResult {
  ok: boolean;
  message: string;
  details?: string;
}

export interface AiEvaluationContext {
  matchId: string;
  homeTeam: string;
  awayTeam: string;
  kickoff: string;
  venue: string;
  status: MatchStatus;
  score: string;
  oddsHome: number;
  oddsDraw: number;
  oddsAway: number;
}

export interface AiEvaluationRequest {
  config: AiModelConfig;
  context: AiEvaluationContext;
}

export interface AiGenerationResult {
  ok: boolean;
  content: string;
  message?: string;
}
