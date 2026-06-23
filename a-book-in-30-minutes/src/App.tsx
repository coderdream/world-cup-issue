import { type ReactElement, useEffect } from "react";
import { AppLayout } from "@/components/AppLayout";
import { AboutPage } from "@/pages/AboutPage";
import { AudioPage } from "@/pages/AudioPage";
import { HomePage } from "@/pages/HomePage";
import { LogsPage } from "@/pages/LogsPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { useAppStore } from "@/store/useAppStore";
import type { RouteKey } from "@/types";

const pageMap: Record<RouteKey, ReactElement> = {
  home: <HomePage />,
  audio: <AudioPage />,
  logs: <LogsPage />,
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
