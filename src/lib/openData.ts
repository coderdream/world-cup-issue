import type { Match } from "@/types";

export const DATA_SOURCES = [
  "https://pub-9d9e6c0cb6934fb0a0c505e3c64f39b2.r2.dev/cupwatch/data/worldcup-2026.json",
  "https://cdn.jsdelivr.net/gh/openfootball/worldcup.json@master/2026/worldcup.json",
  "https://raw.githubusercontent.com/openfootball/worldcup.json/master/2026/worldcup.json"
];

export interface RefreshResult {
  source: string;
  fetchedAt: string;
  rawCount: number;
  importedMatches: Match[];
}

export async function fetchOpenFootball(timeoutMs = 6000): Promise<RefreshResult> {
  let lastError: unknown = null;
  for (const source of DATA_SOURCES) {
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), timeoutMs);
    try {
      const response = await fetch(source, { signal: controller.signal });
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      const json = await response.json();
      return {
        source,
        fetchedAt: new Date().toISOString(),
        rawCount: Array.isArray(json.matches) ? json.matches.length : 0,
        importedMatches: []
      };
    } catch (error) {
      lastError = error;
    } finally {
      window.clearTimeout(timeout);
    }
  }
  throw lastError instanceof Error ? lastError : new Error("全部公开数据源均不可达");
}
