import { create } from "zustand";
import { frameworkApi } from "@/services/frameworkApi";
import { defaultSettings } from "@/store/defaults";
import type { AppSettings, BookMaterials, BookMaterialsRequest, MaterialsOutputTab, RouteKey, ScanMaterialFilesResult } from "@/types";

export const defaultBookMaterialsRequest: BookMaterialsRequest = {
  epubPath: "",
  targetMinChars: defaultSettings.materialProfile.targetMinChars,
  targetMaxChars: defaultSettings.materialProfile.targetMaxChars,
  channelName: defaultSettings.materialProfile.channelName,
  language: defaultSettings.materialProfile.language,
  extraDirection: defaultSettings.materialProfile.extraDirection
};

export interface MaterialsWorkbenchState {
  request: BookMaterialsRequest;
  materials: BookMaterials | null;
  scanResult: ScanMaterialFilesResult | null;
  fileStatuses: Record<string, MaterialGenerationStatus>;
  selectedTaskPath: string;
  selectedTaskPaths: string[];
  outputDir: string;
  error: string;
  copyState: string;
  exportState: string;
  activeTab: MaterialsOutputTab;
  currentTraceId: string;
  busy: boolean;
  scanning: boolean;
  exporting: boolean;
}

export interface MaterialGenerationStatus {
  status: "pending" | "generating" | "success" | "failed";
  progress: number;
  narrationChars?: number;
  message?: string;
}

export interface AudioWorkbenchState {
  text: string;
  outputDir: string;
  fileName: string;
  currentTraceId: string;
  busy: boolean;
  error: string;
  status: string;
}

interface AppStore {
  route: RouteKey;
  settings: AppSettings;
  version: string;
  hydrated: boolean;
  materialsWorkbench: MaterialsWorkbenchState;
  audioWorkbench: AudioWorkbenchState;
  setRoute: (route: RouteKey) => void;
  updateMaterialsWorkbench: (state: Partial<MaterialsWorkbenchState>) => void;
  updateAudioWorkbench: (state: Partial<AudioWorkbenchState>) => void;
  updateBookMaterialsRequest: (request: Partial<BookMaterialsRequest>) => void;
  hydrate: () => Promise<void>;
  updateSettings: (settings: Partial<AppSettings>) => Promise<void>;
}

let settingsSaveQueue = Promise.resolve();
let settingsSaveSequence = 0;

function mergeSettings(current: AppSettings, patch: Partial<AppSettings>): AppSettings {
  const materialProfile = {
    ...current.materialProfile,
    ...(patch.materialProfile ?? {})
  };
  const categories = normalizeMaterialCategories(materialProfile.categories, materialProfile.categoryName || materialProfile.channelName);
  return {
    ...current,
    ...patch,
    aiProfile: {
      ...current.aiProfile,
      ...(patch.aiProfile ?? {})
    },
    geminiProfile: {
      ...current.geminiProfile,
      ...(patch.geminiProfile ?? {})
    },
    feishuProfile: {
      ...current.feishuProfile,
      ...(patch.feishuProfile ?? {})
    },
    materialProfile: {
      ...materialProfile,
      categoryName: categories.includes(materialProfile.categoryName) ? materialProfile.categoryName : categories[0],
      channelName: materialProfile.channelName || categories[0],
      categories
    },
    speechProfile: {
      ...current.speechProfile,
      ...(patch.speechProfile ?? {}),
      regionKeys: {
        ...current.speechProfile.regionKeys,
        ...((patch.speechProfile ?? {}).regionKeys ?? {})
      }
    },
    toolProfile: {
      ...current.toolProfile,
      ...(patch.toolProfile ?? {})
    },
    uiProfile: {
      ...current.uiProfile,
      ...(patch.uiProfile ?? {})
    },
    pipelineProfile: {
      ...current.pipelineProfile,
      ...(patch.pipelineProfile ?? {})
    }
  };
}

function normalizeMaterialCategories(categories: string[] | undefined, preferred?: string) {
  const defaults = defaultSettings.materialProfile.categories;
  const values = [...defaults, preferred, ...(categories ?? [])]
    .filter((value): value is string => typeof value === "string")
    .map((value) => value.trim())
    .filter(Boolean);
  return Array.from(new Set(values));
}

export const useAppStore = create<AppStore>((set, get) => ({
  route: "home",
  settings: defaultSettings,
  version: "0.1.0",
  hydrated: false,
  materialsWorkbench: {
    request: defaultBookMaterialsRequest,
    materials: null,
    scanResult: null,
    fileStatuses: {},
    selectedTaskPath: "",
    selectedTaskPaths: [],
    outputDir: "",
    error: "",
    copyState: "",
    exportState: "",
    activeTab: "title",
    currentTraceId: "",
    busy: false,
    scanning: false,
    exporting: false
  },
  audioWorkbench: {
    text: "",
    outputDir: "",
    fileName: "narration",
    currentTraceId: "",
    busy: false,
    error: "",
    status: ""
  },
  setRoute: (route) => set({ route }),
  updateMaterialsWorkbench: (state) =>
    set((current) => ({
      materialsWorkbench: {
        ...current.materialsWorkbench,
        ...state
      }
    })),
  updateAudioWorkbench: (state) =>
    set((current) => ({
      audioWorkbench: {
        ...current.audioWorkbench,
        ...state
      }
    })),
  updateBookMaterialsRequest: (request) =>
    set((current) => ({
      materialsWorkbench: {
        ...current.materialsWorkbench,
        request: {
          ...current.materialsWorkbench.request,
          ...request
        }
      }
    })),
  hydrate: async () => {
    const state = await frameworkApi.getAppState();
    set({ settings: mergeSettings(defaultSettings, state.settings), version: state.version, hydrated: true });
  },
  updateSettings: async (settings) => {
    const next = mergeSettings(get().settings, settings);
    const saveSequence = ++settingsSaveSequence;
    set({ settings: next });
    settingsSaveQueue = settingsSaveQueue
      .catch(() => undefined)
      .then(async () => {
        const saved = await frameworkApi.setSettings(next);
        if (saveSequence === settingsSaveSequence) {
          set({ settings: saved });
        }
      });
    await settingsSaveQueue;
  }
}));
