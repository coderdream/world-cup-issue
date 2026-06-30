import { type ReactElement, useEffect } from "react";
import { AppLayout } from "@/components/AppLayout";
import { AboutPage } from "@/pages/AboutPage";
import { HistoryPage } from "@/pages/HistoryPage";
import { HomePage } from "@/pages/HomePage";
import { LogsPage } from "@/pages/LogsPage";
import { QuarkPage } from "@/pages/QuarkPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { SkillsPage } from "@/pages/SkillsPage";
import { StepsPage } from "@/pages/StepsPage";
import { useAppStore } from "@/store/useAppStore";
import type { RouteKey } from "@/types";

const pageMap: Record<RouteKey, ReactElement> = {
  home: <HomePage />,
  steps: <StepsPage />,
  logs: <LogsPage />,
  history: <HistoryPage />,
  quark: <QuarkPage />,
  skills: <SkillsPage />,
  settings: <SettingsPage />,
  about: <AboutPage />
};

export default function App() {
  const route = useAppStore((state) => state.route);
  const hydrate = useAppStore((state) => state.hydrate);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  return <AppLayout>{pageMap[route]}</AppLayout>;
}
