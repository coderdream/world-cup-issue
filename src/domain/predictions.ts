import type { Match, Prediction, ResultPick } from "@/types";
import { resultOf } from "@/utils/standings";

export function createPrediction(match: Match, pick: ResultPick): Prediction {
  return {
    matchId: match.id,
    pick,
    lockedAt: new Date().toISOString(),
    resolved: false
  };
}

export function resolvePrediction(match: Match, prediction: Prediction): Prediction {
  const result = resultOf(match);
  if (!result) return prediction;
  return {
    ...prediction,
    resolved: true,
    correct: result === prediction.pick
  };
}

export function pickLabel(pick: ResultPick) {
  if (pick === "home") return "主队胜";
  if (pick === "away") return "客队胜";
  return "平局";
}
