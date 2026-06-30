import { useCallback, useEffect, useState } from "react";
import { frameworkApi } from "@/services/frameworkApi";
import type { VideoCreatorDashboard } from "@/types";

export function useDashboard() {
  const [dashboard, setDashboard] = useState<VideoCreatorDashboard | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      setDashboard(await frameworkApi.getVideoCreatorDashboard());
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { dashboard, loading, error, refresh, setDashboard };
}

