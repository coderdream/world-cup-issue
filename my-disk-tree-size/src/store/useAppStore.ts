import { create } from "zustand";
import { diskApi } from "@/services/diskApi";
import { defaultSettings } from "@/store/defaults";
import type { AppSettings, RouteKey } from "@/types";

interface AppStore {
  route: RouteKey;
  settings: AppSettings;
  version: string;
  hydrated: boolean;
  setRoute: (route: RouteKey) => void;
  hydrate: () => Promise<void>;
  updateSettings: (settings: Partial<AppSettings>) => Promise<void>;
}

export const useAppStore = create<AppStore>((set, get) => ({
  route: "scan",
  settings: defaultSettings,
  version: "0.1.20",
  hydrated: false,
  setRoute: (route) => set({ route }),
  hydrate: async () => {
    const state = await diskApi.getAppState();
    set({ settings: mergeSettings(state.settings), version: state.version, hydrated: true });
  },
  updateSettings: async (settings) => {
    const next = await diskApi.setSettings(mergeSettings({ ...get().settings, ...settings }));
    set({ settings: mergeSettings(next) });
  }
}));

function mergeSettings(settings: AppSettings): AppSettings {
  const excludedDirNames = [...settings.excludedDirNames];
  for (const item of defaultSettings.excludedDirNames) {
    if (!excludedDirNames.some((value) => value.toLowerCase() === item.toLowerCase())) {
      excludedDirNames.push(item);
    }
  }
  return { ...defaultSettings, ...settings, excludedDirNames };
}



