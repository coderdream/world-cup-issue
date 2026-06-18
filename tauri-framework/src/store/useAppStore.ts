import { create } from "zustand";
import { frameworkApi } from "@/services/frameworkApi";
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
  route: "home",
  settings: defaultSettings,
  version: "0.1.0",
  hydrated: false,
  setRoute: (route) => set({ route }),
  hydrate: async () => {
    const state = await frameworkApi.getAppState();
    set({ settings: state.settings, version: state.version, hydrated: true });
  },
  updateSettings: async (settings) => {
    const next = await frameworkApi.setSettings({ ...get().settings, ...settings });
    set({ settings: next });
  }
}));
