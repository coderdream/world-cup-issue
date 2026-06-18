import { type ReactElement, useEffect } from "react";
import { AppLayout } from "@/components/AppLayout";
import { getLiveMatches } from "@/domain/matches";
import {
  AboutPage,
  AiAnalysisPage,
  BracketPage,
  FloatingScorebar,
  OverviewPage,
  PredictionsPage,
  SchedulePage,
  ScoresPage,
  SettingsPage,
  StandingsPage,
  TeamsPage
} from "@/pages/CupWatchPages";
import { useCupStore } from "@/store/useCupStore";
import type { RouteKey } from "@/types";

const pageMap: Record<RouteKey, ReactElement> = {
  overview: <OverviewPage />,
  schedule: <SchedulePage />,
  scores: <ScoresPage />,
  standings: <StandingsPage />,
  bracket: <BracketPage />,
  ai: <AiAnalysisPage />,
  predictions: <PredictionsPage />,
  teams: <TeamsPage />,
  settings: <SettingsPage />,
  about: <AboutPage />
};

export default function App() {
  const route = useCupStore((state) => state.route);
  const hydrate = useCupStore((state) => state.hydrate);
  const refreshMatches = useCupStore((state) => state.refreshMatches);
  const setRoute = useCupStore((state) => state.setRoute);
  const isScorebar = new URLSearchParams(window.location.search).get("scorebar") === "1";

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void refreshMatches();
    }, 30_000);
    return () => window.clearInterval(timer);
  }, [refreshMatches]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let canceled = false;

    async function bindHotkeyRoute() {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen("worldcupissue://hotkey-open", () => {
          const matches = useCupStore.getState().matches;
          setRoute(getLiveMatches(matches).length > 0 ? "scores" : "schedule");
        });
        if (canceled) unlisten();
      } catch {
        // Browser preview has no native event bus.
      }
    }

    void bindHotkeyRoute();
    return () => {
      canceled = true;
      unlisten?.();
    };
  }, [setRoute]);

  if (isScorebar) return <FloatingScorebar />;

  return <AppLayout>{pageMap[route]}</AppLayout>;
}
