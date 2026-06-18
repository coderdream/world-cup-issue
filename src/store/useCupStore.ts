import { create } from "zustand";
import { defaultLicense, defaultSettings, matches as fallbackMatches, teams as fallbackTeams } from "@/data/worldCupData";
import { normalizeMatches } from "@/domain/matches";
import { cupwatchApi } from "@/lib/api/cupwatch";
import type {
  AppSettings,
  AppStatePayload,
  LicenseState,
  Match,
  Prediction,
  RefreshMatchesResult,
  RouteKey,
  Team
} from "@/types";

interface CupStore {
  route: RouteKey;
  teams: Team[];
  matches: Match[];
  favorites: string[];
  predictions: Prediction[];
  settings: AppSettings | null;
  license: LicenseState | null;
  lastUpdated: string | null;
  setRoute: (route: RouteKey) => void;
  hydrate: () => Promise<void>;
  updateSettings: (settings: Partial<AppSettings>) => Promise<void>;
  toggleSpoiler: () => Promise<void>;
  toggleFavorite: (teamId: string) => Promise<void>;
  savePrediction: (prediction: Prediction) => Promise<void>;
  refreshMatches: () => Promise<void>;
}

export const useCupStore = create<CupStore>((set, get) => ({
  route: "overview",
  teams: fallbackTeams,
  matches: fallbackMatches,
  favorites: [],
  predictions: [],
  settings: null,
  license: null,
  lastUpdated: null,
  setRoute: (route) => set({ route }),
  hydrate: async () => {
    const payload = await cupwatchApi.getAppState();
    const persisted = JSON.parse(localStorage.getItem("worldcupissue-state") || "{}") as { favorites?: string[] };
    const now = new Date();
    const hydratedMatches = payload.matches.length >= 16 ? payload.matches : fallbackMatches;
    set({
      teams: payload.teams.length >= 48 ? payload.teams : fallbackTeams,
      matches: normalizeMatches(hydratedMatches, now),
      settings: payload.settings ?? defaultSettings,
      predictions: payload.predictions,
      license: payload.license ?? defaultLicense,
      lastUpdated: payload.lastUpdated ?? null,
      favorites: persisted.favorites ?? []
    });
    void get().refreshMatches();
  },
  refreshMatches: async () => {
    try {
      const result = await cupwatchApi.refreshMatches();
      const nextState: Partial<CupStore> = {
        matches: normalizeMatches(result.matches)
      };
      if (result.lastUpdated) nextState.lastUpdated = result.lastUpdated;
      set(nextState);
    } catch {
      // Keep cached or bundled data visible when the public source is unavailable.
    }
  },
  updateSettings: async (patch) => {
    const current = get().settings;
    if (!current) return;
    const settings = await cupwatchApi.setSettings({ ...current, ...patch });
    set({ settings });
  },
  toggleSpoiler: async () => {
    const settings = await cupwatchApi.toggleSpoilerMode();
    set({ settings });
  },
  toggleFavorite: async (teamId) => {
    const favorites = await cupwatchApi.toggleFavoriteTeam(teamId);
    set({ favorites });
  },
  savePrediction: async (prediction) => {
    const predictions = await cupwatchApi.savePrediction(prediction);
    set({ predictions });
  }
}));
